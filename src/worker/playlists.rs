use std::sync::Arc;

use vizia::prelude::ContextProxy;

use crate::messages::PlaylistEntry;
use crate::ui::events::{PlaylistsAppEvent, SystemAppEvent};

use super::{SharedBackend, load_images_parallel, with_spotify_auth_retry};

pub fn refresh_user_playlists(backend: SharedBackend, mut proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        match with_spotify_auth_retry(&backend, |spotify| async move {
            spotify.list_user_playlists(20).await
        })
        .await
        {
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
        match with_spotify_auth_retry(&backend, |spotify| {
            let playlist_id = playlist_id.clone();
            async move { spotify.get_playlist_tracks(&playlist_id, 50).await }
        })
        .await
        {
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
                let total_duration_ms = tracks.iter().map(|track| track.duration_ms as u64).sum::<u64>();
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
