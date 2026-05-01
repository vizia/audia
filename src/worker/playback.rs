use std::{sync::Arc, time::Duration};

use vizia::prelude::{ContextProxy, ImageRetentionPolicy};

use crate::messages::Track;
use crate::ui::events::{PlaybackAppEvent, PlaybackProgressSource, SystemAppEvent};

use super::{SharedBackend, fetch_image_bytes, shared_playback, with_spotify_auth_retry};

pub fn start_playback_progress_poller(backend: SharedBackend, proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        state.runtime.clone()
    };

    runtime.spawn(async move {
        let mut proxy = proxy;
        let mut poll_interval = tokio::time::interval(Duration::from_millis(500));

        loop {
            poll_interval.tick().await;

            let playback = {
                let playback = shared_playback(&backend);
                let state = playback
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
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
                    source: PlaybackProgressSource::Local,
                    position_ms,
                    duration_ms,
                    is_playing,
                });
            }

            // Use with_spotify_auth_retry to ensure token is fresh before polling
            let remote_progress = with_spotify_auth_retry(&backend, |spotify| async move {
                spotify.playback_progress().await
            })
            .await;

            if let Ok(Some((position_ms, duration_ms, is_playing))) = remote_progress {
                let _ = proxy.emit(PlaybackAppEvent::Progress {
                    source: PlaybackProgressSource::Remote,
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
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
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

pub fn refresh_playback_devices(backend: SharedBackend, mut proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        match with_spotify_auth_retry(&backend, |spotify| async move {
            spotify.list_playback_devices().await
        })
        .await
        {
            Ok(devices) => {
                let _ = proxy.emit(PlaybackAppEvent::Devices(devices));
                let _ = proxy.emit(SystemAppEvent::StatusMessage(
                    "Playback devices refreshed.".to_string(),
                ));
            }
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }
    });
}

pub fn playback_play_local_track(backend: SharedBackend, track: Track, proxy: ContextProxy) {
    proxy.spawn(move |proxy| {
        let playback = {
            let playback = shared_playback(&backend);
            let state = playback
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
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
            let playback = shared_playback(&backend);
            let state = playback
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
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
            let playback = shared_playback(&backend);
            let state = playback
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
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

pub fn playback_play_selected_track(
    backend: SharedBackend,
    track_id: String,
    mut proxy: ContextProxy,
) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        if let Err(err) = ensure_active_playback_device_async(&backend).await {
            let _ = proxy.emit(SystemAppEvent::Error(err));
            return;
        }

        match with_spotify_auth_retry(&backend, |spotify| {
            let track_id = track_id.clone();
            async move { spotify.playback_play_track(&track_id).await }
        })
        .await
        {
            Ok(()) => {
                let _ = proxy.emit(SystemAppEvent::StatusMessage(
                    "Playing selected track on Spotify device.".to_string(),
                ));
            }
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }
    });
}

pub fn playback_stop(backend: SharedBackend, _proxy: ContextProxy) {
    let playback = {
        let playback = shared_playback(&backend);
        let state = playback
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.local_handle()
    };
    let _ = playback.stop();
}

pub fn playback_pause(backend: SharedBackend, mut proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        playback_action_remote_async(&backend, &mut proxy, "Pause", RemotePlaybackAction::Pause)
            .await;
    });
}

pub fn playback_resume_remote(backend: SharedBackend, mut proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        playback_action_remote_async(&backend, &mut proxy, "Resume", RemotePlaybackAction::Resume)
            .await;
    });
}

pub fn playback_next(backend: SharedBackend, mut proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        playback_action_remote_async(&backend, &mut proxy, "Next", RemotePlaybackAction::Next)
            .await;
    });
}

pub fn playback_set_volume(backend: SharedBackend, volume_percent: u8, mut proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };
    let target_volume = volume_percent.min(100);

    runtime.spawn(async move {
        playback_action_remote_async(
            &backend,
            &mut proxy,
            "Set volume",
            RemotePlaybackAction::SetVolume(target_volume),
        )
        .await;
    });
}

pub fn playback_seek_local(backend: SharedBackend, position_ms: u32, proxy: ContextProxy) {
    proxy.spawn(move |proxy| {
        let playback = {
            let playback = shared_playback(&backend);
            let state = playback
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.local_handle()
        };
        let result = playback.seek_to(position_ms);

        if let Err(err) = result {
            let _ = proxy.emit(SystemAppEvent::Error(err));
        }
    });
}

pub fn playback_seek(backend: SharedBackend, position_ms: u32, mut proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        if let Err(err) = ensure_active_playback_device_async(&backend).await {
            let _ = proxy.emit(SystemAppEvent::Error(err));
            return;
        }

        if let Err(err) = with_spotify_auth_retry(&backend, |spotify| async move {
            spotify.playback_seek(position_ms).await
        })
        .await
        {
            let _ = proxy.emit(SystemAppEvent::Error(err));
        }
    });
}

pub fn playback_transfer_device(
    backend: SharedBackend,
    device_id: String,
    mut proxy: ContextProxy,
) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        match with_spotify_auth_retry(&backend, |spotify| {
            let device_id = device_id.clone();
            async move { spotify.transfer_playback(&device_id).await }
        })
        .await
        {
            Ok(()) => {
                let _ = proxy.emit(SystemAppEvent::StatusMessage(
                    "Playback transferred to selected Spotify device.".to_string(),
                ));
            }
            Err(err) => {
                let _ = proxy.emit(SystemAppEvent::Error(err));
            }
        }
    });
}

pub fn playback_previous(backend: SharedBackend, mut proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        Arc::clone(&state.runtime)
    };

    runtime.spawn(async move {
        playback_action_remote_async(
            &backend,
            &mut proxy,
            "Previous",
            RemotePlaybackAction::Previous,
        )
        .await;
    });
}

async fn playback_action_remote_async(
    backend: &SharedBackend,
    proxy: &mut ContextProxy,
    label: &'static str,
    action: RemotePlaybackAction,
) {
    if let Err(err) = ensure_active_playback_device_async(backend).await {
        let _ = proxy.emit(SystemAppEvent::Error(err));
        return;
    }

    let result = with_spotify_auth_retry(backend, |spotify| async move {
        match action {
            RemotePlaybackAction::Pause => spotify.playback_pause().await,
            RemotePlaybackAction::Resume => spotify.playback_resume().await,
            RemotePlaybackAction::Stop => spotify.playback_stop().await,
            RemotePlaybackAction::Next => spotify.playback_next().await,
            RemotePlaybackAction::Previous => spotify.playback_previous().await,
            RemotePlaybackAction::SetVolume(volume) => spotify.playback_set_volume(volume).await,
        }
    })
    .await;

    match result {
        Ok(()) => {
            let _ = proxy.emit(SystemAppEvent::StatusMessage(format!(
                "Playback {label} command sent."
            )));
        }
        Err(err) => {
            let _ = proxy.emit(SystemAppEvent::Error(err));
        }
    }
}

#[derive(Clone, Copy)]
enum RemotePlaybackAction {
    Pause,
    Resume,
    Stop,
    Next,
    Previous,
    SetVolume(u8),
}

async fn ensure_active_playback_device_async(backend: &SharedBackend) -> Result<(), String> {
    let devices = with_spotify_auth_retry(backend, |spotify| async move {
        spotify.list_playback_devices().await
    })
    .await?;

    if devices.iter().any(|device| device.is_active) {
        return Ok(());
    }

    let device_id = devices
        .iter()
        .find_map(|device| device.id.clone())
        .ok_or_else(|| {
            "No available Spotify playback device. Open Spotify on a device or use local playback."
                .to_string()
        })?;

    with_spotify_auth_retry(backend, |spotify| {
        let device_id = device_id.clone();
        async move { spotify.transfer_playback(&device_id).await }
    })
    .await?;

    Ok(())
}
