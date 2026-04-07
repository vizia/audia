use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{thread, time::Duration};

use tokio::runtime::Runtime;
use vizia::prelude::{ContextProxy, ImageRetentionPolicy};

use crate::messages::{PlaylistEntry, Track};
use crate::oauth;
use crate::playback::PlaybackService;
use crate::spotify::SpotifyService;
use crate::storage::{ClientCredentialStore, TokenStore, clear_persisted_login};
use crate::ui::events::{
    OAuthAppEvent, PlaybackAppEvent, PlaybackProgressSource, PlaylistsAppEvent, SearchAppEvent,
    SystemAppEvent,
};

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
    // Returns the current time in seconds since the UNIX epoch.
    pub fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    // Determines if the current access token needs to be refreshed based on its expiration time.
    pub fn token_needs_refresh(&self) -> bool {
        if self.refresh_token.is_none() {
            return false;
        }

        match self.token_expires_at {
            Some(expires_at) => Self::now_secs() + 60 >= expires_at,
            None => false,
        }
    }
}

pub type SharedBackend = Arc<Mutex<BackendState>>;

// Initializes the backend state and starts the initial authentication check.
pub fn init_backend(proxy: ContextProxy) -> SharedBackend {
    let backend: SharedBackend = Arc::new(Mutex::new(BackendState::default()));
    let backend_clone = Arc::clone(&backend);

    proxy.spawn(move |proxy| {
        let runtime = {
            let state = backend_clone.lock().unwrap();
            state.runtime.clone()
        };

        if let Ok(Some(token)) = TokenStore::load() {
            {
                let mut state = backend_clone.lock().unwrap();
                state.spotify.set_access_token(token.access_token.clone());
                state.refresh_token = token.refresh_token.clone();
                state.token_expires_at = token.expires_at;
            }

            if let Ok(Some(creds)) = ClientCredentialStore::load() {
                let mut state = backend_clone.lock().unwrap();
                state.client_id = Some(creds.client_id);
            }

            let (needs_refresh, cid, rt) = {
                let state = backend_clone.lock().unwrap();
                (
                    state.token_needs_refresh(),
                    state.client_id.clone(),
                    state.refresh_token.clone(),
                )
            };

            if needs_refresh {
                if let (Some(cid), Some(rt)) = (cid, rt) {
                    match runtime.block_on(oauth::refresh_access_token(&cid, &rt)) {
                        Ok(tokens) => {
                            apply_token_response(
                                &backend_clone,
                                &tokens,
                                &cid,
                                proxy,
                                runtime.as_ref(),
                            );
                            let _ = proxy.emit(SystemAppEvent::Ready);
                            return;
                        }
                        Err(err) => {
                            let _ = proxy.emit(SystemAppEvent::Error(format!(
                                "Silent token refresh failed: {err}"
                            )));
                        }
                    }
                }
            } else {
                let valid = {
                    let state = backend_clone.lock().unwrap();
                    runtime
                        .block_on(state.spotify.validate_token())
                        .unwrap_or(false)
                };

                if valid {
                    emit_login_profile_event(&backend_clone, runtime.as_ref(), proxy);

                    let mut state = backend_clone.lock().unwrap();
                    if state
                        .playback
                        .bootstrap_from_access_token(runtime.as_ref(), &token.access_token)
                        .is_ok()
                    {
                        let _ = proxy.emit(PlaybackAppEvent::SessionReady);
                    }
                } else {
                    let _ = proxy.emit(SystemAppEvent::StatusMessage(
                        "Saved token is invalid. Please log in again.".to_string(),
                    ));
                }
            }
        }

        let _ = proxy.emit(SystemAppEvent::Ready);
    });

    backend
}

// Starts a background thread that periodically polls the Spotify API for playback progress updates and emits them to the UI.
pub fn start_playback_progress_poller(backend: SharedBackend, proxy: ContextProxy) {
    proxy.spawn(move |proxy| {
        let poll_interval = Duration::from_millis(500);

        loop {
            let runtime = {
                let state = backend.lock().unwrap();
                state.runtime.clone()
            };

            let local_track_ended = {
                let state = backend.lock().unwrap();
                state.playback.consume_track_finished()
            };

            if local_track_ended {
                let _ = proxy.emit(PlaybackAppEvent::LocalTrackEnded);
            }

            let local_progress = {
                let state = backend.lock().unwrap();
                state.playback.playback_progress()
            };

            if let Some((position_ms, duration_ms, is_playing)) = local_progress {
                let _ = proxy.emit(PlaybackAppEvent::Progress {
                    source: PlaybackProgressSource::Local,
                    position_ms,
                    duration_ms,
                    is_playing,
                });
            }

            let remote_progress = {
                let state = backend.lock().unwrap();
                runtime.block_on(state.spotify.playback_progress())
            };

            if let Ok(Some((position_ms, duration_ms, is_playing))) = remote_progress {
                let _ = proxy.emit(PlaybackAppEvent::Progress {
                    source: PlaybackProgressSource::Remote,
                    position_ms,
                    duration_ms,
                    is_playing,
                });
            }

            thread::sleep(poll_interval);
        }
    });
}

// Loads the specified image URL, stores it in the resource manager, and emits an event with the resulting image key for UI display.
pub fn load_playback_artwork(
    backend: SharedBackend,
    image_url: Option<String>,
    mut proxy: ContextProxy,
) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        let Some(url) = image_url else {
            let _ = proxy.emit(PlaybackAppEvent::ArtworkLoaded { image_key: None });
            return;
        };

        let bytes = fetch_image_bytes(url.clone()).await;

        let image_key = if let Some(image_bytes) = bytes {
            let key = format!("playback-artwork:{}", url);
            let _ = proxy.load_image(key.clone(), &image_bytes, ImageRetentionPolicy::Forever);
            Some(key)
        } else {
            None
        };

        let _ = proxy.emit(PlaybackAppEvent::ArtworkLoaded { image_key });
    });
}

// Handles the OAuth login flow by opening the user's browser for authentication,
// waiting for the callback, exchanging the authorization code for tokens,
// and updating the backend state accordingly.
pub fn start_oauth_login(backend: SharedBackend, client_id: String, proxy: ContextProxy) {
    proxy.spawn(move |proxy| {
        let runtime = {
            let state = backend.lock().unwrap();
            state.runtime.clone()
        };

        {
            let mut state = backend.lock().unwrap();
            if state.oauth_in_progress {
                let _ = proxy.emit(SystemAppEvent::StatusMessage(
                    "OAuth login is already in progress. Complete it in your browser.".to_string(),
                ));
                return;
            }
            state.oauth_in_progress = true;
        }

        if let Err(err) = (ClientCredentialStore {
            client_id: client_id.clone(),
        })
        .save()
        {
            backend.lock().unwrap().oauth_in_progress = false;
            let _ = proxy.emit(SystemAppEvent::Error(format!(
                "Failed to save client credentials: {err}"
            )));
            return;
        }

        {
            let mut state = backend.lock().unwrap();
            state.client_id = Some(client_id.clone());
        }

        let state_token = oauth::generate_state();
        let code_verifier = oauth::generate_code_verifier();
        let challenge = oauth::code_challenge(&code_verifier);
        let url = oauth::auth_url(&client_id, &state_token, &challenge);

        if let Err(err) = webbrowser::open(&url) {
            backend.lock().unwrap().oauth_in_progress = false;
            let _ = proxy.emit(SystemAppEvent::Error(format!(
                "Failed to open browser: {err}"
            )));
            return;
        }

        let _ = proxy.emit(OAuthAppEvent::BrowserOpened);
        let _ = proxy.emit(SystemAppEvent::StatusMessage(
            "Waiting for OAuth callback from browser...".to_string(),
        ));

        let state_token_clone = state_token.clone();
        let code_result = std::thread::spawn(move || oauth::wait_for_callback(&state_token_clone))
            .join()
            .map_err(|_| "OAuth callback thread panicked".to_string())
            .and_then(|result| result);

        let code = match code_result {
            Ok(code) => code,
            Err(err) => {
                backend.lock().unwrap().oauth_in_progress = false;
                let _ = proxy.emit(SystemAppEvent::Error(format!(
                    "OAuth callback error: {err}"
                )));
                return;
            }
        };

        let _ = proxy.emit(SystemAppEvent::StatusMessage(
            "OAuth callback received. Exchanging code for tokens...".to_string(),
        ));

        match runtime.block_on(oauth::exchange_code(&client_id, &code, &code_verifier)) {
            Ok(tokens) => {
                apply_token_response(&backend, &tokens, &client_id, proxy, &runtime);
                backend.lock().unwrap().oauth_in_progress = false;
            }
            Err(err) => {
                backend.lock().unwrap().oauth_in_progress = false;
                let _ = proxy.emit(SystemAppEvent::Error(format!(
                    "Token exchange failed: {err}"
                )));
            }
        }
    });
}

//
pub fn refresh_access_token(backend: SharedBackend, proxy: ContextProxy) {
    proxy.spawn(move |proxy| {
        let runtime = {
            let state = backend.lock().unwrap();
            state.runtime.clone()
        };

        let (cid, rt) = {
            let state = backend.lock().unwrap();
            (state.client_id.clone(), state.refresh_token.clone())
        };

        match (cid, rt) {
            (Some(cid), Some(rt)) => match runtime.block_on(oauth::refresh_access_token(&cid, &rt))
            {
                Ok(tokens) => {
                    apply_token_response(&backend, &tokens, &cid, proxy, runtime.as_ref())
                }
                Err(err) => {
                    let _ = proxy.emit(SystemAppEvent::Error(format!(
                        "Token refresh failed: {err}"
                    )));
                }
            },
            _ => {
                let _ = proxy.emit(SystemAppEvent::Error(
                    "Cannot refresh: no client ID or refresh token stored.".to_string(),
                ));
            }
        }
    });
}

// Resets the login state by clearing persisted login data, Spotify access tokens, and playback state.
pub fn reset_login(backend: SharedBackend, proxy: ContextProxy) {
    proxy.spawn(move |proxy| {
        if let Err(err) = clear_persisted_login() {
            let _ = proxy.emit(SystemAppEvent::Error(format!(
                "Failed to clear persisted login: {err}"
            )));
            return;
        }

        {
            let mut state = backend.lock().unwrap();
            state.spotify.clear_access_token();
            state.playback.reset();
            state.refresh_token = None;
            state.client_id = None;
            state.token_expires_at = None;
        }

        let _ = proxy.emit(OAuthAppEvent::LoggedOut);
    });
}

// Searches for tracks, artists, and albums in the Spotify catalog based on the provided query.
pub fn search_tracks(backend: SharedBackend, query: String, mut proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {

        if let Err(err) = ensure_fresh_access_token_async(&backend).await {
            let _ = proxy.emit(SystemAppEvent::Error(err));
            return;
        }

        let spotify = {
            let state = backend.lock().unwrap();
            state.spotify.clone()
        };

        let result = spotify.search_catalog(&query).await;

        match result {
            Ok(mut results) => {
                let count = results.tracks.len();
                let artist_count = results.artists.len();
                let album_count = results.albums.len();

                // Fast path: show textual results immediately, then hydrate artwork.
                let _ = proxy.emit(SearchAppEvent::Results(results.clone()));
                let _ = proxy.emit(SystemAppEvent::StatusMessage(format!(
                    "Search complete: {count} tracks, {artist_count} artists, {album_count} albums. Loading artwork..."
                )));

                let track_jobs = results
                    .tracks
                    .iter()
                    .enumerate()
                    .filter_map(|(index, track)| {
                        track.album_image_url.as_ref().map(|url| {
                            (
                                index,
                                format!("search-track-artwork:{}", track.id),
                                url.clone(),
                            )
                        })
                    })
                    .collect::<Vec<_>>();

                let loaded_track_images = load_images_parallel(&mut proxy, track_jobs).await;
                for (index, key) in loaded_track_images {
                    if let Some(track) = results.tracks.get_mut(index) {
                        track.album_image_key = Some(key);
                    }
                }

                let artist_jobs = results
                    .artists
                    .iter()
                    .enumerate()
                    .filter_map(|(index, artist)| {
                        artist
                            .image_url
                            .as_ref()
                            .map(|url| (index, format!("search-artist-artwork:{}", artist.id), url.clone()))
                    })
                    .collect::<Vec<_>>();

                let loaded_artist_images = load_images_parallel(&mut proxy, artist_jobs).await;
                for (index, key) in loaded_artist_images {
                    if let Some(artist) = results.artists.get_mut(index) {
                        artist.image_key = Some(key);
                    }
                }

                let album_jobs = results
                    .albums
                    .iter()
                    .enumerate()
                    .filter_map(|(index, album)| {
                        album
                            .image_url
                            .as_ref()
                            .map(|url| (index, format!("search-album-artwork:{}", album.id), url.clone()))
                    })
                    .collect::<Vec<_>>();

                let loaded_album_images = load_images_parallel(&mut proxy, album_jobs).await;
                for (index, key) in loaded_album_images {
                    if let Some(album) = results.albums.get_mut(index) {
                        album.image_key = Some(key);
                    }
                }

                let _ = proxy.emit(SearchAppEvent::Results(results));
                let _ = proxy.emit(SystemAppEvent::StatusMessage(format!(
                    "Search complete: {count} tracks, {artist_count} artists, {album_count} albums."
                )));
            }
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }
    });
}

// Refreshes the list of available playback devices from Spotify.
pub fn refresh_playback_devices(backend: SharedBackend, mut proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        if let Err(err) = ensure_fresh_access_token_async(&backend).await {
            let _ = proxy.emit(SystemAppEvent::Error(err));
            return;
        }

        let spotify = {
            let state = backend.lock().unwrap();
            state.spotify.clone()
        };

        match spotify.list_playback_devices().await {
            Ok(devices) => {
                let _ = proxy.emit(PlaybackAppEvent::Devices(devices));
                let _ = proxy.emit(SystemAppEvent::StatusMessage(
                    "Playback devices refreshed.".to_string(),
                ));
            }
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }
    });
}

// Refreshes the user's playlists by fetching them from Spotify and
// emitting them to the UI.
pub fn refresh_user_playlists(backend: SharedBackend, mut proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        if let Err(err) = ensure_fresh_access_token_async(&backend).await {
            let _ = proxy.emit(SystemAppEvent::Error(err));
            return;
        }

        let spotify = {
            let state = backend.lock().unwrap();
            state.spotify.clone()
        };

        match spotify.list_user_playlists(20).await {
            Ok(playlists) => {
                let mut mapped = Vec::with_capacity(playlists.len());
                let mut image_jobs = Vec::new();

                for (index, playlist) in playlists.into_iter().enumerate() {
                    if let Some(url) = playlist.image_url.as_ref() {
                        image_jobs.push((index, format!("playlist-artwork:{}", url), url.clone()));
                    }

                    mapped.push(PlaylistEntry {
                        name: playlist.name,
                        image_key: None,
                        id: playlist.id,
                        track_count: playlist.track_count,
                        total_duration_ms: 0,
                    });
                }

                let loaded_images = load_images_parallel(&mut proxy, image_jobs).await;
                for (index, key) in loaded_images {
                    if let Some(playlist) = mapped.get_mut(index) {
                        playlist.image_key = Some(key);
                    }
                }

                let _ = proxy.emit(PlaylistsAppEvent::Playlists(mapped));
            }
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }
    });
}

// Fetches the tracks for the specified playlist from Spotify and emits them to the UI.
pub fn fetch_playlist_tracks(
    backend: SharedBackend,
    playlist_id: String,
    playlist_name: String,
    mut proxy: ContextProxy,
) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        if let Err(err) = ensure_fresh_access_token_async(&backend).await {
            let _ = proxy.emit(SystemAppEvent::Error(err));
            return;
        }

        let spotify = {
            let state = backend.lock().unwrap();
            state.spotify.clone()
        };

        match spotify.get_playlist_tracks(&playlist_id, 50).await {
            Ok(mut tracks) => {
                let image_jobs = tracks
                    .iter()
                    .enumerate()
                    .filter_map(|(index, track)| {
                        track.album_image_url.as_ref().map(|url| {
                            (
                                index,
                                format!("playlist-track-artwork:{}", url),
                                url.clone(),
                            )
                        })
                    })
                    .collect::<Vec<_>>();

                let loaded_images = load_images_parallel(&mut proxy, image_jobs).await;
                for (index, key) in loaded_images {
                    if let Some(track) = tracks.get_mut(index) {
                        track.album_image_key = Some(key);
                    }
                }

                let count = tracks.len();
                let total_duration_ms = tracks
                    .iter()
                    .map(|track| track.duration_ms as u64)
                    .sum::<u64>();
                let _ = proxy.emit(PlaylistsAppEvent::PlaylistTracks {
                    id: playlist_id,
                    name: playlist_name,
                    tracks,
                    track_count: count,
                    total_duration_ms,
                });
                let _ = proxy.emit(SystemAppEvent::StatusMessage(format!(
                    "Loaded {count} tracks from playlist."
                )));
            }
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }
    });
}

// Plays a single track on the local playback device.
pub fn playback_play_local_track(backend: SharedBackend, track: Track, proxy: ContextProxy) {
    proxy.spawn(move |proxy| {
        let result = {
            let mut state = backend.lock().unwrap();
            state.playback.play_track(&track)
        };

        match result {
            Ok(()) => {
                let _ = proxy.emit(SystemAppEvent::StatusMessage(
                    "Local track playback started.".to_string(),
                ));
            }
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }
    });
}

// Pauses local playback of the current track.
pub fn playback_pause_local(backend: SharedBackend, proxy: ContextProxy) {
    proxy.spawn(move |proxy| {
        let result = {
            let state = backend.lock().unwrap();
            state.playback.pause()
        };

        match result {
            Ok(()) => {
                let _ = proxy.emit(SystemAppEvent::StatusMessage(
                    "Local playback paused.".to_string(),
                ));
            }
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }
    });
}

// Resumes local playback of the current track.
pub fn playback_resume_local(backend: SharedBackend, proxy: ContextProxy) {
    proxy.spawn(move |proxy| {
        let result = {
            let state = backend.lock().unwrap();
            state.playback.resume()
        };

        match result {
            Ok(()) => {
                let _ = proxy.emit(SystemAppEvent::StatusMessage(
                    "Local playback resumed.".to_string(),
                ));
            }
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }
    });
}

// Plays the selected track on the active Spotify device.
pub fn playback_play_selected_track(
    backend: SharedBackend,
    track_id: String,
    mut proxy: ContextProxy,
) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        if let Err(err) = ensure_fresh_access_token_async(&backend).await {
            let _ = proxy.emit(SystemAppEvent::Error(err));
            return;
        }

        if let Err(err) = ensure_active_playback_device_async(&backend).await {
            let _ = proxy.emit(SystemAppEvent::Error(err));
            return;
        }

        let spotify = {
            let state = backend.lock().unwrap();
            state.spotify.clone()
        };

        match spotify.playback_play_track(&track_id).await {
            Ok(()) => {
                let _ = proxy.emit(SystemAppEvent::StatusMessage(
                    "Playing selected track on Spotify device.".to_string(),
                ));
            }
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }
    });
}

// Stops playback on both the local device and the active Spotify device.
pub fn playback_stop(backend: SharedBackend, mut proxy: ContextProxy) {
    // Stop the local player immediately.
    {
        let state = backend.lock().unwrap();
        let _ = state.playback.stop();
    }

    // let runtime = {
    //     let state = backend.lock().unwrap();
    //     Arc::clone(&state.runtime)
    // };

    // runtime.spawn(async move {
    //     playback_action_remote_async(&backend, &mut proxy, "Stop", RemotePlaybackAction::Stop)
    //         .await;
    // });
}

// Pauses playback on the active Spotify device.
pub fn playback_pause(backend: SharedBackend, mut proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        playback_action_remote_async(&backend, &mut proxy, "Pause", RemotePlaybackAction::Pause)
            .await;
    });
}

// Resumes playback on the active Spotify device.
pub fn playback_resume_remote(backend: SharedBackend, mut proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        playback_action_remote_async(&backend, &mut proxy, "Resume", RemotePlaybackAction::Resume)
            .await;
    });
}

// Skips to the next track on the active Spotify device.
pub fn playback_next(backend: SharedBackend, mut proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        playback_action_remote_async(&backend, &mut proxy, "Next", RemotePlaybackAction::Next)
            .await;
    });
}

// Sets the volume on the active Spotify device.
pub fn playback_set_volume(backend: SharedBackend, volume_percent: u8, mut proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };
    let target_volume = volume_percent.min(100);

    runtime.spawn(async move {
        playback_action_remote_async(
            &backend,
            &mut proxy,
            "Set volume",
            RemotePlaybackAction::SetVolume(target_volume),
        )
        .await;
    });
}

// Seeks to the specified position in the current track in local playback.
pub fn playback_seek_local(backend: SharedBackend, position_ms: u32, proxy: ContextProxy) {
    proxy.spawn(move |proxy| {
        let result = {
            let state = backend.lock().unwrap();
            state.playback.seek_to(position_ms)
        };

        if let Err(err) = result {
            let _ = proxy.emit(SystemAppEvent::Error(err));
        }
    });
}

// Seeks to the specified position in the current track on the active Spotify device.
pub fn playback_seek(backend: SharedBackend, position_ms: u32, mut proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        if let Err(err) = ensure_fresh_access_token_async(&backend).await {
            let _ = proxy.emit(SystemAppEvent::Error(err));
            return;
        }

        if let Err(err) = ensure_active_playback_device_async(&backend).await {
            let _ = proxy.emit(SystemAppEvent::Error(err));
            return;
        }

        let spotify = {
            let state = backend.lock().unwrap();
            state.spotify.clone()
        };

        if let Err(err) = spotify.playback_seek(position_ms).await {
            let _ = proxy.emit(SystemAppEvent::Error(err));
        }
    });
}

// Transfers playback to a selected Spotify device.
pub fn playback_transfer_device(
    backend: SharedBackend,
    device_id: String,
    mut proxy: ContextProxy,
) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        if let Err(err) = ensure_fresh_access_token_async(&backend).await {
            let _ = proxy.emit(SystemAppEvent::Error(err));
            return;
        }

        let spotify = {
            let state = backend.lock().unwrap();
            state.spotify.clone()
        };

        match spotify.transfer_playback(&device_id).await {
            Ok(()) => {
                let _ = proxy.emit(SystemAppEvent::StatusMessage(
                    "Playback transferred to selected Spotify device.".to_string(),
                ));
            }
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }
    });
}

// Moves to the previous track on the active Spotify device.
pub fn playback_previous(backend: SharedBackend, mut proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        playback_action_remote_async(
            &backend,
            &mut proxy,
            "Previous",
            RemotePlaybackAction::Previous,
        )
        .await;
    });
}

// Helper function to perform a playback action on the active Spotify device, ensuring that the access token is fresh and a playback device is active before making the API call.
async fn playback_action_remote_async(
    backend: &SharedBackend,
    proxy: &mut ContextProxy,
    label: &'static str,
    action: RemotePlaybackAction,
) {
    if let Err(err) = ensure_fresh_access_token_async(backend).await {
        let _ = proxy.emit(SystemAppEvent::Error(err));
        return;
    }

    if let Err(err) = ensure_active_playback_device_async(backend).await {
        let _ = proxy.emit(SystemAppEvent::Error(err));
        return;
    }

    let spotify = {
        let state = backend.lock().unwrap();
        state.spotify.clone()
    };

    let result = match action {
        RemotePlaybackAction::Pause => spotify.playback_pause().await,
        RemotePlaybackAction::Resume => spotify.playback_resume().await,
        RemotePlaybackAction::Stop => spotify.playback_stop().await,
        RemotePlaybackAction::Next => spotify.playback_next().await,
        RemotePlaybackAction::Previous => spotify.playback_previous().await,
        RemotePlaybackAction::SetVolume(volume) => spotify.playback_set_volume(volume).await,
    };

    match result {
        Ok(()) => {
            let _ = proxy.emit(SystemAppEvent::StatusMessage(format!(
                "Playback {label} command sent."
            )));
        }
        Err(err) => {
            let _ = proxy.emit(SystemAppEvent::Error(err));
        }
    }
}

enum RemotePlaybackAction {
    Pause,
    Resume,
    Stop,
    Next,
    Previous,
    SetVolume(u8),
}

async fn ensure_active_playback_device_async(backend: &SharedBackend) -> Result<(), String> {
    let spotify = {
        let state = backend.lock().unwrap();
        state.spotify.clone()
    };

    let devices = spotify.list_playback_devices().await?;

    if devices.iter().any(|device| device.is_active) {
        return Ok(());
    }

    let device_id = devices
        .iter()
        .find_map(|device| device.id.clone())
        .ok_or_else(|| {
            "No available Spotify playback device. Open Spotify on a device or use local playback."
                .to_string()
        })?;

    spotify.transfer_playback(&device_id).await?;

    Ok(())
}

async fn ensure_fresh_access_token_async(backend: &SharedBackend) -> Result<(), String> {
    let (needs_refresh, cid, rt) = {
        let state = backend.lock().unwrap();
        (
            state.token_needs_refresh(),
            state.client_id.clone(),
            state.refresh_token.clone(),
        )
    };

    if !needs_refresh {
        return Ok(());
    }

    let (cid, rt) = match (cid, rt) {
        (Some(cid), Some(rt)) => (cid, rt),
        _ => {
            return Err(
                "Token refresh required but client ID or refresh token is missing.".to_string(),
            );
        }
    };

    let tokens = oauth::refresh_access_token(&cid, &rt)
        .await
        .map_err(|err| format!("Auto-refresh failed: {err}"))?;

    let expires_at = BackendState::now_secs() + tokens.expires_in;
    let new_refresh = tokens.refresh_token.clone();

    {
        let mut state = backend.lock().unwrap();
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

    let refresh_token = backend.lock().unwrap().refresh_token.clone();
    let _ = TokenStore {
        access_token: tokens.access_token,
        refresh_token,
        expires_at: Some(expires_at),
    }
    .save();

    Ok(())
}

// Applies the token response from Spotify by updating the backend state with the new access token, refresh token, and expiration time. It also attempts to bootstrap the playback state with the new access token and emits a login profile event to update the UI with the user's profile information.
fn apply_token_response(
    backend: &SharedBackend,
    tokens: &oauth::TokenResponse,
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

// Emits a login profile event to the UI with the user's Spotify profile information, including their display name and profile image if available.
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
