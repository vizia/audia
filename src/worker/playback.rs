use std::time::Duration;

use vizia::prelude::{Context, EventContext, ImageRetentionPolicy, Task, TaskResult};

use crate::messages::Track;
use crate::ui::events::{PlaybackEvents, SystemEvents};

use super::{SharedBackend, fetch_image_bytes, lock_playback, shared_playback};

pub fn start_playback_progress_poller(backend: SharedBackend, cx: &Context) {
    let proxy = cx.get_proxy();
    cx.add_task(
        Task::new(move |cancellation| {
            let mut proxy = proxy.clone();
            let backend = backend.clone();
            async move {
                let mut poll_interval = tokio::time::interval(Duration::from_millis(500));

                loop {
                    if cancellation.is_cancelled() {
                        return Ok::<(), String>(());
                    }

                    poll_interval.tick().await;

                    let playback = {
                        let playback = match shared_playback(&backend) {
                            Ok(playback) => playback,
                            Err(err) => {
                                let _ = proxy.emit(SystemEvents::Error(err));
                                return Ok::<(), String>(());
                            }
                        };
                        let state = match lock_playback(&playback) {
                            Ok(state) => state,
                            Err(err) => {
                                let _ = proxy.emit(SystemEvents::Error(err));
                                return Ok::<(), String>(());
                            }
                        };
                        state.local_handle()
                    };

                    let local_track_ended = playback.consume_track_finished()
                        || playback.mark_track_finished_if_stalled();

                    if local_track_ended {
                        let _ = proxy.emit(PlaybackEvents::LocalTrackEnded);
                    }

                    let local_progress = playback.playback_progress();

                    if let Some((position_ms, duration_ms, is_playing)) = local_progress {
                        let _ = proxy.emit(PlaybackEvents::Progress {
                            position_ms,
                            duration_ms,
                            is_playing,
                        });
                    }
                }
            }
        })
        .name("playback-progress-poller")
        .on_result(|result, proxy| {
            if let TaskResult::Error(err) = result {
                let _ = proxy.emit(SystemEvents::Error(err));
            }
        }),
    );
}

pub fn load_playback_artwork(image_url: Option<String>, cx: &EventContext<'_>) {
    cx.add_task(
        Task::new(move |_| {
            let image_url = image_url.clone();
            async move {
                let Some(url) = image_url else {
                    return Ok::<Option<(String, Vec<u8>)>, String>(None);
                };

                let image_bytes = fetch_image_bytes(url.clone())
                    .await
                    .ok_or_else(|| format!("Failed to fetch playback artwork from {url}"))?;

                let key = format!("playback-artwork:{}", url);

                Ok::<Option<(String, Vec<u8>)>, String>(Some((key, image_bytes)))
            }
        })
        .name("load-playback-artwork")
        .on_result(|result, proxy| match result {
            TaskResult::Completed(Some((key, image_bytes))) => {
                match proxy.load_image(key.clone(), &image_bytes, ImageRetentionPolicy::Forever) {
                    Ok(()) => {
                        let _ = proxy.emit(PlaybackEvents::ArtworkLoaded {
                            image_key: Some(key),
                        });
                    }
                    Err(err) => {
                        let _ = proxy.emit(SystemEvents::Error(format!(
                            "Failed to load playback artwork image: {err}"
                        )));
                    }
                }
            }
            TaskResult::Completed(None) => {}
            TaskResult::Error(err) => {
                let _ = proxy.emit(SystemEvents::Error(err));
            }
            TaskResult::Timeout | TaskResult::Cancelled | TaskResult::Disconnected { .. } => {}
        }),
    );
}

pub fn playback_play_local_track(backend: SharedBackend, track: Track, cx: &EventContext<'_>) {
    let proxy = cx.get_proxy();
    cx.add_task(
        Task::new(move |_| {
            let mut proxy = proxy.clone();
            let backend = backend.clone();
            let track = track.clone();
            async move {
                let result = tokio::task::spawn_blocking(move || {
                    let playback = {
                        let playback = shared_playback(&backend)?;
                        let state = lock_playback(&playback)?;
                        state.local_handle()
                    };
                    playback.play_track(&track)
                })
                .await;

                match result {
                    Ok(Ok(())) => {
                        let _ = proxy.emit(SystemEvents::StatusMessage(
                            "Local track playback started.".to_string(),
                        ));
                    }
                    Ok(Err(err)) => {
                        let _ = proxy.emit(SystemEvents::Error(err));
                    }
                    Err(err) => {
                        let _ = proxy.emit(SystemEvents::Error(format!(
                            "Playback task failed: {err}"
                        )));
                    }
                }

                Ok::<(), String>(())
            }
        })
        .name("playback-play-local-track")
        .on_result(|result, proxy| {
            if let TaskResult::Error(err) = result {
                let _ = proxy.emit(SystemEvents::Error(err));
            }
        }),
    );
}

pub fn playback_pause_local(backend: SharedBackend, cx: &EventContext<'_>) {
    let proxy = cx.get_proxy();
    cx.add_task(
        Task::new(move |_| {
            let mut proxy = proxy.clone();
            let backend = backend.clone();
            async move {
                let result = tokio::task::spawn_blocking(move || {
                    let playback = {
                        let playback = shared_playback(&backend)?;
                        let state = lock_playback(&playback)?;
                        state.local_handle()
                    };
                    playback.pause()
                })
                .await;

                match result {
                    Ok(Ok(())) => {
                        let _ = proxy.emit(SystemEvents::StatusMessage(
                            "Local playback paused.".to_string(),
                        ));
                    }
                    Ok(Err(err)) => {
                        let _ = proxy.emit(SystemEvents::Error(err));
                    }
                    Err(err) => {
                        let _ = proxy.emit(SystemEvents::Error(format!(
                            "Playback task failed: {err}"
                        )));
                    }
                }

                Ok::<(), String>(())
            }
        })
        .name("playback-pause-local")
        .on_result(|result, proxy| {
            if let TaskResult::Error(err) = result {
                let _ = proxy.emit(SystemEvents::Error(err));
            }
        }),
    );
}

pub fn playback_resume_local(backend: SharedBackend, cx: &EventContext<'_>) {
    let proxy = cx.get_proxy();
    cx.add_task(
        Task::new(move |_| {
            let mut proxy = proxy.clone();
            let backend = backend.clone();
            async move {
                let result = tokio::task::spawn_blocking(move || {
                    let playback = {
                        let playback = shared_playback(&backend)?;
                        let state = lock_playback(&playback)?;
                        state.local_handle()
                    };
                    playback.resume()
                })
                .await;

                match result {
                    Ok(Ok(())) => {
                        let _ = proxy.emit(SystemEvents::StatusMessage(
                            "Local playback resumed.".to_string(),
                        ));
                    }
                    Ok(Err(err)) => {
                        let _ = proxy.emit(SystemEvents::Error(err));
                    }
                    Err(err) => {
                        let _ = proxy.emit(SystemEvents::Error(format!(
                            "Playback task failed: {err}"
                        )));
                    }
                }

                Ok::<(), String>(())
            }
        })
        .name("playback-resume-local")
        .on_result(|result, proxy| {
            if let TaskResult::Error(err) = result {
                let _ = proxy.emit(SystemEvents::Error(err));
            }
        }),
    );
}

pub fn playback_stop_local(backend: SharedBackend, cx: &EventContext<'_>) {
    let mut proxy = cx.get_proxy();
    let playback = match shared_playback(&backend) {
        Ok(playback) => playback,
        Err(err) => {
            let _ = proxy.emit(SystemEvents::Error(err));
            return;
        }
    };
    let playback = match lock_playback(&playback) {
        Ok(state) => state.local_handle(),
        Err(err) => {
            let _ = proxy.emit(SystemEvents::Error(err));
            return;
        }
    };
    let _ = playback.stop();
}

pub fn playback_seek_local(backend: SharedBackend, position_ms: u32, cx: &EventContext<'_>) {
    let proxy = cx.get_proxy();
    cx.add_task(
        Task::new(move |_| {
            let mut proxy = proxy.clone();
            let backend = backend.clone();
            async move {
                let result = tokio::task::spawn_blocking(move || {
                    let playback = {
                        let playback = shared_playback(&backend)?;
                        let state = lock_playback(&playback)?;
                        state.local_handle()
                    };
                    playback.seek_to(position_ms)
                })
                .await;

                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(err)) => {
                        let _ = proxy.emit(SystemEvents::Error(err));
                    }
                    Err(err) => {
                        let _ = proxy.emit(SystemEvents::Error(format!(
                            "Playback task failed: {err}"
                        )));
                    }
                }

                Ok::<(), String>(())
            }
        })
        .name("playback-seek-local")
        .on_result(|result, proxy| {
            if let TaskResult::Error(err) = result {
                let _ = proxy.emit(SystemEvents::Error(err));
            }
        }),
    );
}
