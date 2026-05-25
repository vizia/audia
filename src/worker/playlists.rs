use vizia::prelude::{ContextProxy, EventContext, Task, TaskHandle, TaskResult};

use crate::messages::PlaylistEntry;
use crate::ui::events::{PlaylistsAppEvent, SystemAppEvent};

use super::{SharedBackend, load_images_parallel, with_spotify_auth_retry};

async fn refresh_user_playlists_inner(backend: SharedBackend, proxy: &mut ContextProxy) {
    match with_spotify_auth_retry(&backend, |spotify| async move {
        spotify.list_user_playlists(20).await
    })
    .await
    {
        Ok(playlists) => {
            let mut mapped = Vec::with_capacity(playlists.len());
            let mut image_jobs = Vec::new();

            for (index, playlist) in playlists.into_iter().enumerate() {
                if let Some(url) = playlist.image_url.as_ref() {
                    image_jobs.push((index, format!("playlist-artwork:{}", url), url.clone()));
                }

                mapped.push(PlaylistEntry {
                    name: playlist.name,
                    image_key: None,
                    id: playlist.id,
                    track_count: playlist.track_count,
                    total_duration_ms: 0,
                });
            }

            let _ = proxy.emit(PlaylistsAppEvent::Playlists(mapped.clone()));

            let loaded_images = load_images_parallel(proxy, image_jobs).await;
            for (index, key) in loaded_images {
                if let Some(playlist) = mapped.get_mut(index) {
                    playlist.image_key = Some(key);
                }
            }

            let _ = proxy.emit(PlaylistsAppEvent::Playlists(mapped));
        }
        Err(err) => {
            let _ = proxy.emit(SystemAppEvent::Error(err));
        }
    }
}

async fn fetch_playlist_tracks_inner(
    backend: SharedBackend,
    playlist_id: String,
    playlist_name: String,
    request_id: u64,
    proxy: &mut ContextProxy,
) {
    let first_page = with_spotify_auth_retry(&backend, |spotify| {
        let playlist_id = playlist_id.clone();
        async move { spotify.get_playlist_tracks_page(&playlist_id, 50, 0).await }
    })
    .await;

    let (mut tracks, total) = match first_page {
        Ok(page) => page,
        Err(err) => {
            let _ = proxy.emit(SystemAppEvent::Error(err));
            return;
        }
    };

    let mut offset = tracks.len();
    let mut has_more = offset < total;

    let count = tracks.len();
    let total_duration_ms = tracks.iter().map(|track| track.duration_ms as u64).sum::<u64>();
    let _ = proxy.emit(PlaylistsAppEvent::PlaylistTracks {
        request_id,
        id: playlist_id.clone(),
        name: playlist_name.clone(),
        tracks: tracks.clone(),
        track_count: count,
        total_duration_ms,
    });

    let first_page_len = tracks.len();
    let first_page_image_jobs = tracks
        .iter()
        .take(first_page_len)
        .enumerate()
        .filter_map(|(index, track)| {
            track
                .album_image_url
                .as_ref()
                .map(|url| (index, format!("playlist-track-artwork:{}", url), url.clone()))
        })
        .collect::<Vec<_>>();

    let first_page_images = load_images_parallel(proxy, first_page_image_jobs).await;
    for (index, key) in first_page_images {
        if let Some(track) = tracks.get_mut(index) {
            track.album_image_key = Some(key);
        }
    }

    let count = tracks.len();
    let total_duration_ms = tracks.iter().map(|track| track.duration_ms as u64).sum::<u64>();
    let _ = proxy.emit(PlaylistsAppEvent::PlaylistTracks {
        request_id,
        id: playlist_id.clone(),
        name: playlist_name.clone(),
        tracks: tracks.clone(),
        track_count: count,
        total_duration_ms,
    });

    while has_more {
        let page_result = with_spotify_auth_retry(&backend, |spotify| {
            let playlist_id = playlist_id.clone();
            async move { spotify.get_playlist_tracks_page(&playlist_id, 50, offset).await }
        })
        .await;

        let (mut page_tracks, total) = match page_result {
            Ok(page) => page,
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
                return;
            }
        };

        let page_size = page_tracks.len();
        let page_start = tracks.len();
        tracks.append(&mut page_tracks);

        let count = tracks.len();
        let total_duration_ms = tracks.iter().map(|track| track.duration_ms as u64).sum::<u64>();
        let _ = proxy.emit(PlaylistsAppEvent::PlaylistTracks {
            request_id,
            id: playlist_id.clone(),
            name: playlist_name.clone(),
            tracks: tracks.clone(),
            track_count: count,
            total_duration_ms,
        });

        let page_end = tracks.len();
        let page_image_jobs = tracks
            .iter()
            .enumerate()
            .skip(page_start)
            .take(page_end - page_start)
            .filter_map(|(index, track)| {
                track
                    .album_image_url
                    .as_ref()
                    .map(|url| (index, format!("playlist-track-artwork:{}", url), url.clone()))
            })
            .collect::<Vec<_>>();

        let page_images = load_images_parallel(proxy, page_image_jobs).await;
        for (index, key) in page_images {
            if let Some(track) = tracks.get_mut(index) {
                track.album_image_key = Some(key);
            }
        }

        let count = tracks.len();
        let total_duration_ms = tracks.iter().map(|track| track.duration_ms as u64).sum::<u64>();
        let _ = proxy.emit(PlaylistsAppEvent::PlaylistTracks {
            request_id,
            id: playlist_id.clone(),
            name: playlist_name.clone(),
            tracks: tracks.clone(),
            track_count: count,
            total_duration_ms,
        });

        offset += page_size;
        has_more = page_size > 0 && offset < total;
    }

    let count = tracks.len();
    let total_duration_ms = tracks.iter().map(|track| track.duration_ms as u64).sum::<u64>();
    let _ = proxy.emit(PlaylistsAppEvent::PlaylistTracks {
        request_id,
        id: playlist_id,
        name: playlist_name,
        tracks,
        track_count: count,
        total_duration_ms,
    });
    let _ = proxy.emit(SystemAppEvent::StatusMessage(format!(
        "Loaded {count} tracks from playlist."
    )));
}

pub fn refresh_user_playlists(backend: SharedBackend, cx: &EventContext<'_>) {
    let proxy = cx.get_proxy();
    cx.add_task(
        Task::new(move |_| {
            let mut proxy = proxy.clone();
            let backend = backend.clone();
            async move {
            refresh_user_playlists_inner(backend, &mut proxy).await;
            Ok::<(), String>(())
            }
        })
        .name("refresh-user-playlists")
        .on_result(|result, proxy| {
            if let TaskResult::Error(err) = result {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }),
    );
}

pub fn create_playlist(backend: SharedBackend, name: String, cx: &EventContext<'_>) {
    let proxy = cx.get_proxy();
    let task_name = ("create-playlist", name.clone());
    cx.add_task(
        Task::new(move |_| {
            let mut proxy = proxy.clone();
            let backend = backend.clone();
            let name = name.clone();
            async move {
            let trimmed_name = name.trim().to_string();
            match with_spotify_auth_retry(&backend, |spotify| {
                let trimmed_name = trimmed_name.clone();
                async move { spotify.create_playlist(&trimmed_name).await }
            })
            .await
            {
                Ok(playlist) => {
                    let _ = proxy.emit(PlaylistsAppEvent::PlaylistCreated {
                        id: playlist.id,
                        name: playlist.name.clone(),
                    });
                    let _ = proxy.emit(SystemAppEvent::StatusMessage(format!(
                        "Created playlist '{}'.",
                        playlist.name
                    )));
                    refresh_user_playlists_inner(backend, &mut proxy).await;
                }
                Err(err) => {
                    let message = if err.contains("status 403") {
                        "Spotify denied playlist creation (403). Reset login and sign in again to grant playlist-modify scopes."
                            .to_string()
                    } else {
                        err
                    };
                    let _ = proxy.emit(PlaylistsAppEvent::PlaylistCreateFailed(message.clone()));
                    let _ = proxy.emit(SystemAppEvent::Error(message));
                }
            }

            Ok::<(), String>(())
            }
        })
        .name(task_name)
        .on_result(|result, proxy| {
            if let TaskResult::Error(err) = result {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }),
    );
}

pub fn rename_playlist(
    backend: SharedBackend,
    playlist_id: String,
    name: String,
    cx: &EventContext<'_>,
) {
    let proxy = cx.get_proxy();
    let task_name = ("rename-playlist", playlist_id.clone());
    cx.add_task(
        Task::new(move |_| {
            let mut proxy = proxy.clone();
            let backend = backend.clone();
            let playlist_id = playlist_id.clone();
            let name = name.clone();
            async move {
            let trimmed_name = name.trim().to_string();
            match with_spotify_auth_retry(&backend, |spotify| {
                let playlist_id = playlist_id.clone();
                let trimmed_name = trimmed_name.clone();
                async move { spotify.rename_playlist(&playlist_id, &trimmed_name).await }
            })
            .await
            {
                Ok(()) => {
                    let _ = proxy.emit(PlaylistsAppEvent::PlaylistRenamed {
                        id: playlist_id,
                        name: trimmed_name.clone(),
                    });
                    let _ = proxy.emit(SystemAppEvent::StatusMessage(format!(
                        "Renamed playlist to '{trimmed_name}'."
                    )));
                    refresh_user_playlists_inner(backend, &mut proxy).await;
                }
                Err(err) => {
                    let _ = proxy.emit(PlaylistsAppEvent::PlaylistRenameFailed(err.clone()));
                    let _ = proxy.emit(SystemAppEvent::Error(err));
                }
            }

            Ok::<(), String>(())
            }
        })
        .name(task_name)
        .on_result(|result, proxy| {
            if let TaskResult::Error(err) = result {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }),
    );
}

pub fn delete_playlist(backend: SharedBackend, playlist_id: String, cx: &EventContext<'_>) {
    let proxy = cx.get_proxy();
    let task_name = ("delete-playlist", playlist_id.clone());
    cx.add_task(
        Task::new(move |_| {
            let mut proxy = proxy.clone();
            let backend = backend.clone();
            let playlist_id = playlist_id.clone();
            async move {
            match with_spotify_auth_retry(&backend, |spotify| {
                let playlist_id = playlist_id.clone();
                async move { spotify.unfollow_playlist(&playlist_id).await }
            })
            .await
            {
                Ok(()) => {
                    let _ = proxy.emit(PlaylistsAppEvent::PlaylistDeleted(playlist_id));
                    let _ = proxy.emit(SystemAppEvent::StatusMessage("Playlist removed.".to_string()));
                    refresh_user_playlists_inner(backend, &mut proxy).await;
                }
                Err(err) => {
                    let _ = proxy.emit(PlaylistsAppEvent::PlaylistDeleteFailed(err.clone()));
                    let _ = proxy.emit(SystemAppEvent::Error(err));
                }
            }

            Ok::<(), String>(())
            }
        })
        .name(task_name)
        .on_result(|result, proxy| {
            if let TaskResult::Error(err) = result {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }),
    );
}

pub fn fetch_playlist_tracks(
    backend: SharedBackend,
    playlist_id: String,
    playlist_name: String,
    request_id: u64,
    cx: &EventContext<'_>,
) -> TaskHandle {
    let proxy = cx.get_proxy();
    let task_name = ("fetch-playlist-tracks", playlist_id.clone());
    cx.add_task(
        Task::new(move |_| {
            let mut proxy = proxy.clone();
            let backend = backend.clone();
            let playlist_id = playlist_id.clone();
            let playlist_name = playlist_name.clone();
            async move {
            fetch_playlist_tracks_inner(backend, playlist_id, playlist_name, request_id, &mut proxy)
                .await;
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

pub fn add_track_to_playlist(
    backend: SharedBackend,
    track_id: String,
    playlist_id: String,
    cx: &EventContext<'_>,
) {
    let proxy = cx.get_proxy();
    let task_name = ("add-track-to-playlist", playlist_id.clone(), track_id.clone());
    cx.add_task(
        Task::new(move |_| {
            let mut proxy = proxy.clone();
            let backend = backend.clone();
            let track_id = track_id.clone();
            let playlist_id = playlist_id.clone();
            async move {
            match with_spotify_auth_retry(&backend, |spotify| {
                let track_id = track_id.clone();
                let playlist_id = playlist_id.clone();
                async move {
                    let track_uri = format!("spotify:track:{}", track_id);
                    spotify.add_tracks_to_playlist(&playlist_id, vec![track_uri]).await
                }
            })
            .await
            {
                Ok(()) => {
                    let _ =
                        proxy.emit(SystemAppEvent::StatusMessage("Track added to playlist.".to_string()));
                }
                Err(err) => {
                    let message = if err.contains("status 403") {
                        let lowered = err.to_ascii_lowercase();
                        if lowered.contains("insufficient_scope")
                            || lowered.contains("insufficient client scope")
                        {
                            format!(
                                "Spotify denied adding track (403): {err}. Your token is missing playlist modify scopes. Re-login once and approve `playlist-modify-private` and `playlist-modify-public`, then retry."
                            )
                        } else {
                            format!(
                                "Spotify denied adding track (403): {err}. This playlist is likely read-only for your account (for example followed but not owned/collaborative). Choose a playlist you can edit."
                            )
                        }
                    } else {
                        err
                    };
                    let _ = proxy.emit(SystemAppEvent::Error(message));
                }
            }

            Ok::<(), String>(())
            }
        })
        .name(task_name)
        .on_result(|result, proxy| {
            if let TaskResult::Error(err) = result {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }),
    );
}

pub fn remove_track_from_playlist(
    backend: SharedBackend,
    track_id: String,
    playlist_id: String,
    playlist_name: String,
    request_id: u64,
    cx: &EventContext<'_>,
) {
    let proxy = cx.get_proxy();
    let task_name = (
        "remove-track-from-playlist",
        playlist_id.clone(),
        track_id.clone(),
    );
    cx.add_task(
        Task::new(move |_| {
            let mut proxy = proxy.clone();
            let backend = backend.clone();
            let track_id = track_id.clone();
            let playlist_id = playlist_id.clone();
            let playlist_name = playlist_name.clone();
            async move {
            match with_spotify_auth_retry(&backend, |spotify| {
                let track_id = track_id.clone();
                let playlist_id = playlist_id.clone();
                async move {
                    let track_uri = format!("spotify:track:{track_id}");
                    spotify.remove_tracks_from_playlist(&playlist_id, vec![track_uri]).await
                }
            })
            .await
            {
                Ok(()) => {
                    let _ = proxy
                        .emit(SystemAppEvent::StatusMessage("Removed track from playlist.".to_string()));
                    fetch_playlist_tracks_inner(
                        backend,
                        playlist_id,
                        playlist_name,
                        request_id,
                        &mut proxy,
                    )
                    .await;
                }
                Err(err) => {
                    let message = if err.contains("status 403") {
                        format!(
                            "Spotify denied removing track (403): {err}. This playlist may be read-only for your account or missing playlist modify scopes."
                        )
                    } else {
                        err
                    };
                    let _ = proxy.emit(SystemAppEvent::Error(message));
                }
            }

            Ok::<(), String>(())
            }
        })
        .name(task_name)
        .on_result(|result, proxy| {
            if let TaskResult::Error(err) = result {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }),
    );
}
