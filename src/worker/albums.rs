use vizia::prelude::{ContextProxy, EventContext, Task, TaskHandle, TaskResult};

use crate::messages::{Album, Track};
use crate::ui::events::{AlbumTracksData, SearchAppEvent, SystemAppEvent};

use super::{SharedBackend, load_images_parallel, with_spotify_auth_retry};

async fn fetch_album_tracks_inner(
    backend: SharedBackend,
    album: Album,
) -> Result<AlbumTracksData, String> {
    let album_id = album.id.clone();
    let (tracks, release_year) = tokio::try_join!(
        with_spotify_auth_retry(&backend, |spotify| {
            let album_id = album_id.clone();
            async move { spotify.get_album_tracks(&album_id).await }
        }),
        with_spotify_auth_retry(&backend, |spotify| {
            let album_id = album_id.clone();
            async move { Ok(spotify.get_album_release_year(&album_id).await) }
        })
    )?;

    let track_count = tracks.len();
    let total_duration_ms: u64 = tracks.iter().map(|t| t.duration_ms as u64).sum();

    Ok(AlbumTracksData {
        id: album.id,
        name: album.name,
        artist: album.artist,
        image_url: album.image_url,
        image_key: None,
        tracks,
        release_year,
        track_count,
        total_duration_ms,
    })
}

async fn hydrate_album_artwork_inner(
    mut data: AlbumTracksData,
    proxy: &mut ContextProxy,
) -> Result<AlbumTracksData, String> {
    let image_key = if let Some(url) = &data.image_url {
        let key = format!("album-artwork:{}", url);
        let image_jobs = vec![(0usize, key.clone(), url.clone())];
        let loaded = load_images_parallel(proxy, image_jobs).await;
        loaded.into_iter().next().map(|(_, k)| k)
    } else {
        None
    };

    for track in &mut data.tracks {
        track.album_image_key = image_key.clone();
    }
    data.image_key = image_key;

    Ok(data)
}

pub fn fetch_album_tracks(
    backend: SharedBackend,
    album: Album,
    cx: &EventContext<'_>,
) -> TaskHandle {
    let task_name = ("fetch-album-tracks", album.id.clone());
    cx.add_task(
        Task::new(move |_| {
            let backend = backend.clone();
            let album = album.clone();
            async move { fetch_album_tracks_inner(backend, album).await }
        })
        .name(task_name)
        .on_result(|result, proxy| match result {
            TaskResult::Completed(data) => {
                let track_count = data.track_count;
                let _ = proxy.emit(SearchAppEvent::AlbumTracks(AlbumTracksData {
                    image_key: None,
                    ..data.clone()
                }));
                if data.image_url.is_some() {
                    let _ = proxy.emit(SystemAppEvent::StatusMessage(format!(
                        "Loaded {} tracks from album. Loading artwork...",
                        track_count
                    )));
                    let _ = proxy.emit(SearchAppEvent::HydrateAlbumArtwork(data));
                } else {
                    let _ = proxy.emit(SystemAppEvent::StatusMessage(format!(
                        "Loaded {} tracks from album.",
                        track_count
                    )));
                }
            }
            TaskResult::Error(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
            TaskResult::Timeout | TaskResult::Cancelled | TaskResult::Disconnected { .. } => {}
        }),
    )
}

pub fn hydrate_album_artwork(
    id: String,
    name: String,
    artist: String,
    image_url: Option<String>,
    tracks: Vec<Track>,
    release_year: Option<u32>,
    track_count: usize,
    total_duration_ms: u64,
    cx: &EventContext<'_>,
) -> TaskHandle {
    let proxy = cx.get_proxy();
    let task_name = ("hydrate-album-artwork", id.clone());
    cx.add_task(
        Task::new(move |_| {
            let mut proxy = proxy.clone();
            let data = AlbumTracksData {
                id: id.clone(),
                name: name.clone(),
                artist: artist.clone(),
                image_url: image_url.clone(),
                image_key: None,
                tracks: tracks.clone(),
                release_year,
                track_count,
                total_duration_ms,
            };
            async move { hydrate_album_artwork_inner(data, &mut proxy).await }
        })
        .name(task_name)
        .on_result(|result, proxy| match result {
            TaskResult::Completed(data) => {
                let track_count = data.track_count;
                let _ = proxy.emit(SearchAppEvent::AlbumTracks(data));
                let _ = proxy.emit(SystemAppEvent::StatusMessage(format!(
                    "Loaded {} tracks from album.",
                    track_count
                )));
            }
            TaskResult::Error(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
            TaskResult::Timeout | TaskResult::Cancelled | TaskResult::Disconnected { .. } => {}
        }),
    )
}

pub fn fetch_album_from_track(
    backend: SharedBackend,
    track_id: String,
    cx: &EventContext<'_>,
) -> TaskHandle {
    let task_name = ("fetch-album-from-track", track_id.clone());
    cx.add_task(
        Task::new(move |_| {
            let backend = backend.clone();
            let track_id = track_id.clone();
            async move {
                with_spotify_auth_retry(&backend, |spotify| {
                    let track_id = track_id.clone();
                    async move { spotify.get_album_for_track(&track_id).await }
                })
                .await
            }
        })
        .name(task_name)
        .on_result(|result, proxy| match result {
            TaskResult::Completed(album) => {
                let _ = proxy.emit(SearchAppEvent::LoadAlbumTracks(album));
            }
            TaskResult::Error(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
            TaskResult::Timeout | TaskResult::Cancelled | TaskResult::Disconnected { .. } => {}
        }),
    )
}
