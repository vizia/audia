use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::runtime::Runtime;
use vizia::prelude::{ContextProxy, ImageRetentionPolicy};

use crate::oauth as oauth_api;
use crate::playback::PlaybackService;
use crate::spotify::SpotifyService;
use crate::storage::TokenStore;
use crate::ui::events::{OAuthAppEvent, PlaybackAppEvent};

mod albums;
mod artists;
mod auth;
mod oauth;
mod playback;
mod playlists;
mod search;

pub use albums::{fetch_album_from_track, fetch_album_tracks};
pub use artists::{fetch_artist_view, fetch_artist_view_by_name};
pub use auth::init_backend;
pub use oauth::{refresh_access_token, reset_login, start_oauth_login};
pub use playback::{
    load_playback_artwork, playback_next, playback_pause, playback_pause_local,
    playback_play_local_track, playback_play_selected_track, playback_previous,
    playback_resume_local, playback_resume_remote, playback_seek, playback_seek_local,
    playback_set_volume, playback_stop, playback_transfer_device, refresh_playback_devices,
    start_playback_progress_poller,
};
pub use playlists::{fetch_playlist_tracks, refresh_user_playlists};
pub use search::search_tracks;

const IMAGE_FETCH_CONCURRENCY: usize = 8;

async fn fetch_image_bytes(url: String) -> Option<Vec<u8>> {
    let response = reqwest::get(url).await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    response.bytes().await.ok().map(|b| b.to_vec())
}

async fn load_images_parallel(
    proxy: &mut ContextProxy,
    jobs: Vec<(usize, String, String)>,
) -> Vec<(usize, String)> {
    let mut loaded = Vec::new();
    let mut set = tokio::task::JoinSet::new();

    for (index, key, url) in jobs {
        set.spawn(async move { (index, key, fetch_image_bytes(url).await) });

        if set.len() >= IMAGE_FETCH_CONCURRENCY
            && let Some(Ok((idx, key, Some(image_bytes)))) = set.join_next().await
        {
            let _ = proxy.load_image(key.clone(), &image_bytes, ImageRetentionPolicy::Forever);
            loaded.push((idx, key));
        }
    }

    while let Some(job_result) = set.join_next().await {
        if let Ok((idx, key, Some(image_bytes))) = job_result {
            let _ = proxy.load_image(key.clone(), &image_bytes, ImageRetentionPolicy::Forever);
            loaded.push((idx, key));
        }
    }

    loaded
}

pub struct BackendState {
    pub runtime: Arc<Runtime>,
    pub spotify: SpotifyService,
    pub playback: PlaybackService,
    pub oauth_in_progress: bool,
    pub refresh_token: Option<String>,
    pub client_id: Option<String>,
    pub token_expires_at: Option<u64>,
}

impl Default for BackendState {
    fn default() -> Self {
        let runtime = Runtime::new().expect("failed to build shared tokio runtime");
        Self {
            runtime: Arc::new(runtime),
            spotify: SpotifyService::default(),
            playback: PlaybackService::default(),
            oauth_in_progress: false,
            refresh_token: None,
            client_id: None,
            token_expires_at: None,
        }
    }
}

impl BackendState {
    pub fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub fn token_needs_refresh(&self) -> bool {
        if self.refresh_token.is_none() {
            return false;
        }

        match self.token_expires_at {
            Some(expires_at) => Self::now_secs() + 60 >= expires_at,
            None => true,
        }
    }
}

pub type SharedBackend = Arc<Mutex<BackendState>>;

fn lock_backend(backend: &SharedBackend) -> MutexGuard<'_, BackendState> {
    backend
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn is_auth_error(err: &str) -> bool {
    let lowered = err.to_ascii_lowercase();
    lowered.contains("status 401")
        || lowered.contains("401 unauthorized")
        || lowered.contains("invalid access token")
        || lowered.contains("the access token expired")
}

async fn force_refresh_access_token_async(backend: &SharedBackend) -> Result<(), String> {
    let (cid, rt) = {
        let state = lock_backend(backend);
        (state.client_id.clone(), state.refresh_token.clone())
    };

    let (cid, rt) = match (cid, rt) {
        (Some(cid), Some(rt)) => (cid, rt),
        _ => {
            return Err(
                "Token refresh required but client ID or refresh token is missing.".to_string(),
            );
        }
    };

    let tokens = oauth_api::refresh_access_token(&cid, &rt)
        .await
        .map_err(|err| format!("Auto-refresh failed: {err}"))?;

    let expires_at = BackendState::now_secs() + tokens.expires_in;
    let new_refresh = tokens.refresh_token.clone();

    {
        let mut state = lock_backend(backend);
        state.spotify.set_access_token(tokens.access_token.clone());
        if let Some(rt) = new_refresh {
            state.refresh_token = Some(rt);
        }
        state.token_expires_at = Some(expires_at);

        let runtime = Arc::clone(&state.runtime);
        let _ = state
            .playback
            .bootstrap_from_access_token(runtime.as_ref(), &tokens.access_token);
    }

    let refresh_token = lock_backend(backend).refresh_token.clone();
    let _ = TokenStore {
        access_token: tokens.access_token,
        refresh_token,
        expires_at: Some(expires_at),
    }
    .save();

    Ok(())
}

async fn with_spotify_auth_retry<T, F, Fut>(backend: &SharedBackend, mut op: F) -> Result<T, String>
where
    F: FnMut(SpotifyService) -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    ensure_fresh_access_token_async(backend).await?;

    let spotify = { lock_backend(backend).spotify.clone() };
    match op(spotify).await {
        Ok(value) => Ok(value),
        Err(err) if is_auth_error(&err) => {
            force_refresh_access_token_async(backend).await?;
            let spotify = { lock_backend(backend).spotify.clone() };
            op(spotify).await
        }
        Err(err) => Err(err),
    }
}

async fn ensure_fresh_access_token_async(backend: &SharedBackend) -> Result<(), String> {
    let needs_refresh = { lock_backend(backend).token_needs_refresh() };

    if !needs_refresh {
        return Ok(());
    }

    force_refresh_access_token_async(backend).await
}

fn apply_token_response(
    backend: &SharedBackend,
    tokens: &oauth_api::TokenResponse,
    client_id: &str,
    proxy: &mut ContextProxy,
    runtime: &tokio::runtime::Runtime,
) {
    let expires_at = BackendState::now_secs() + tokens.expires_in;
    let new_refresh = tokens.refresh_token.clone();

    {
        let mut state = backend.lock().unwrap();
        state.spotify.set_access_token(tokens.access_token.clone());
        if let Some(rt) = new_refresh {
            state.refresh_token = Some(rt);
        }
        state.token_expires_at = Some(expires_at);
        state.client_id = Some(client_id.to_string());

        if state
            .playback
            .bootstrap_from_access_token(runtime, &tokens.access_token)
            .is_ok()
        {
            let _ = proxy.emit(PlaybackAppEvent::SessionReady);
        }
    }

    let refresh_token = backend.lock().unwrap().refresh_token.clone();
    let _ = TokenStore {
        access_token: tokens.access_token.clone(),
        refresh_token,
        expires_at: Some(expires_at),
    }
    .save();

    emit_login_profile_event(backend, runtime, proxy);
}

fn emit_login_profile_event(
    backend: &SharedBackend,
    runtime: &tokio::runtime::Runtime,
    proxy: &mut ContextProxy,
) {
    let profile = runtime
        .block_on(backend.lock().unwrap().spotify.fetch_profile())
        .ok();

    let username = profile
        .as_ref()
        .and_then(|profile| profile.display_name.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let profile_image_key =
        if let Some(image_bytes) = profile.and_then(|profile| profile.image_bytes) {
            let key = format!("spotify-profile-avatar-{}", BackendState::now_secs());
            let _ = proxy.load_image(key.clone(), &image_bytes, ImageRetentionPolicy::Forever);
            Some(key)
        } else {
            None
        };

    let _ = proxy.emit(OAuthAppEvent::LoginComplete {
        username,
        profile_image_key,
    });
}
