use std::future::Future;
use std::sync::OnceLock;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use vizia::prelude::{ContextProxy, ImageRetentionPolicy};

use crate::oauth as oauth_api;
use crate::playback::PlaybackService;
use crate::spotify::{SpotifyProfile, SpotifyService};
use crate::storage::TokenStore;
use crate::ui::events::{OAuthEvent, PlaybackEvent};

mod albums;
mod artists;
mod auth;
mod oauth;
mod playback;
mod playlists;
mod search;

pub use albums::{fetch_album_from_track, fetch_album_tracks, hydrate_album_artwork};
pub use artists::{fetch_artist_view, fetch_artist_view_from_track, hydrate_artist_artwork};
pub use auth::init_backend;
pub use oauth::{refresh_access_token, reset_login, start_oauth_login};
pub use playback::{
    load_playback_artwork, playback_pause_local, playback_play_local_track, playback_resume_local,
    playback_seek_local, playback_stop_local, start_playback_progress_poller,
};
pub use playlists::{
    add_track_to_playlist, create_playlist, delete_playlist, fetch_playlist_tracks,
    hydrate_user_playlist_artwork, refresh_user_playlists, remove_track_from_playlist,
    rename_playlist,
};
pub use search::{hydrate_search_artwork, search_tracks};

const IMAGE_FETCH_CONCURRENCY: usize = 8;
static IMAGE_HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn image_http_client() -> &'static reqwest::Client {
    IMAGE_HTTP_CLIENT.get_or_init(reqwest::Client::new)
}

async fn fetch_image_bytes(url: String) -> Option<Vec<u8>> {
    let response = image_http_client().get(url).send().await.ok()?;
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
    pub spotify: SpotifyService,
    pub playback: SharedPlayback,
    pub oauth_in_progress: bool,
    pub refresh_token: Option<String>,
    pub client_id: Option<String>,
    pub token_expires_at: Option<u64>,
}

impl Default for BackendState {
    fn default() -> Self {
        Self {
            spotify: SpotifyService::default(),
            playback: Arc::new(Mutex::new(PlaybackService::default())),
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
pub type SharedPlayback = Arc<Mutex<PlaybackService>>;

pub fn lock_backend(backend: &SharedBackend) -> Result<MutexGuard<'_, BackendState>, String> {
    backend
        .lock()
        .map_err(|_| "backend state is unavailable after a prior panic".to_string())
}

pub fn lock_playback(playback: &SharedPlayback) -> Result<MutexGuard<'_, PlaybackService>, String> {
    playback
        .lock()
        .map_err(|_| "playback state is unavailable after a prior panic".to_string())
}

pub fn shared_playback(backend: &SharedBackend) -> Result<SharedPlayback, String> {
    Ok(lock_backend(backend)?.playback.clone())
}

pub fn set_oauth_in_progress(backend: &SharedBackend, in_progress: bool) -> Result<(), String> {
    lock_backend(backend)?.oauth_in_progress = in_progress;
    Ok(())
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
        let state = lock_backend(backend)?;
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
        let mut state = lock_backend(backend)?;
        state.spotify.set_access_token(tokens.access_token.clone());
        if let Some(rt) = new_refresh {
            state.refresh_token = Some(rt);
        }
        state.token_expires_at = Some(expires_at);
    }

    let _ = bootstrap_playback_from_token(backend, &tokens.access_token).await;

    let refresh_token = lock_backend(backend)?.refresh_token.clone();
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

    let spotify = { lock_backend(backend)?.spotify.clone() };
    match op(spotify).await {
        Ok(value) => Ok(value),
        Err(err) if is_auth_error(&err) => {
            force_refresh_access_token_async(backend).await?;
            let spotify = { lock_backend(backend)?.spotify.clone() };
            op(spotify).await
        }
        Err(err) => Err(err),
    }
}

async fn ensure_fresh_access_token_async(backend: &SharedBackend) -> Result<(), String> {
    let needs_refresh = { lock_backend(backend)?.token_needs_refresh() };

    if !needs_refresh {
        return Ok(());
    }

    force_refresh_access_token_async(backend).await
}

async fn bootstrap_playback_from_token(
    backend: &SharedBackend,
    access_token: &str,
) -> Result<(), String> {
    let shared_playback = shared_playback(backend)?;
    let mut playback = {
        let mut state = lock_playback(&shared_playback)?;
        std::mem::take(&mut *state)
    };

    let result = playback.bootstrap_from_access_token(access_token).await;

    let mut state = lock_playback(&shared_playback)?;
    *state = playback;
    result
}

async fn apply_token_response(
    backend: &SharedBackend,
    tokens: &oauth_api::TokenResponse,
    client_id: &str,
    proxy: &mut ContextProxy,
) -> Result<(), String> {
    let expires_at = BackendState::now_secs() + tokens.expires_in;
    let new_refresh = tokens.refresh_token.clone();

    {
        let mut state = lock_backend(backend)?;
        state.spotify.set_access_token(tokens.access_token.clone());
        if let Some(rt) = new_refresh {
            state.refresh_token = Some(rt);
        }
        state.token_expires_at = Some(expires_at);
        state.client_id = Some(client_id.to_string());
    }

    if bootstrap_playback_from_token(backend, &tokens.access_token)
        .await
        .is_ok()
    {
        let _ = proxy.emit(PlaybackEvent::SessionReady);
    }

    let refresh_token = lock_backend(backend)?.refresh_token.clone();
    let _ = TokenStore {
        access_token: tokens.access_token.clone(),
        refresh_token,
        expires_at: Some(expires_at),
    }
    .save();

    let spotify = { lock_backend(backend)?.spotify.clone() };
    let profile = spotify.fetch_profile().await.ok();
    emit_login_profile_event(profile, proxy);

    Ok(())
}

fn emit_login_profile_event(profile: Option<SpotifyProfile>, proxy: &mut ContextProxy) {
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

    let _ = proxy.emit(OAuthEvent::LoginComplete {
        username,
        profile_image_key,
    });
}
