use std::time::Duration;

use vizia::prelude::{ContextProxy, ImageRetentionPolicy};

use crate::messages::Track;
use crate::ui::events::{PlaybackAppEvent, SystemAppEvent};

use super::{
    SharedBackend, backend_runtime, fetch_image_bytes, lock_playback, shared_playback,
};

pub fn start_playback_progress_poller(backend: SharedBackend, proxy: ContextProxy) {
    let runtime = match backend_runtime(&backend) {
        Ok(runtime) => runtime,
        Err(err) => {
            let mut proxy = proxy;
            let _ = proxy.emit(SystemAppEvent::Error(err));
            return;
        }
    };

    runtime.spawn(async move {
        let mut proxy = proxy;
        let mut poll_interval = tokio::time::interval(Duration::from_millis(500));

        loop {
            poll_interval.tick().await;

            let playback = {
                let playback = match shared_playback(&backend) {
                    Ok(playback) => playback,
                    Err(err) => {
                        let _ = proxy.emit(SystemAppEvent::Error(err));
                        return;
                    }
                };
                let state = match lock_playback(&playback) {
                    Ok(state) => state,
                    Err(err) => {
                        let _ = proxy.emit(SystemAppEvent::Error(err));
                        return;
                    }
                };
                state.local_handle()
            };

            let local_track_ended =
                playback.consume_track_finished() || playback.mark_track_finished_if_stalled();

            if local_track_ended {
                let _ = proxy.emit(PlaybackAppEvent::LocalTrackEnded);
            }

            let local_progress = playback.playback_progress();

            if let Some((position_ms, duration_ms, is_playing)) = local_progress {
                let _ = proxy.emit(PlaybackAppEvent::Progress {
                    position_ms,
                    duration_ms,
                    is_playing,
                });
            }
        }
    });
}

pub fn load_playback_artwork(
    backend: SharedBackend,
    image_url: Option<String>,
    mut proxy: ContextProxy,
) {
    let runtime = match backend_runtime(&backend) {
        Ok(runtime) => runtime,
        Err(err) => {
            let _ = proxy.emit(SystemAppEvent::Error(err));
            return;
        }
    };

    runtime.spawn(async move {
        let Some(url) = image_url else {
            let _ = proxy.emit(PlaybackAppEvent::ArtworkLoaded { image_key: None });
            return;
        };

        let bytes = fetch_image_bytes(url.clone()).await;

        let image_key = if let Some(image_bytes) = bytes {
            let key = format!("playback-artwork:{}", url);
            let _ = proxy.load_image(key.clone(), &image_bytes, ImageRetentionPolicy::Forever);
            Some(key)
        } else {
            None
        };

        let _ = proxy.emit(PlaybackAppEvent::ArtworkLoaded { image_key });
    });
}

pub fn playback_play_local_track(backend: SharedBackend, track: Track, proxy: ContextProxy) {
    proxy.spawn(move |proxy| {
        let playback = {
            let playback = match shared_playback(&backend) {
                Ok(playback) => playback,
                Err(err) => {
                    let _ = proxy.emit(SystemAppEvent::Error(err));
                    return;
                }
            };
            let state = match lock_playback(&playback) {
                Ok(state) => state,
                Err(err) => {
                    let _ = proxy.emit(SystemAppEvent::Error(err));
                    return;
                }
            };
            state.local_handle()
        };
        let result = playback.play_track(&track);

        match result {
            Ok(()) => {
                let _ = proxy.emit(SystemAppEvent::StatusMessage(
                    "Local track playback started.".to_string(),
                ));
            }
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }
    });
}

pub fn playback_pause_local(backend: SharedBackend, proxy: ContextProxy) {
    proxy.spawn(move |proxy| {
        let playback = {
            let playback = match shared_playback(&backend) {
                Ok(playback) => playback,
                Err(err) => {
                    let _ = proxy.emit(SystemAppEvent::Error(err));
                    return;
                }
            };
            let state = match lock_playback(&playback) {
                Ok(state) => state,
                Err(err) => {
                    let _ = proxy.emit(SystemAppEvent::Error(err));
                    return;
                }
            };
            state.local_handle()
        };
        let result = playback.pause();

        match result {
            Ok(()) => {
                let _ = proxy.emit(SystemAppEvent::StatusMessage(
                    "Local playback paused.".to_string(),
                ));
            }
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }
    });
}

pub fn playback_resume_local(backend: SharedBackend, proxy: ContextProxy) {
    proxy.spawn(move |proxy| {
        let playback = {
            let playback = match shared_playback(&backend) {
                Ok(playback) => playback,
                Err(err) => {
                    let _ = proxy.emit(SystemAppEvent::Error(err));
                    return;
                }
            };
            let state = match lock_playback(&playback) {
                Ok(state) => state,
                Err(err) => {
                    let _ = proxy.emit(SystemAppEvent::Error(err));
                    return;
                }
            };
            state.local_handle()
        };
        let result = playback.resume();

        match result {
            Ok(()) => {
                let _ = proxy.emit(SystemAppEvent::StatusMessage(
                    "Local playback resumed.".to_string(),
                ));
            }
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }
    });
}

pub fn playback_stop_local(backend: SharedBackend, proxy: ContextProxy) {
    let mut proxy = proxy;
    let playback = match shared_playback(&backend) {
        Ok(playback) => playback,
        Err(err) => {
            let _ = proxy.emit(SystemAppEvent::Error(err));
            return;
        }
    };
    let playback = match lock_playback(&playback) {
        Ok(state) => state.local_handle(),
        Err(err) => {
            let _ = proxy.emit(SystemAppEvent::Error(err));
            return;
        }
    };
    let _ = playback.stop();
}

pub fn playback_seek_local(backend: SharedBackend, position_ms: u32, proxy: ContextProxy) {
    proxy.spawn(move |proxy| {
        let playback = {
            let playback = match shared_playback(&backend) {
                Ok(playback) => playback,
                Err(err) => {
                    let _ = proxy.emit(SystemAppEvent::Error(err));
                    return;
                }
            };
            let state = match lock_playback(&playback) {
                Ok(state) => state,
                Err(err) => {
                    let _ = proxy.emit(SystemAppEvent::Error(err));
                    return;
                }
            };
            state.local_handle()
        };
        let result = playback.seek_to(position_ms);

        if let Err(err) = result {
            let _ = proxy.emit(SystemAppEvent::Error(err));
        }
    });
}
