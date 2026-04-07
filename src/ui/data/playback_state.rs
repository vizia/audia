use vizia::prelude::*;

use crate::{
    messages::{PlaybackDevice, Track},
    ui::{
        events::{PlaybackAppEvent, PlaybackProgressSource, PlaybackUiEvent},
        model_data::PlaybackTarget,
    },
    worker::{self, SharedBackend},
};
use rand::seq::SliceRandom;

pub struct PlaybackState {
    pub backend: SharedBackend,
    pub status: Signal<String>,
    pub playback_devices: Signal<Vec<PlaybackDevice>>,
    pub playback_device_options: Signal<Vec<String>>,
    pub selected_playback_device_index: Signal<Option<usize>>,
    pub playback_ready: Signal<bool>,
    pub playback_is_playing: Signal<bool>,
    pub queue_tracks: Signal<Vec<Track>>,
    pub playback_scrub_percent: Signal<f32>,
    pub queue_current_index: Signal<Option<usize>>,
    pub selected_playback_target: Signal<Option<PlaybackTarget>>,
    pub playback_volume: Signal<f32>,
    pub recently_played: Signal<Vec<Track>>,

    pub playback_duration_ms: Signal<u32>,
    pub playback_track_name: Signal<String>,
    pub playback_track_artist: Signal<String>,
    pub playback_track_image_key: Signal<Option<String>>,
    pub playback_overlay_image_key: Signal<Option<String>>,
    pub last_remote_volume_sent: Option<u8>,
    pub last_remote_volume_sent_at: Option<std::time::Instant>,
    pub last_remote_seek_sent_ms: Option<u32>,
    pub last_remote_seek_sent_at: Option<std::time::Instant>,
    pub last_scrub_user_input_at: Option<std::time::Instant>,
    pub artwork_fade_animation: Animation,
}

impl PlaybackState {
    pub(crate) fn local_playback_option_label(&self) -> String {
        if self.playback_ready.get() {
            "Local Device [ready]".to_string()
        } else {
            "Local Device [unavailable]".to_string()
        }
    }

    pub(crate) fn refresh_playback_device_selection(&mut self) {
        let devices = self.playback_devices.get();
        let mut options = vec![self.local_playback_option_label()];
        options.extend(devices.iter().map(|device| {
            let activity = if device.is_active { "active" } else { "idle" };
            if device.id.is_some() {
                format!("{} [{}]", device.name, activity)
            } else {
                format!("{} [{}] (unavailable)", device.name, activity)
            }
        }));
        self.playback_device_options.set(options);

        match self.selected_playback_target.get() {
            Some(PlaybackTarget::Local) => {
                self.selected_playback_device_index.set(Some(0));
                return;
            }
            Some(PlaybackTarget::Remote(device_id)) => {
                if let Some((index, _)) = devices
                    .iter()
                    .enumerate()
                    .find(|(_, device)| device.id.as_deref() == Some(device_id.as_str()))
                {
                    self.selected_playback_device_index.set(Some(index + 1));
                    return;
                }
            }
            None => {}
        }

        if let Some((index, active)) = devices
            .iter()
            .enumerate()
            .find(|(_, device)| device.is_active && device.id.is_some())
        {
            if let Some(device_id) = active.id.clone() {
                self.selected_playback_target
                    .set(Some(PlaybackTarget::Remote(device_id)));
                self.selected_playback_device_index.set(Some(index + 1));
                return;
            }
        }

        self.selected_playback_target
            .set(Some(PlaybackTarget::Local));
        self.selected_playback_device_index.set(Some(0));
    }
}

impl Model for PlaybackState {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|playback_event, _: &mut _| match playback_event {
            PlaybackUiEvent::ClearQueue => {
                self.queue_tracks.update(|queue| queue.clear());
                self.queue_current_index.set(None);
                self.playback_duration_ms.set(0);
                self.playback_scrub_percent.set(0.0);
                self.playback_track_name.set("".to_string());
                self.playback_track_artist.set("".to_string());
                self.playback_track_image_key.set(None);
                self.playback_overlay_image_key.set(None);
                cx.emit(PlaybackUiEvent::Stop);
            }
            PlaybackUiEvent::ClearRecentlyPlayed => {
                self.recently_played.update(|list| list.clear());
            }
            PlaybackUiEvent::ShuffleQueue => {
                let mut tracks = self.queue_tracks.get();
                if tracks.len() < 2 {
                    self.status
                        .set("Need at least two tracks in queue to shuffle.".to_string());
                    return;
                }

                let next_current_index = if let Some(current_index) = self.queue_current_index.get()
                {
                    let pinned_index = current_index.min(tracks.len().saturating_sub(1));
                    let current_track = tracks.remove(pinned_index);
                    tracks.shuffle(&mut rand::thread_rng());
                    tracks.insert(0, current_track);
                    Some(0)
                } else {
                    tracks.shuffle(&mut rand::thread_rng());
                    None
                };

                self.queue_tracks.set(tracks);
                self.queue_current_index.set(next_current_index);
                self.status.set("Queue shuffled.".to_string());
            }
            PlaybackUiEvent::AddToQueue(tracks) => {
                self.queue_tracks
                    .update(|queue| queue.extend(tracks.clone()));
                if self.queue_current_index.get().is_none() {
                    cx.emit(PlaybackUiEvent::Play);
                }
                self.status
                    .set(format!("Added {} tracks to the queue.", tracks.len()));
            }
            PlaybackUiEvent::RefreshDevices => {
                self.status
                    .set("Refreshing playback devices...".to_string());
                worker::refresh_playback_devices(self.backend.clone(), cx.get_proxy());
            }
            PlaybackUiEvent::SelectPlaybackDevice(index) => {
                if *index == 0 {
                    self.selected_playback_target
                        .set(Some(PlaybackTarget::Local));
                    self.selected_playback_device_index.set(Some(0));

                    if self.playback_ready.get() {
                        self.status
                            .set("Playback target set to Local Device.".to_string());
                    } else {
                        self.status
                            .set("Local Device is currently unavailable.".to_string());
                    }
                    return;
                }

                let devices = self.playback_devices.get();
                let Some(device) = devices.get(index.saturating_sub(1)).cloned() else {
                    self.status
                        .set("Selected playback device is unavailable.".to_string());
                    return;
                };

                let Some(device_id) = device.id else {
                    self.status
                        .set("Selected Spotify device is unavailable.".to_string());
                    return;
                };

                self.selected_playback_target
                    .set(Some(PlaybackTarget::Remote(device_id.clone())));
                self.selected_playback_device_index.set(Some(*index));
                self.status
                    .set(format!("Playback target set to '{}'.", device.name));

                worker::playback_transfer_device(self.backend.clone(), device_id, cx.get_proxy());
            }

            PlaybackUiEvent::SelectQueueTrack(index) => {
                // The current track (index 0) is being skipped — add it to recently played.
                // Tracks between 1 and index-1 were never played; just discard them.
                if *index > 0 {
                    let current = self.queue_tracks.with(|queue| queue.first().cloned());
                    if let Some(track) = current {
                        self.recently_played.update(|list| list.push(track));
                    }
                    self.queue_tracks.update(|queue| {
                        queue.drain(0..*index);
                    });
                }

                let selected_track = self.queue_tracks.with(|queue| queue[0].clone());

                self.queue_current_index.set(Some(0));
                self.playback_duration_ms.set(selected_track.duration_ms);
                self.playback_scrub_percent.set(0.0);
                self.playback_track_name.set(selected_track.name.clone());
                self.playback_track_artist
                    .set(selected_track.artist.clone());
                worker::load_playback_artwork(
                    self.backend.clone(),
                    selected_track.album_image_url.clone(),
                    cx.get_proxy(),
                );

                match self.selected_playback_target.get() {
                    Some(PlaybackTarget::Local) => {
                        self.status
                            .set("Playing selected queue song on local device...".to_string());
                        worker::playback_play_local_track(
                            self.backend.clone(),
                            selected_track,
                            cx.get_proxy(),
                        );
                        self.playback_is_playing.set(true);
                    }
                    Some(PlaybackTarget::Remote(_)) => {
                        self.status.set(format!(
                            "Playing queued song '{}' on selected Spotify device...",
                            selected_track.name
                        ));
                        worker::playback_play_selected_track(
                            self.backend.clone(),
                            selected_track.id.clone(),
                            cx.get_proxy(),
                        );
                        self.playback_is_playing.set(true);
                    }
                    None => {
                        self.status.set("No device selected.".to_string());
                    }
                }
            }
            PlaybackUiEvent::Previous => match self.selected_playback_target.get() {
                Some(PlaybackTarget::Local) => {
                    // If more than 3 seconds into the current track, restart it.
                    let position_ms = {
                        let pct = self.playback_scrub_percent.get();
                        let dur = self.playback_duration_ms.get();
                        (pct / 100.0 * dur as f32).round() as u32
                    };

                    if position_ms > 3000 {
                        // Restart the current track.
                        self.status.set("Restarting current track...".to_string());
                        cx.emit(PlaybackUiEvent::Play);
                        return;
                    }

                    // At (or near) the start — try to restore from recently played.
                    let prev_track = self.recently_played.with(|list| list.last().cloned());
                    if let Some(track) = prev_track {
                        self.recently_played.update(|list| {
                            list.pop();
                        });
                        self.queue_tracks.update(|queue| {
                            queue.insert(0, track);
                        });
                        self.queue_current_index.set(Some(0));
                        self.status
                            .set("Playing previous track from recently played...".to_string());
                        cx.emit(PlaybackUiEvent::Play);
                    } else if self.queue_tracks.with(|queue| !queue.is_empty()) {
                        // Nothing in recently played — just restart.
                        self.status
                            .set("Replaying from start of queue...".to_string());
                        cx.emit(PlaybackUiEvent::Play);
                    } else {
                        self.status.set("Nothing to go back to.".to_string());
                    }
                }
                Some(PlaybackTarget::Remote(_)) => {
                    self.status.set("Sending previous command...".to_string());
                    worker::playback_previous(self.backend.clone(), cx.get_proxy());
                }
                None => {
                    self.status.set("No device selected.".to_string());
                }
            },
            PlaybackUiEvent::Toggle => {
                if self.playback_is_playing.get() {
                    cx.emit(PlaybackUiEvent::Pause);
                } else {
                    match self.selected_playback_target.get() {
                        Some(PlaybackTarget::Local) => {
                            if self.playback_scrub_percent.get() > 0.0 {
                                cx.emit(PlaybackUiEvent::Resume);
                            } else {
                                cx.emit(PlaybackUiEvent::Play);
                            }
                        }
                        Some(PlaybackTarget::Remote(_)) => {
                            cx.emit(PlaybackUiEvent::Resume);
                        }
                        _ => {
                            cx.emit(PlaybackUiEvent::Play);
                        }
                    }
                }
            }
            PlaybackUiEvent::Stop => {
                self.playback_is_playing.set(false);
                self.playback_scrub_percent.set(0.0);
                self.playback_track_name.set("".to_string());
                worker::playback_stop(self.backend.clone(), cx.get_proxy());
            }
            PlaybackUiEvent::Resume => match self.selected_playback_target.get() {
                Some(PlaybackTarget::Local) => {
                    self.status
                        .set("Resuming playback on local device...".to_string());
                    worker::playback_resume_local(self.backend.clone(), cx.get_proxy());
                    self.playback_is_playing.set(true);
                }
                Some(PlaybackTarget::Remote(_)) => {
                    self.status.set("Resuming playback...".to_string());
                    worker::playback_resume_remote(self.backend.clone(), cx.get_proxy());
                    self.playback_is_playing.set(true);
                }
                None => {
                    self.status.set("No device selected.".to_string());
                }
            },
            PlaybackUiEvent::Play => match self.selected_playback_target.get() {
                Some(PlaybackTarget::Local) => {
                    let queue_length = self.queue_tracks.with(|queue| queue.len());
                    if queue_length > 0 {
                        let start_index = self
                            .queue_current_index
                            .get()
                            .unwrap_or(0)
                            .min(queue_length.saturating_sub(1));
                        let track = self.queue_tracks.with(|queue| queue[start_index].clone());
                        self.queue_current_index.set(Some(start_index));

                        self.playback_duration_ms.set(track.duration_ms);
                        self.playback_scrub_percent.set(0.0);
                        self.playback_track_name.set(track.name.clone());
                        self.playback_track_artist.set(track.artist.clone());
                        worker::load_playback_artwork(
                            self.backend.clone(),
                            track.album_image_url.clone(),
                            cx.get_proxy(),
                        );

                        self.status
                            .set("Starting playback from queue on local device...".to_string());

                        worker::playback_play_local_track(
                            self.backend.clone(),
                            track,
                            cx.get_proxy(),
                        );
                        self.playback_is_playing.set(true);
                        return;
                    }

                    // let idx = self.selected_index.get();
                    // let search_results = self.search_result_rows.get();
                    // if idx >= search_results.len() {
                    //     self.status.set(
                    //         "Search for tracks first, then play on the local device.".to_string(),
                    //     );
                    //     return;
                    // }

                    // let track = search_results[idx].clone();
                    // let track_id = track.id.clone();
                    // self.playback_duration_ms.set(track.duration_ms);
                    // self.playback_scrub_percent.set(0.0);
                    // self.playback_track_name.set(track.name.clone());
                    // self.playback_track_artist.set(track.artist.clone());
                    // worker::load_playback_artwork(
                    //     self.backend.clone(),
                    //     track.album_image_url.clone(),
                    //     cx.get_proxy(),
                    // );
                    // self.status
                    //     .set("Fetching recommendations and starting playback...".to_string());
                    // worker::playback_play_local_with_recommendations(
                    //     self.backend.clone(),
                    //     track_id,
                    //     track,
                    //     cx.get_proxy(),
                    // );
                    // self.playback_is_playing.set(true);
                }
                Some(PlaybackTarget::Remote(_)) => {
                    let queued_tracks = self.queue_tracks.get();
                    if !queued_tracks.is_empty() {
                        let start_index = self
                            .queue_current_index
                            .get()
                            .unwrap_or(0)
                            .min(queued_tracks.len().saturating_sub(1));
                        let track = queued_tracks[start_index].clone();
                        self.queue_current_index.set(Some(start_index));

                        self.playback_duration_ms.set(track.duration_ms);
                        self.playback_scrub_percent.set(0.0);
                        self.playback_track_name.set(track.name.clone());
                        self.playback_track_artist.set(track.artist.clone());
                        worker::load_playback_artwork(
                            self.backend.clone(),
                            track.album_image_url.clone(),
                            cx.get_proxy(),
                        );
                        self.status.set(format!(
                            "Playing queued track '{}' on selected Spotify device...",
                            track.name
                        ));
                        worker::playback_play_selected_track(
                            self.backend.clone(),
                            track.id,
                            cx.get_proxy(),
                        );
                        self.playback_is_playing.set(true);
                        return;
                    }

                    // let idx = self.selected_index.get();
                    // let search_results = self.search_result_rows.get();
                    // if idx >= search_results.len() {
                    //     self.status.set("No track selected to play.".to_string());
                    //     return;
                    // }

                    // let track = &search_results[idx];
                    // let track_id = track.id.clone();
                    // self.playback_duration_ms.set(track.duration_ms);
                    // self.playback_scrub_percent.set(0.0);
                    // self.playback_track_name.set(track.name.clone());
                    // self.playback_track_artist.set(track.artist.clone());
                    // worker::load_playback_artwork(
                    //     self.backend.clone(),
                    //     track.album_image_url.clone(),
                    //     cx.get_proxy(),
                    // );
                    // self.status.set(format!(
                    //     "Playing '{}' on selected Spotify device...",
                    //     track.name
                    // ));
                    // worker::playback_play_selected_track(
                    //     self.backend.clone(),
                    //     track_id,
                    //     cx.get_proxy(),
                    // );
                    // self.playback_is_playing.set(true);
                }
                None => {
                    self.status.set("No device selected.".to_string());
                }
            },
            PlaybackUiEvent::Pause => match self.selected_playback_target.get() {
                Some(PlaybackTarget::Local) => {
                    self.status
                        .set("Sending pause command to local device...".to_string());
                    worker::playback_pause_local(self.backend.clone(), cx.get_proxy());
                    self.playback_is_playing.set(false);
                }
                _ => {
                    self.status.set("Sending pause command...".to_string());
                    worker::playback_pause(self.backend.clone(), cx.get_proxy());
                    self.playback_is_playing.set(false);
                }
            },
            PlaybackUiEvent::Next => match self.selected_playback_target.get() {
                Some(PlaybackTarget::Local) => {
                    // Remove the current front track and add it to recently played (manual skip).
                    let skipped = self.queue_tracks.with(|queue| queue.first().cloned());
                    if let Some(track) = skipped {
                        self.recently_played.update(|list| list.push(track));
                        self.queue_tracks.update(|queue| {
                            queue.remove(0);
                        });
                    }

                    let queue_length = self.queue_tracks.with(|queue| queue.len());
                    if queue_length > 0 {
                        self.queue_current_index.set(Some(0));
                        cx.emit(PlaybackUiEvent::Play);
                    } else {
                        self.queue_current_index.set(None);
                        self.playback_is_playing.set(false);
                        self.status.set("Reached end of queue.".to_string());
                    }
                    return;

                    // let queued_tracks = self.queue_tracks.get();
                    // if !queued_tracks.is_empty() {
                    //     let Some(current_index) = self.queue_current_index.get() else {
                    //         self.status.set(
                    //             "Queue is loaded. Press Play to start from the first song."
                    //                 .to_string(),
                    //         );
                    //         return;
                    //     };

                    //     let next_index = current_index + 1;
                    //     if next_index >= queued_tracks.len() {
                    //         self.status
                    //             .set("Already at the end of the queue.".to_string());
                    //         return;
                    //     }

                    //     let track = queued_tracks[next_index].clone();

                    //     self.queue_tracks.set(queued_tracks.clone());
                    //     self.queue_current_index.set(Some(next_index));

                    //     self.playback_duration_ms.set(track.duration_ms);
                    //     self.playback_scrub_percent.set(0.0);
                    //     self.playback_track_name.set(track.name.clone());
                    //     self.playback_track_artist.set(track.artist.clone());
                    //     worker::load_playback_artwork(
                    //         self.backend.clone(),
                    //         track.album_image_url.clone(),
                    //         cx.get_proxy(),
                    //     );

                    //     let queue_ids = queued_tracks
                    //         .iter()
                    //         .map(|t| t.id.clone())
                    //         .collect::<Vec<_>>();
                    //     self.status
                    //         .set("Playing next song from queue on local device...".to_string());
                    //     worker::playback_play_local_queue(
                    //         self.backend.clone(),
                    //         queue_ids,
                    //         queued_tracks,
                    //         next_index,
                    //         cx.get_proxy(),
                    //     );
                    //     self.playback_is_playing.set(true);
                    // } else {
                    //     self.status
                    //         .set("Sending next command to local device...".to_string());
                    //     worker::playback_next_local(self.backend.clone(), cx.get_proxy());
                    // }
                }
                Some(PlaybackTarget::Remote(_)) => {
                    let queued_tracks = self.queue_tracks.get();
                    if !queued_tracks.is_empty() {
                        let Some(current_index) = self.queue_current_index.get() else {
                            self.status.set(
                                "Queue is loaded. Press Play to start from the first song."
                                    .to_string(),
                            );
                            return;
                        };

                        let next_index = current_index + 1;
                        if next_index >= queued_tracks.len() {
                            self.status
                                .set("Already at the end of the queue.".to_string());
                            return;
                        }

                        let track = queued_tracks[next_index].clone();
                        self.queue_tracks.set(queued_tracks);
                        self.queue_current_index.set(Some(next_index));
                        self.playback_duration_ms.set(track.duration_ms);
                        self.playback_scrub_percent.set(0.0);
                        self.playback_track_name.set(track.name.clone());
                        self.playback_track_artist.set(track.artist.clone());
                        worker::load_playback_artwork(
                            self.backend.clone(),
                            track.album_image_url.clone(),
                            cx.get_proxy(),
                        );

                        self.status.set(format!(
                            "Playing next queued song '{}' on selected Spotify device...",
                            track.name
                        ));
                        worker::playback_play_selected_track(
                            self.backend.clone(),
                            track.id,
                            cx.get_proxy(),
                        );
                        self.playback_is_playing.set(true);
                    } else {
                        self.status.set("Sending next command...".to_string());
                        worker::playback_next(self.backend.clone(), cx.get_proxy());
                    }
                }
                None => {
                    self.status.set("No device selected.".to_string());
                }
            },
            PlaybackUiEvent::SetVolume(value) => {
                let clamped = value.clamp(0.0, 100.0);
                self.playback_volume.set(clamped);

                match self.selected_playback_target.get() {
                    Some(PlaybackTarget::Remote(_)) => {
                        let target = clamped.round() as u8;
                        let now = Instant::now();
                        let should_send = match (
                            self.last_remote_volume_sent,
                            self.last_remote_volume_sent_at,
                        ) {
                            (Some(last), Some(last_at)) => {
                                target != last
                                    && (now.duration_since(last_at) >= Duration::from_millis(80)
                                        || target.abs_diff(last) >= 3)
                            }
                            _ => true,
                        };

                        if should_send {
                            self.last_remote_volume_sent = Some(target);
                            self.last_remote_volume_sent_at = Some(now);
                            worker::playback_set_volume(
                                self.backend.clone(),
                                target,
                                cx.get_proxy(),
                            );
                        }
                    }
                    Some(PlaybackTarget::Local) => {
                        let target = clamped.round() as u8;
                        let result = {
                            let state = self.backend.lock().unwrap();
                            state.playback.set_volume_percent(target)
                        };

                        if let Err(err) = result {
                            self.status
                                .set(format!("Failed to set local volume: {err}"));
                        }
                    }
                    None => {
                        let message = "No device selected.".to_string();
                        if self.status.get() != message {
                            self.status.set(message);
                        }
                    }
                }
            }
            PlaybackUiEvent::SetScrub(value) => {
                let clamped = value.clamp(0.0, 100.0);
                self.playback_scrub_percent.set(clamped);
                self.last_scrub_user_input_at = Some(Instant::now());

                let duration = self.playback_duration_ms.get();
                if duration == 0 {
                    return;
                }

                let target_ms = ((duration as f32) * (clamped / 100.0)).round() as u32;

                match self.selected_playback_target.get() {
                    Some(PlaybackTarget::Remote(_)) => {
                        let now = Instant::now();
                        let should_send =
                            match (self.last_remote_seek_sent_ms, self.last_remote_seek_sent_at) {
                                (Some(last_ms), Some(last_at)) => {
                                    now.duration_since(last_at) >= Duration::from_millis(120)
                                        || target_ms.abs_diff(last_ms) >= 1200
                                }
                                _ => true,
                            };

                        if should_send {
                            self.last_remote_seek_sent_ms = Some(target_ms);
                            self.last_remote_seek_sent_at = Some(now);
                            worker::playback_seek(self.backend.clone(), target_ms, cx.get_proxy());
                        }
                    }
                    Some(PlaybackTarget::Local) => {
                        worker::playback_seek_local(
                            self.backend.clone(),
                            target_ms,
                            cx.get_proxy(),
                        );
                    }
                    None => {
                        let message = "No device selected.".to_string();
                        if self.status.get() != message {
                            self.status.set(message);
                        }
                    }
                }
            }
        });

        event.map(|playback_event, _: &mut _| match playback_event {
            PlaybackAppEvent::SessionReady => {
                self.playback_ready.set(true);
                self.status
                    .set("Playback session is ready (OAuth token-based).".to_string());
                self.refresh_playback_device_selection();
            }
            PlaybackAppEvent::Devices(devices) => {
                self.playback_devices.set(devices.clone());
                self.refresh_playback_device_selection();
            }
            PlaybackAppEvent::LocalTrackEnded => {
                if !matches!(
                    self.selected_playback_target.get(),
                    Some(PlaybackTarget::Local)
                ) {
                    return;
                }

                // Remove the finished track from the front of the queue and record it.
                let finished_track = self.queue_tracks.with(|queue| queue.first().cloned());
                if let Some(track) = finished_track {
                    self.queue_tracks.update(|queue| {
                        queue.remove(0);
                    });
                    self.recently_played.update(|list| list.push(track));
                }

                let queue_length = self.queue_tracks.with(|queue| queue.len());
                if queue_length > 0 {
                    self.queue_current_index.set(Some(0));
                    cx.emit(PlaybackUiEvent::Play);
                } else {
                    self.queue_current_index.set(None);
                    self.playback_is_playing.set(false);
                    self.status.set("Reached end of queue.".to_string());
                }
            }

            PlaybackAppEvent::Progress {
                source,
                position_ms,
                duration_ms,
                is_playing,
            } => {
                let source_matches_target = matches!(
                    (source, self.selected_playback_target.get()),
                    (PlaybackProgressSource::Local, Some(PlaybackTarget::Local))
                        | (
                            PlaybackProgressSource::Remote,
                            Some(PlaybackTarget::Remote(_))
                        )
                );

                if source_matches_target {
                    if self.playback_is_playing.get() != *is_playing {
                        self.playback_is_playing.set(*is_playing);
                    }

                    self.playback_duration_ms.set(*duration_ms);

                    let suppress_for_recent_scrub = self
                        .last_scrub_user_input_at
                        .map(|at| at.elapsed() < Duration::from_millis(700))
                        .unwrap_or(false);

                    if !suppress_for_recent_scrub && *duration_ms > 0 {
                        let ratio = (*position_ms as f32 / *duration_ms as f32).clamp(0.0, 1.0);
                        self.playback_scrub_percent.set(ratio * 100.0);
                    }
                }
            }
            PlaybackAppEvent::ArtworkLoaded { image_key } => {
                let current_image = self.playback_track_image_key.get();
                if current_image != *image_key {
                    self.playback_overlay_image_key.set(current_image);
                    self.playback_track_image_key.set(image_key.clone());
                    cx.play_animation_for(
                        self.artwork_fade_animation,
                        "artwork-overlay",
                        Duration::from_millis(420),
                        Duration::default(),
                    );
                }
            }
        });
    }
}
