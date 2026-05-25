use vizia::prelude::{ContextProxy, EventContext, Task, TaskHandle, TaskResult};

use crate::{
    messages::Album,
    ui::events::{ArtistEvent, SearchEvent, SystemEvent},
};

use super::{SharedBackend, load_images_parallel, with_spotify_auth_retry};

#[derive(Clone, Debug)]
struct ArtistViewTaskData {
    id: String,
    name: String,
    image_url: Option<String>,
    albums: Vec<Album>,
}

#[derive(Clone, Debug)]
struct HydratedArtistView {
    id: String,
    name: String,
    image_key: Option<String>,
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

async fn hydrate_artist_view_artwork(
    data: ArtistViewTaskData,
    proxy: &mut ContextProxy,
) -> Result<HydratedArtistView, String> {
    let mut albums = data.albums;

    let album_jobs = albums
        .iter()
        .enumerate()
        .filter_map(|(index, album)| {
            album.image_url.as_ref().map(|url| {
                (
                    index,
                    format!("artist-album-artwork:{}:{}", data.id, album.id),
                    url.clone(),
                )
            })
        })
        .collect::<Vec<_>>();

    let loaded_album_images = load_images_parallel(proxy, album_jobs).await;
    for (index, key) in loaded_album_images {
        if let Some(album) = albums.get_mut(index) {
            album.image_key = Some(key);
        }
    }

    let image_key = if let Some(url) = data.image_url.as_ref() {
        let key = format!("artist-artwork:{}", data.id);
        let image_jobs = vec![(0usize, key, url.clone())];
        let loaded = load_images_parallel(proxy, image_jobs).await;
        loaded.into_iter().next().map(|(_, loaded_key)| loaded_key)
    } else {
        None
    };

    Ok(HydratedArtistView {
        id: data.id,
        name: data.name,
        image_key,
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
                let album_count = data.albums.len();
                let _ = proxy.emit(ArtistEvent::ArtistView {
                    id: data.id.clone(),
                    name: data.name.clone(),
                    image_key: None,
                    albums: data.albums.clone(),
                });
                let _ = proxy.emit(SystemEvent::StatusMessage(format!(
                    "Loaded artist details: {album_count} albums. Loading artwork..."
                )));
                let _ = proxy.emit(SearchEvent::HydrateArtistArtwork {
                    id: data.id,
                    name: data.name,
                    image_url: data.image_url,
                    albums: data.albums,
                });
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
                let album_count = data.albums.len();
                let _ = proxy.emit(ArtistEvent::ArtistView {
                    id: data.id.clone(),
                    name: data.name.clone(),
                    image_key: None,
                    albums: data.albums.clone(),
                });
                let _ = proxy.emit(SystemEvent::StatusMessage(format!(
                    "Loaded artist details: {album_count} albums. Loading artwork..."
                )));
                let _ = proxy.emit(SearchEvent::HydrateArtistArtwork {
                    id: data.id,
                    name: data.name,
                    image_url: data.image_url,
                    albums: data.albums,
                });
            }
            TaskResult::Error(err) => {
                let _ = proxy.emit(SystemEvent::Error(err));
            }
            TaskResult::Timeout | TaskResult::Cancelled | TaskResult::Disconnected { .. } => {}
        }),
    )
}

pub fn hydrate_artist_artwork(
    id: String,
    name: String,
    image_url: Option<String>,
    albums: Vec<Album>,
    cx: &EventContext<'_>,
) -> TaskHandle {
    let proxy = cx.get_proxy();
    let task_name = ("hydrate-artist-artwork", id.clone());
    cx.add_task(
        Task::new(move |_| {
            let mut proxy = proxy.clone();
            let data = ArtistViewTaskData {
                id: id.clone(),
                name: name.clone(),
                image_url: image_url.clone(),
                albums: albums.clone(),
            };
            async move { hydrate_artist_view_artwork(data, &mut proxy).await }
        })
        .name(task_name)
        .on_result(|result, proxy| match result {
            TaskResult::Completed(view) => {
                let album_count = view.albums.len();
                let _ = proxy.emit(ArtistEvent::ArtistView {
                    id: view.id,
                    name: view.name,
                    image_key: view.image_key,
                    albums: view.albums,
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
