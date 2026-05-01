use std::sync::Arc;

use vizia::prelude::ContextProxy;

use crate::messages::Album;
use crate::ui::events::{SearchAppEvent, SystemAppEvent};

use super::{SharedBackend, load_images_parallel, with_spotify_auth_retry};

pub fn fetch_album_tracks(backend: SharedBackend, album: Album, mut proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        match with_spotify_auth_retry(&backend, |spotify| {
            let album_id = album.id.clone();
            async move { spotify.get_album_tracks(&album_id).await }
        })
        .await
        {
            Ok(mut tracks) => {
                let album_id = album.id.clone();
                let (image_key, release_year) = tokio::join!(
                    async {
                        if let Some(url) = &album.image_url {
                            let key = format!("album-artwork:{}", url);
                            let image_jobs = vec![(0usize, key.clone(), url.clone())];
                            let loaded = load_images_parallel(&mut proxy, image_jobs).await;
                            loaded.into_iter().next().map(|(_, k)| k)
                        } else {
                            None
                        }
                    },
                    with_spotify_auth_retry(&backend, |spotify| {
                        let album_id = album_id.clone();
                        async move { Ok(spotify.get_album_release_year(&album_id).await) }
                    })
                );

                for track in &mut tracks {
                    track.album_image_key = image_key.clone();
                }

                let track_count = tracks.len();
                let total_duration_ms: u64 = tracks.iter().map(|t| t.duration_ms as u64).sum();

                let _ = proxy.emit(SearchAppEvent::AlbumTracks {
                    id: album.id,
                    name: album.name,
                    artist: album.artist,
                    image_key,
                    tracks,
                    release_year: release_year.unwrap_or(None),
                    track_count,
                    total_duration_ms,
                });
                let _ = proxy.emit(SystemAppEvent::StatusMessage(format!(
                    "Loaded {track_count} tracks from album."
                )));
            }
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }
    });
}

pub fn fetch_album_from_track(backend: SharedBackend, track_id: String, mut proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        let album = match with_spotify_auth_retry(&backend, |spotify| {
            let track_id = track_id.clone();
            async move { spotify.get_album_for_track(&track_id).await }
        })
        .await
        {
            Ok(album) => album,
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
                return;
            }
        };

        match with_spotify_auth_retry(&backend, |spotify| {
            let album_id = album.id.clone();
            async move { spotify.get_album_tracks(&album_id).await }
        })
        .await
        {
            Ok(mut tracks) => {
                let album_id = album.id.clone();
                let (image_key, release_year) = tokio::join!(
                    async {
                        if let Some(url) = &album.image_url {
                            let key = format!("album-artwork:{}", url);
                            let image_jobs = vec![(0usize, key.clone(), url.clone())];
                            let loaded = load_images_parallel(&mut proxy, image_jobs).await;
                            loaded.into_iter().next().map(|(_, k)| k)
                        } else {
                            None
                        }
                    },
                    with_spotify_auth_retry(&backend, |spotify| {
                        let album_id = album_id.clone();
                        async move { Ok(spotify.get_album_release_year(&album_id).await) }
                    })
                );

                for track in &mut tracks {
                    track.album_image_key = image_key.clone();
                }

                let track_count = tracks.len();
                let total_duration_ms: u64 = tracks.iter().map(|t| t.duration_ms as u64).sum();

                let _ = proxy.emit(SearchAppEvent::AlbumTracks {
                    id: album.id,
                    name: album.name,
                    artist: album.artist,
                    image_key,
                    tracks,
                    release_year: release_year.unwrap_or(None),
                    track_count,
                    total_duration_ms,
                });
                let _ = proxy.emit(SystemAppEvent::StatusMessage(format!(
                    "Loaded {track_count} tracks from album."
                )));
            }
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }
    });
}
