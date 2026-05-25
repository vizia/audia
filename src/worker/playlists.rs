use vizia::prelude::{ContextProxy, EventContext, Task, TaskHandle, TaskResult};

use crate::messages::PlaylistEntry;
use crate::ui::events::{PlaylistsAppEvent, SystemAppEvent};

use super::{SharedBackend, load_images_parallel, with_spotify_auth_retry};

#[derive(Clone, Debug)]
struct PlaylistCreatedResult {
    id: String,
    name: String,
}

#[derive(Clone, Debug)]
struct PlaylistRenamedResult {
    id: String,
    name: String,
}

#[derive(Clone, Debug)]
struct PlaylistTracksRefreshRequest {
    request_id: u64,
    id: String,
    name: String,
}

#[derive(Clone, Debug)]
struct RefreshUserPlaylistsResult {
    playlists: Vec<PlaylistEntry>,
    artwork_urls: Vec<Option<String>>,
}

async fn refresh_user_playlists_inner(
    backend: SharedBackend,
) -> Result<RefreshUserPlaylistsResult, String> {
    let playlists = with_spotify_auth_retry(&backend, |spotify| async move {
        spotify.list_user_playlists(20).await
    })
    .await?;

    let mut mapped = Vec::with_capacity(playlists.len());
    let mut artwork_urls = Vec::with_capacity(playlists.len());

    for playlist in playlists {
        mapped.push(PlaylistEntry {
            name: playlist.name,
            image_key: None,
            id: playlist.id,
            track_count: playlist.track_count,
            total_duration_ms: 0,
        });
        artwork_urls.push(playlist.image_url);
    }

    Ok(RefreshUserPlaylistsResult {
        playlists: mapped,
        artwork_urls,
    })
}

pub fn hydrate_user_playlist_artwork(
    playlists: Vec<PlaylistEntry>,
    artwork_urls: Vec<Option<String>>,
    cx: &EventContext<'_>,
) -> TaskHandle {
    let proxy = cx.get_proxy();
    cx.add_task(
        Task::new(move |_| {
            let mut proxy = proxy.clone();
            let mut playlists = playlists.clone();
            let artwork_urls = artwork_urls.clone();
            async move {
                let image_jobs = artwork_urls
                    .iter()
                    .enumerate()
                    .filter_map(|(index, url)| {
                        // Playlist IDs are stable cache keys for artwork.
                        if let Some(url) = url.as_ref() {
                            let key = playlists
                                .get(index)
                                .map(|playlist| format!("playlist-artwork:{}", playlist.id))?;
                            Some((index, key, url.clone()))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                let loaded_images = load_images_parallel(&mut proxy, image_jobs).await;
                for (index, key) in loaded_images {
                    if let Some(playlist) = playlists.get_mut(index) {
                        playlist.image_key = Some(key);
                    }
                }

                Ok::<Vec<PlaylistEntry>, String>(playlists)
            }
        })
        .name("hydrate-user-playlists-artwork")
        .on_result(|result, proxy| match result {
            TaskResult::Completed(playlists) => {
                let _ = proxy.emit(PlaylistsAppEvent::Playlists(playlists));
            }
            TaskResult::Error(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
            TaskResult::Timeout | TaskResult::Cancelled | TaskResult::Disconnected => {}
        }),
    )
}

async fn fetch_playlist_tracks_inner(
    backend: SharedBackend,
    playlist_id: String,
    playlist_name: String,
    request_id: u64,
    proxy: &mut ContextProxy,
) -> Result<(), String> {
    let first_page = with_spotify_auth_retry(&backend, |spotify| {
        let playlist_id = playlist_id.clone();
        async move { spotify.get_playlist_tracks_page(&playlist_id, 50, 0).await }
    })
    .await;

    let (mut tracks, total) = first_page?;

    let mut offset = tracks.len();
    let mut has_more = offset < total;

    let count = tracks.len();
    let total_duration_ms = tracks
        .iter()
        .map(|track| track.duration_ms as u64)
        .sum::<u64>();
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
            track.album_image_url.as_ref().map(|url| {
                (
                    index,
                    format!("playlist-track-artwork:{}", url),
                    url.clone(),
                )
            })
        })
        .collect::<Vec<_>>();

    let first_page_images = load_images_parallel(proxy, first_page_image_jobs).await;
    for (index, key) in first_page_images {
        if let Some(track) = tracks.get_mut(index) {
            track.album_image_key = Some(key);
        }
    }

    let count = tracks.len();
    let total_duration_ms = tracks
        .iter()
        .map(|track| track.duration_ms as u64)
        .sum::<u64>();
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
            async move {
                spotify
                    .get_playlist_tracks_page(&playlist_id, 50, offset)
                    .await
            }
        })
        .await;

        let (mut page_tracks, total) = page_result?;

        let page_size = page_tracks.len();
        let page_start = tracks.len();
        tracks.append(&mut page_tracks);

        let count = tracks.len();
        let total_duration_ms = tracks
            .iter()
            .map(|track| track.duration_ms as u64)
            .sum::<u64>();
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
                track.album_image_url.as_ref().map(|url| {
                    (
                        index,
                        format!("playlist-track-artwork:{}", url),
                        url.clone(),
                    )
                })
            })
            .collect::<Vec<_>>();

        let page_images = load_images_parallel(proxy, page_image_jobs).await;
        for (index, key) in page_images {
            if let Some(track) = tracks.get_mut(index) {
                track.album_image_key = Some(key);
            }
        }

        let count = tracks.len();
        let total_duration_ms = tracks
            .iter()
            .map(|track| track.duration_ms as u64)
            .sum::<u64>();
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
    let total_duration_ms = tracks
        .iter()
        .map(|track| track.duration_ms as u64)
        .sum::<u64>();
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

    Ok(())
}

pub fn refresh_user_playlists(backend: SharedBackend, cx: &EventContext<'_>) {
    cx.add_task(
        Task::new(move |_| {
            let backend = backend.clone();
            async move { refresh_user_playlists_inner(backend).await }
        })
        .name("refresh-user-playlists")
        .on_result(|result, proxy| match result {
            TaskResult::Completed(payload) => {
                let _ = proxy.emit(PlaylistsAppEvent::Playlists(payload.playlists.clone()));
                let _ = proxy.emit(PlaylistsAppEvent::HydrateUserPlaylistsArtwork {
                    playlists: payload.playlists,
                    artwork_urls: payload.artwork_urls,
                });
            }
            TaskResult::Error(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
            TaskResult::Timeout | TaskResult::Cancelled | TaskResult::Disconnected => {}
        }),
    );
}

pub fn create_playlist(backend: SharedBackend, name: String, cx: &EventContext<'_>) {
    let task_name = ("create-playlist", name.clone());
    cx.add_task(
        Task::new(move |_| {
            let backend = backend.clone();
            let name = name.clone();
            async move {
                let trimmed_name = name.trim().to_string();
                let playlist = with_spotify_auth_retry(&backend, |spotify| {
                    let trimmed_name = trimmed_name.clone();
                    async move { spotify.create_playlist(&trimmed_name).await }
                })
                .await?;

                Ok::<PlaylistCreatedResult, String>(PlaylistCreatedResult {
                    id: playlist.id,
                    name: playlist.name,
                })
            }
        })
        .name(task_name)
        .on_result(|result, proxy| match result {
            TaskResult::Completed(playlist) => {
                let _ = proxy.emit(PlaylistsAppEvent::PlaylistCreated {
                    id: playlist.id,
                    name: playlist.name.clone(),
                });
                let _ = proxy.emit(SystemAppEvent::StatusMessage(format!(
                    "Created playlist '{}'.",
                    playlist.name
                )));
                let _ = proxy.emit(PlaylistsAppEvent::RefreshUserPlaylists);
            }
            TaskResult::Error(err) => {
                let _ = proxy.emit(PlaylistsAppEvent::PlaylistCreateFailed(err.clone()));
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
            TaskResult::Timeout | TaskResult::Cancelled | TaskResult::Disconnected => {}
        }),
    );
}

pub fn rename_playlist(
    backend: SharedBackend,
    playlist_id: String,
    name: String,
    cx: &EventContext<'_>,
) {
    let task_name = ("rename-playlist", playlist_id.clone());
    cx.add_task(
        Task::new(move |_| {
            let backend = backend.clone();
            let playlist_id = playlist_id.clone();
            let name = name.clone();
            async move {
                let trimmed_name = name.trim().to_string();
                with_spotify_auth_retry(&backend, |spotify| {
                    let playlist_id = playlist_id.clone();
                    let trimmed_name = trimmed_name.clone();
                    async move { spotify.rename_playlist(&playlist_id, &trimmed_name).await }
                })
                .await?;

                Ok::<PlaylistRenamedResult, String>(PlaylistRenamedResult {
                    id: playlist_id,
                    name: trimmed_name,
                })
            }
        })
        .name(task_name)
        .on_result(|result, proxy| match result {
            TaskResult::Completed(playlist) => {
                let _ = proxy.emit(PlaylistsAppEvent::PlaylistRenamed {
                    id: playlist.id,
                    name: playlist.name.clone(),
                });
                let _ = proxy.emit(SystemAppEvent::StatusMessage(format!(
                    "Renamed playlist to '{}'.",
                    playlist.name
                )));
                let _ = proxy.emit(PlaylistsAppEvent::RefreshUserPlaylists);
            }
            TaskResult::Error(err) => {
                let _ = proxy.emit(PlaylistsAppEvent::PlaylistRenameFailed(err.clone()));
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
            TaskResult::Timeout | TaskResult::Cancelled | TaskResult::Disconnected => {}
        }),
    );
}

pub fn delete_playlist(backend: SharedBackend, playlist_id: String, cx: &EventContext<'_>) {
    let task_name = ("delete-playlist", playlist_id.clone());
    cx.add_task(
        Task::new(move |_| {
            let backend = backend.clone();
            let playlist_id = playlist_id.clone();
            async move {
                with_spotify_auth_retry(&backend, |spotify| {
                    let playlist_id = playlist_id.clone();
                    async move { spotify.unfollow_playlist(&playlist_id).await }
                })
                .await?;

                Ok::<String, String>(playlist_id)
            }
        })
        .name(task_name)
        .on_result(|result, proxy| match result {
            TaskResult::Completed(playlist_id) => {
                let _ = proxy.emit(PlaylistsAppEvent::PlaylistDeleted(playlist_id));
                let _ = proxy.emit(SystemAppEvent::StatusMessage(
                    "Playlist removed.".to_string(),
                ));
                let _ = proxy.emit(PlaylistsAppEvent::RefreshUserPlaylists);
            }
            TaskResult::Error(err) => {
                let _ = proxy.emit(PlaylistsAppEvent::PlaylistDeleteFailed(err.clone()));
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
            TaskResult::Timeout | TaskResult::Cancelled | TaskResult::Disconnected => {}
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
                fetch_playlist_tracks_inner(
                    backend,
                    playlist_id,
                    playlist_name,
                    request_id,
                    &mut proxy,
                )
                .await?;
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

/// Adds a track to a playlist.
pub fn add_track_to_playlist(
    backend: SharedBackend,
    track_id: String,
    playlist_id: String,
    cx: &EventContext<'_>,
) {
    let task_name = (
        "add-track-to-playlist",
        playlist_id.clone(),
        track_id.clone(),
    );
    cx.add_task(
        Task::new(move |_| {
            let backend = backend.clone();
            let track_id = track_id.clone();
            let playlist_id = playlist_id.clone();
            async move {
                with_spotify_auth_retry(&backend, |spotify| {
                    let track_id = track_id.clone();
                    let playlist_id = playlist_id.clone();
                    async move {
                        let track_uri = format!("spotify:track:{}", track_id);
                        spotify
                            .add_tracks_to_playlist(&playlist_id, vec![track_uri])
                            .await
                    }
                })
                .await?;

                Ok::<(), String>(())
            }
        })
        .name(task_name)
        .on_result(|result, proxy| match result {
            TaskResult::Completed(()) => {
                let _ = proxy.emit(SystemAppEvent::StatusMessage(
                    "Track added to playlist.".to_string(),
                ));
            }
            TaskResult::Error(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
            TaskResult::Timeout | TaskResult::Cancelled | TaskResult::Disconnected => {}
        }),
    );
}

/// Removes a track from a playlist, then triggers a refresh of the playlist's tracks to update the UI.
pub fn remove_track_from_playlist(
    backend: SharedBackend,
    track_id: String,
    playlist_id: String,
    playlist_name: String,
    request_id: u64,
    cx: &EventContext<'_>,
) {
    let task_name = (
        "remove-track-from-playlist",
        playlist_id.clone(),
        track_id.clone(),
    );
    cx.add_task(
        Task::new(move |_| {
            let backend = backend.clone();
            let track_id = track_id.clone();
            let playlist_id = playlist_id.clone();
            let playlist_name = playlist_name.clone();
            async move {
                with_spotify_auth_retry(&backend, |spotify| {
                    let track_id = track_id.clone();
                    let playlist_id = playlist_id.clone();
                    async move {
                        let track_uri = format!("spotify:track:{track_id}");
                        spotify
                            .remove_tracks_from_playlist(&playlist_id, vec![track_uri])
                            .await
                    }
                })
                .await?;

                Ok::<PlaylistTracksRefreshRequest, String>(PlaylistTracksRefreshRequest {
                    request_id,
                    id: playlist_id,
                    name: playlist_name,
                })
            }
        })
        .name(task_name)
        .on_result(|result, proxy| match result {
            TaskResult::Completed(refresh_request) => {
                let _ = proxy.emit(SystemAppEvent::StatusMessage(
                    "Removed track from playlist.".to_string(),
                ));
                let _ = proxy.emit(PlaylistsAppEvent::RefreshPlaylistTracks {
                    request_id: refresh_request.request_id,
                    id: refresh_request.id,
                    name: refresh_request.name,
                });
            }
            TaskResult::Error(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
            TaskResult::Timeout | TaskResult::Cancelled | TaskResult::Disconnected => {}
        }),
    );
}
