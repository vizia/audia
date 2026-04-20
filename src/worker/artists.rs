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
        let artist = match with_spotify_auth_retry(&backend, |spotify| {
            let artist_id = artist_id.clone();
            async move { spotify.get_artist(&artist_id).await }
        })
        .await
        {
            Ok(artist) => artist,
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
                return;
            }
        };

        let mut albums = match with_spotify_auth_retry(&backend, |spotify| {
            let artist_id = artist.id.clone();
            async move { spotify.get_artist_albums(&artist_id).await }
        })
        .await
        {
            Ok(result) => result,
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
                return;
            }
        };

        let artist_image_key = if let Some(url) = artist.image_url.as_ref() {
            let key = format!("artist-artwork:{}", artist.id);
            let image_jobs = vec![(0usize, key, url.clone())];
            let loaded = load_images_parallel(&mut proxy, image_jobs).await;
            loaded.into_iter().next().map(|(_, loaded_key)| loaded_key)
        } else {
            None
        };

        let album_image_jobs = albums
            .iter()
            .enumerate()
            .filter_map(|(index, album)| {
                album.image_url.as_ref().map(|url| {
                    (
                        index,
                        format!("artist-album-artwork:{}:{}", artist.id, album.id),
                        url.clone(),
                    )
                })
            })
            .collect::<Vec<_>>();

        let loaded_album_images = load_images_parallel(&mut proxy, album_image_jobs).await;
        for (index, key) in loaded_album_images {
            if let Some(album) = albums.get_mut(index) {
                album.image_key = Some(key);
            }
        }

        let album_count = albums.len();

        let _ = proxy.emit(SearchAppEvent::ArtistView {
            id: artist.id,
            name: artist.name,
            image_key: artist_image_key,
            albums,
        });

        let _ = proxy.emit(SystemAppEvent::StatusMessage(format!(
            "Loaded artist details: {album_count} albums."
        )));
    });
}

pub fn fetch_artist_view_by_name(
    backend: SharedBackend,
    artist_name: String,
    mut proxy: ContextProxy,
) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        let primary_query = artist_name
            .split(',')
            .next()
            .unwrap_or(artist_name.as_str())
            .trim()
            .to_string();

        let matched_artist = match with_spotify_auth_retry(&backend, |spotify| {
            let query = primary_query.clone();
            async move { spotify.search_artist_first(&query).await }
        })
        .await
        {
            Ok(Some(artist)) => artist,
            Ok(None) => {
                let _ = proxy.emit(SystemAppEvent::StatusMessage(
                    "Could not find this artist on Spotify.".to_string(),
                ));
                return;
            }
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
                return;
            }
        };

        fetch_artist_view(backend, matched_artist.id, proxy);
    });
}
