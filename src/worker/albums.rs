use vizia::prelude::{ContextProxy, EventContext, Task, TaskHandle, TaskResult};

use crate::messages::Album;
use crate::ui::events::{SearchAppEvent, SystemAppEvent};

use super::{SharedBackend, load_images_parallel, with_spotify_auth_retry};

async fn fetch_album_tracks_inner(backend: SharedBackend, album: Album, proxy: &mut ContextProxy) {
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
                        let loaded = load_images_parallel(proxy, image_jobs).await;
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
}

pub fn fetch_album_tracks(
    backend: SharedBackend,
    album: Album,
    cx: &EventContext<'_>,
) -> TaskHandle {
    let proxy = cx.get_proxy();
    let task_name = ("fetch-album-tracks", album.id.clone());
    cx.add_task(
        Task::new(move |_| {
            let mut proxy = proxy.clone();
            let backend = backend.clone();
            let album = album.clone();
            async move {
                fetch_album_tracks_inner(backend, album, &mut proxy).await;
                Ok::<(), String>(())
            }
        })
        .name(task_name)
        .on_result(|result, proxy| {
            if let TaskResult::Error(err) = result {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }),
    )
}

pub fn fetch_album_from_track(
    backend: SharedBackend,
    track_id: String,
    cx: &EventContext<'_>,
) -> TaskHandle {
    let proxy = cx.get_proxy();
    let task_name = ("fetch-album-from-track", track_id.clone());
    cx.add_task(
        Task::new(move |_| {
            let mut proxy = proxy.clone();
            let backend = backend.clone();
            let track_id = track_id.clone();
            async move {
                let album = match with_spotify_auth_retry(&backend, |spotify| {
                    let track_id = track_id.clone();
                    async move { spotify.get_album_for_track(&track_id).await }
                })
                .await
                {
                    Ok(album) => album,
                    Err(err) => {
                        let _ = proxy.emit(SystemAppEvent::Error(err));
                        return Ok::<(), String>(());
                    }
                };

                fetch_album_tracks_inner(backend, album, &mut proxy).await;

                Ok::<(), String>(())
            }
        })
        .name(task_name)
        .on_result(|result, proxy| {
            if let TaskResult::Error(err) = result {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }),
    )
}
