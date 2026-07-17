use vizia::prelude::{EventContext, Task, TaskHandle, TaskResult};

use crate::ui::events::{SearchEvent, SystemEvent};

use super::{SharedBackend, with_spotify_auth_retry};

pub fn search_tracks(backend: SharedBackend, query: String, cx: &EventContext<'_>) -> TaskHandle {
    let task_name = ("search-tracks", query.clone());
    cx.add_task(
        Task::new(move |_| {
            let backend = backend.clone();
            let query = query.clone();
            async move {
                with_spotify_auth_retry(&backend, |spotify| {
                    let query = query.clone();
                    async move { spotify.search_catalog(&query).await }
                })
                .await
            }
        })
        .name(task_name)
        .on_result(|result, proxy| match result {
            TaskResult::Completed(mut results) => {
                for track in &mut results.tracks {
                    track.album_image_key = track.album_image_url.clone();
                }
                for artist in &mut results.artists {
                    artist.image_key = artist.image_url.clone();
                }
                for album in &mut results.albums {
                    album.image_key = album.image_url.clone();
                }

                let count = results.tracks.len();
                let artist_count = results.artists.len();
                let album_count = results.albums.len();

                let _ = proxy.emit(SearchEvent::Results(results));
                let _ = proxy.emit(SystemEvent::StatusMessage(format!(
                    "Search complete: {count} tracks, {artist_count} artists, {album_count} albums."
                )));
            }
            TaskResult::Error(err) => {
                let _ = proxy.emit(SystemEvent::Error(err));
            }
            TaskResult::Timeout | TaskResult::Cancelled | TaskResult::Disconnected { .. } => {}
        }),
    )
}
