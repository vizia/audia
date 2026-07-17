use vizia::prelude::{EventContext, Task, TaskHandle, TaskResult};

use crate::messages::Album;
use crate::ui::events::{AlbumEvent, AlbumTracksData, SearchEvent, SystemEvent};

use super::{SharedBackend, with_spotify_auth_retry};

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
            TaskResult::Completed(mut data) => {
                data.image_key = data.image_url.clone();
                for track in &mut data.tracks {
                    track.album_image_key = data.image_key.clone();
                }

                let track_count = data.track_count;
                let _ = proxy.emit(AlbumEvent::AlbumTracks(data));
                let _ = proxy.emit(SystemEvent::StatusMessage(format!(
                    "Loaded {} tracks from album.",
                    track_count
                )));
            }
            TaskResult::Error(err) => {
                let _ = proxy.emit(SystemEvent::Error(err));
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
                let _ = proxy.emit(SearchEvent::LoadAlbumTracks(album));
            }
            TaskResult::Error(err) => {
                let _ = proxy.emit(SystemEvent::Error(err));
            }
            TaskResult::Timeout | TaskResult::Cancelled | TaskResult::Disconnected { .. } => {}
        }),
    )
}
