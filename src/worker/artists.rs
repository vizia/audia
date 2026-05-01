use std::sync::Arc;

use vizia::prelude::ContextProxy;

use crate::ui::events::{SearchAppEvent, SystemAppEvent};

use super::{SharedBackend, load_images_parallel, with_spotify_auth_retry};

pub fn fetch_artist_view(backend: SharedBackend, artist_id: String, mut proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        let (artist, (mut albums, album_total)) = match tokio::try_join!(
            with_spotify_auth_retry(&backend, |spotify| {
                let artist_id = artist_id.clone();
                async move { spotify.get_artist(&artist_id).await }
            }),
            with_spotify_auth_retry(&backend, |spotify| {
                let artist_id = artist_id.clone();
                async move { spotify.get_artist_albums_page(&artist_id, 10, 0).await }
            })
        ) {
            Ok(result) => result,
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
                return;
            }
        };

        let artist_id = artist.id.clone();
        let artist_name = artist.name.clone();
        let artist_image_url = artist.image_url.clone();

        let mut offset = albums.len();
        let mut has_more = offset < album_total;

        let _ = proxy.emit(SearchAppEvent::ArtistView {
            id: artist_id.clone(),
            name: artist_name.clone(),
            image_key: None,
            albums: albums.clone(),
        });

        let first_page_image_ids = albums
            .iter()
            .map(|album| album.id.clone())
            .collect::<Vec<_>>();
        let first_page_image_jobs = albums
            .iter()
            .enumerate()
            .filter_map(|(index, album)| {
                album.image_url.as_ref().map(|url| {
                    (
                        index,
                        format!("artist-album-artwork:{}:{}", artist_id, album.id),
                        url.clone(),
                    )
                })
            })
            .collect::<Vec<_>>();

        let first_page_images = load_images_parallel(&mut proxy, first_page_image_jobs).await;
        for (local_index, key) in first_page_images {
            if let Some(album_id) = first_page_image_ids.get(local_index)
                && let Some(album) = albums.iter_mut().find(|album| album.id == *album_id)
            {
                album.image_key = Some(key);
            }
        }

        let _ = proxy.emit(SearchAppEvent::ArtistView {
            id: artist_id.clone(),
            name: artist_name.clone(),
            image_key: None,
            albums: albums.clone(),
        });

        while has_more {
            let page_result = with_spotify_auth_retry(&backend, |spotify| {
                let artist_id = artist_id.clone();
                async move { spotify.get_artist_albums_page(&artist_id, 10, offset).await }
            })
            .await;

            let (mut page_albums, total) = match page_result {
                Ok(page) => page,
                Err(err) => {
                    let _ = proxy.emit(SystemAppEvent::Error(err));
                    return;
                }
            };

            let page_size = page_albums.len();
            let page_image_ids = page_albums
                .iter()
                .map(|album| album.id.clone())
                .collect::<Vec<_>>();
            let page_image_jobs = page_albums
                .iter()
                .enumerate()
                .filter_map(|(index, album)| {
                    album.image_url.as_ref().map(|url| {
                        (
                            index,
                            format!("artist-album-artwork:{}:{}", artist_id, album.id),
                            url.clone(),
                        )
                    })
                })
                .collect::<Vec<_>>();

            albums.append(&mut page_albums);
            albums.sort_by(|a, b| {
                b.release_date
                    .cmp(&a.release_date)
                    .then_with(|| a.name.cmp(&b.name))
            });
            albums.dedup_by(|a, b| a.id == b.id);

            let _ = proxy.emit(SearchAppEvent::ArtistView {
                id: artist_id.clone(),
                name: artist_name.clone(),
                image_key: None,
                albums: albums.clone(),
            });

            let page_images = load_images_parallel(&mut proxy, page_image_jobs).await;
            for (local_index, key) in page_images {
                if let Some(album_id) = page_image_ids.get(local_index)
                    && let Some(album) = albums.iter_mut().find(|album| album.id == *album_id)
                {
                    album.image_key = Some(key);
                }
            }

            let _ = proxy.emit(SearchAppEvent::ArtistView {
                id: artist_id.clone(),
                name: artist_name.clone(),
                image_key: None,
                albums: albums.clone(),
            });

            offset += page_size;
            has_more = page_size > 0 && offset < total;
        }

        let artist_image_key = if let Some(url) = artist_image_url.as_ref() {
            let key = format!("artist-artwork:{}", artist_id);
            let image_jobs = vec![(0usize, key, url.clone())];
            let loaded = load_images_parallel(&mut proxy, image_jobs).await;
            loaded.into_iter().next().map(|(_, loaded_key)| loaded_key)
        } else {
            None
        };

        if artist_image_key.is_some() {
            let _ = proxy.emit(SearchAppEvent::ArtistView {
                id: artist_id.clone(),
                name: artist_name.clone(),
                image_key: artist_image_key.clone(),
                albums: albums.clone(),
            });
        }

        let album_count = albums.len();

        let _ = proxy.emit(SearchAppEvent::ArtistView {
            id: artist_id,
            name: artist_name,
            image_key: artist_image_key,
            albums,
        });

        let _ = proxy.emit(SystemAppEvent::StatusMessage(format!(
            "Loaded artist details: {album_count} albums."
        )));
    });
}

pub fn fetch_artist_view_from_track(
    backend: SharedBackend,
    track_id: String,
    mut proxy: ContextProxy,
) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        let artist = match with_spotify_auth_retry(&backend, |spotify| {
            let track_id = track_id.clone();
            async move { spotify.get_primary_artist_for_track(&track_id).await }
        })
        .await
        {
            Ok(artist) => artist,
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
                return;
            }
        };

        fetch_artist_view(backend, artist.id, proxy);
    });
}
