use vizia::prelude::{EventContext, Task, TaskHandle, TaskResult};

use crate::{
    messages::Album,
    ui::events::{ArtistEvent, SystemEvent},
};

use super::{SharedBackend, with_spotify_auth_retry};

#[derive(Clone, Debug)]
struct ArtistViewTaskData {
    id: String,
    name: String,
    image_url: Option<String>,
    albums: Vec<Album>,
}

async fn collect_artist_view_data(
    backend: SharedBackend,
    artist_id: String,
) -> Result<ArtistViewTaskData, String> {
    let (artist, (mut albums, album_total)) = tokio::try_join!(
        with_spotify_auth_retry(&backend, |spotify| {
            let artist_id = artist_id.clone();
            async move { spotify.get_artist(&artist_id).await }
        }),
        with_spotify_auth_retry(&backend, |spotify| {
            let artist_id = artist_id.clone();
            async move { spotify.get_artist_albums_page(&artist_id, 10, 0).await }
        })
    )?;

    let artist_id = artist.id;
    let mut offset = albums.len();
    let mut has_more = offset < album_total;

    while has_more {
        let (mut page_albums, total) = with_spotify_auth_retry(&backend, |spotify| {
            let artist_id = artist_id.clone();
            async move { spotify.get_artist_albums_page(&artist_id, 10, offset).await }
        })
        .await?;

        let page_size = page_albums.len();
        albums.append(&mut page_albums);
        albums.sort_by(|a, b| {
            b.release_date
                .cmp(&a.release_date)
                .then_with(|| a.name.cmp(&b.name))
        });
        albums.dedup_by(|a, b| a.id == b.id);

        offset += page_size;
        has_more = page_size > 0 && offset < total;
    }

    Ok(ArtistViewTaskData {
        id: artist_id,
        name: artist.name,
        image_url: artist.image_url,
        albums,
    })
}

pub fn fetch_artist_view(
    backend: SharedBackend,
    artist_id: String,
    cx: &EventContext<'_>,
) -> TaskHandle {
    let task_name = ("fetch-artist-view", artist_id.clone());
    cx.add_task(
        Task::new(move |_| {
            let backend = backend.clone();
            let artist_id = artist_id.clone();
            async move { collect_artist_view_data(backend, artist_id).await }
        })
        .name(task_name)
        .on_result(|result, proxy| match result {
            TaskResult::Completed(data) => {
                let mut albums = data.albums;
                for album in &mut albums {
                    album.image_key = album.image_url.clone();
                }

                let album_count = albums.len();
                let _ = proxy.emit(ArtistEvent::ArtistView {
                    id: data.id,
                    name: data.name,
                    image_key: data.image_url,
                    albums,
                });
                let _ = proxy.emit(SystemEvent::StatusMessage(format!(
                    "Loaded artist details: {album_count} albums."
                )));
            }
            TaskResult::Error(err) => {
                let _ = proxy.emit(SystemEvent::Error(err));
            }
            TaskResult::Timeout | TaskResult::Cancelled | TaskResult::Disconnected { .. } => {}
        }),
    )
}

pub fn fetch_artist_view_from_track(
    backend: SharedBackend,
    track_id: String,
    cx: &EventContext<'_>,
) -> TaskHandle {
    let task_name = ("fetch-artist-view-from-track", track_id.clone());
    cx.add_task(
        Task::new(move |_| {
            let backend = backend.clone();
            let track_id = track_id.clone();
            async move {
                let artist = with_spotify_auth_retry(&backend, |spotify| {
                    let track_id = track_id.clone();
                    async move { spotify.get_primary_artist_for_track(&track_id).await }
                })
                .await?;

                collect_artist_view_data(backend, artist.id).await
            }
        })
        .name(task_name)
        .on_result(|result, proxy| match result {
            TaskResult::Completed(data) => {
                let mut albums = data.albums;
                for album in &mut albums {
                    album.image_key = album.image_url.clone();
                }

                let album_count = albums.len();
                let _ = proxy.emit(ArtistEvent::ArtistView {
                    id: data.id,
                    name: data.name,
                    image_key: data.image_url,
                    albums,
                });
                let _ = proxy.emit(SystemEvent::StatusMessage(format!(
                    "Loaded artist details: {album_count} albums."
                )));
            }
            TaskResult::Error(err) => {
                let _ = proxy.emit(SystemEvent::Error(err));
            }
            TaskResult::Timeout | TaskResult::Cancelled | TaskResult::Disconnected { .. } => {}
        }),
    )
}
