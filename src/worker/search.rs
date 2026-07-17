use vizia::prelude::{EventContext, Task, TaskHandle, TaskResult};

use crate::ui::events::{SearchEvent, SystemEvent};

use crate::messages::SearchResultsData;

use super::{SharedBackend, load_images_parallel, with_spotify_auth_retry};

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
        .on_result(|result, proxy| {
            match result {
                TaskResult::Completed(results) => {
                    let count = results.tracks.len();
                    let artist_count = results.artists.len();
                    let album_count = results.albums.len();

                    let _ = proxy.emit(SearchEvent::Results(results.clone()));
                    let _ = proxy.emit(SystemEvent::StatusMessage(format!(
                        "Search complete: {count} tracks, {artist_count} artists, {album_count} albums. Loading artwork..."
                    )));
                    let _ = proxy.emit(SearchEvent::HydrateArtwork(results));
                }
                TaskResult::Error(err) => {
                    let _ = proxy.emit(SystemEvent::Error(err));
                }
                TaskResult::Timeout | TaskResult::Cancelled | TaskResult::Disconnected { .. } => {}
            }
        }),
    )
}

pub fn hydrate_search_artwork(results: SearchResultsData, cx: &EventContext<'_>) -> TaskHandle {
    let proxy = cx.get_proxy();
    cx.add_task(
        Task::new(move |_| {
            let mut proxy = proxy.clone();
            let mut results = results.clone();
            async move {
                let track_jobs = results
                    .tracks
                    .iter()
                    .enumerate()
                    .filter_map(|(index, track)| {
                        track
                            .album_image_url
                            .as_ref()
                            .map(|url| (index, url.clone()))
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
                        artist.image_url.as_ref().map(|url| (index, url.clone()))
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
                    .filter_map(|(index, album)| album.image_url.as_ref().map(|url| (index, url.clone())))
                    .collect::<Vec<_>>();

                let loaded_album_images = load_images_parallel(&mut proxy, album_jobs).await;
                for (index, key) in loaded_album_images {
                    if let Some(album) = results.albums.get_mut(index) {
                        album.image_key = Some(key);
                    }
                }

                Ok::<SearchResultsData, String>(results)
            }
        })
        .name("search-artwork-hydration")
        .on_result(|result, proxy| match result {
            TaskResult::Completed(results) => {
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
