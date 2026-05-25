use vizia::prelude::*;

use crate::{
    messages::{Album, Track},
    playback::DEFAULT_LOCAL_VOLUME_PERCENT,
    storage::{LocalPlaybackSettings, QueueSnapshot},
    ui::{
        events::{CenterPanelEvent, PlaybackEvent},
        model_data::CenterPage,
    },
    worker::{self, SharedBackend, SharedPlayback},
};
use rand::seq::SliceRandom;

#[derive(Clone)]
pub struct PlaybackState {
    pub backend: SharedBackend,
    pub playback: SharedPlayback,
    pub status: Signal<String>,
    pub playback_ready: Signal<bool>,
    pub playback_is_playing: Signal<bool>,
    pub queue_tracks: Signal<Vec<Track>>,
    pub playback_scrub_percent: Signal<f32>,
    pub queue_current_index: Signal<Option<usize>>,
    pub playback_volume: Signal<f32>,
    pub playback_is_muted: Signal<bool>,
    pub pre_mute_volume: f32,
    pub recently_played: Signal<Vec<Track>>,

    pub playback_duration_ms: Signal<u32>,
    pub playback_track_name: Signal<String>,
    pub playback_track_artist: Signal<String>,
    pub playback_track_id: Signal<Option<String>>,
    pub playback_track_image_key: Signal<Option<String>>,
    pub playback_track_image_url: Signal<Option<String>>,
    pub playback_overlay_image_key: Signal<Option<String>>,
    pub search_album_rows: Signal<Vec<Album>>,
    pub album_tracks: Signal<Vec<Track>>,
    pub album_image_key: Signal<Option<String>>,
    pub last_scrub_user_input_at: Option<std::time::Instant>,
    pub last_local_track_end_handled_at: Option<std::time::Instant>,
    pub artwork_fade_animation: Animation,

    // Preferences
    pub autoplay_on_queue_add: Signal<bool>,
    pub restore_queue_on_startup: Signal<bool>,
}

impl PlaybackState {
    fn persisted_local_volume_percent() -> u8 {
        LocalPlaybackSettings::load()
            .ok()
            .flatten()
            .map(|settings| settings.local_volume_percent.min(100))
            .unwrap_or(DEFAULT_LOCAL_VOLUME_PERCENT)
    }

    pub fn new(
        backend: SharedBackend,
        playback: SharedPlayback,
        status: Signal<String>,
        artwork_fade_animation: Animation,
    ) -> Self {
        let initial_local_volume = Self::persisted_local_volume_percent() as f32;

        Self {
            backend,
            playback,
            status,
            playback_ready: Signal::new(false),
            playback_is_playing: Signal::new(false),
            queue_tracks: Signal::new(Vec::new()),
            playback_scrub_percent: Signal::new(0.0),
            queue_current_index: Signal::new(None),
            playback_volume: Signal::new(initial_local_volume),
            playback_is_muted: Signal::new(false),
            pre_mute_volume: initial_local_volume,
            recently_played: Signal::new(Vec::new()),

            playback_duration_ms: Signal::new(0),
            playback_track_name: Signal::new(String::new()),
            playback_track_artist: Signal::new(String::new()),
            playback_track_id: Signal::new(None),
            playback_track_image_key: Signal::new(None),
            playback_track_image_url: Signal::new(None),
            playback_overlay_image_key: Signal::new(None),
            search_album_rows: Signal::new(Vec::new()),
            album_tracks: Signal::new(Vec::new()),
            album_image_key: Signal::new(None),
            last_scrub_user_input_at: None,
            last_local_track_end_handled_at: None,
            artwork_fade_animation,

            autoplay_on_queue_add: Signal::new(true),
            restore_queue_on_startup: Signal::new(false),
        }
    }

    fn set_current_track_artwork(&self, cx: &mut EventContext, track: &Track) {
        if let Some(image_key) = track.album_image_key.clone() {
            cx.emit(PlaybackEvent::ArtworkLoaded {
                image_key: Some(image_key),
            });
        }

        let playback_image_url = track
            .album_playback_image_url
            .clone()
            .or(track.album_image_url.clone());

        if let Some(url) = playback_image_url {
            worker::load_playback_artwork(Some(url), cx);
        } else if track.album_image_key.is_none() {
            // Explicitly clear artwork only when this track has no image source at all.
            cx.emit(PlaybackEvent::ArtworkLoaded { image_key: None });
        }
    }

    fn local_track_near_end(&self) -> bool {
        let duration_ms = self.playback_duration_ms.get();
        if duration_ms == 0 {
            return false;
        }
        let position_ms =
            (self.playback_scrub_percent.get() / 100.0 * duration_ms as f32).round() as u32;
        position_ms >= duration_ms.saturating_sub(900)
    }

    fn local_should_start_from_queue(&self) -> bool {
        if self.queue_tracks.with(|queue| queue.is_empty()) {
            return false;
        }

        self.queue_current_index.get().is_none() || self.local_track_near_end()
    }

    fn save_queue(&self) {
        let queue_tracks = self.queue_tracks.get();
        let recently_played = self.recently_played.get();
        let snapshot = QueueSnapshot {
            queue_tracks,
            recently_played,
        };
        let _ = snapshot.save();
    }

    pub fn restore_queue(&mut self) {
        if self.restore_queue_on_startup.get() {
            if let Ok(Some(snapshot)) = QueueSnapshot::load() {
                if !snapshot.queue_tracks.is_empty() {
                    self.queue_tracks.set(snapshot.queue_tracks);
                    self.recently_played.set(snapshot.recently_played);
                    self.status
                        .set("Queue restored from last session.".to_string());
                }
            }
        }
    }
}

impl Model for PlaybackState {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|playback_event, _: &mut _| match playback_event {
            PlaybackEvent::ClearQueue => {
                self.queue_tracks.update(|queue| queue.clear());
                self.queue_current_index.set(None);
                self.playback_duration_ms.set(0);
                self.playback_scrub_percent.set(0.0);
                self.playback_track_name.set("".to_string());
                self.playback_track_artist.set("".to_string());
                self.playback_track_id.set(None);
                self.playback_track_image_key.set(None);
                self.playback_track_image_url.set(None);
                self.playback_overlay_image_key.set(None);
                self.save_queue();
                cx.emit(PlaybackEvent::Stop);
            }
            PlaybackEvent::ClearRecentlyPlayed => {
                self.recently_played.update(|list| list.clear());
            }
            PlaybackEvent::ShuffleQueue => {
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
            PlaybackEvent::AddToQueue(tracks) => {
                let was_empty = self.queue_tracks.with(|queue| queue.is_empty());
                self.queue_tracks
                    .update(|queue| queue.extend(tracks.clone()));
                if was_empty && self.autoplay_on_queue_add.get() {
                    cx.emit(PlaybackEvent::Play);
                }
                self.save_queue();
                self.status
                    .set(format!("Added {} tracks to the queue.", tracks.len()));
            }

            PlaybackEvent::OpenAlbumFromPlayback {
                track_id,
                image_key,
                image_url,
            } => {
                if let Some(track_id) = track_id {
                    self.status
                        .set("Loading album from current track...".to_string());
                    cx.emit(CenterPanelEvent::NavigateTo(CenterPage::AlbumTracks));
                    worker::fetch_album_from_track(self.backend.clone(), track_id.clone(), cx);
                    return;
                }

                let current_album_key = self.album_image_key.get();
                if image_key.is_some()
                    && *image_key == current_album_key
                    && !self.album_tracks.get().is_empty()
                {
                    cx.emit(CenterPanelEvent::NavigateTo(CenterPage::AlbumTracks));
                    return;
                }

                let albums = self.search_album_rows.get();
                let by_key = image_key.as_ref().and_then(|key| {
                    albums
                        .iter()
                        .find(|album| album.image_key.as_ref() == Some(key))
                        .cloned()
                });
                let by_url = image_url.as_ref().and_then(|url| {
                    albums
                        .iter()
                        .find(|album| album.image_url.as_ref() == Some(url))
                        .cloned()
                });

                if let Some(album) = by_key.or(by_url) {
                    self.status
                        .set(format!("Loading tracks for '{}'...", album.name));
                    cx.emit(CenterPanelEvent::NavigateTo(CenterPage::AlbumTracks));
                    worker::fetch_album_tracks(self.backend.clone(), album, cx);
                } else {
                    self.status
                        .set("Album not found in current search results.".to_string());
                }
            }
            PlaybackEvent::SelectQueueTrack(index) => {
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
                self.playback_track_id.set(Some(selected_track.id.clone()));
                self.playback_track_image_url
                    .set(selected_track.album_image_url.clone());
                self.set_current_track_artwork(cx, &selected_track);
                self.save_queue();

                self.status
                    .set("Playing selected queue song on local device...".to_string());
                worker::playback_play_local_track(self.backend.clone(), selected_track, cx);
                self.playback_is_playing.set_if_changed(true);
            }
            PlaybackEvent::Previous => {
                // If more than 3 seconds into the current track, restart it.
                let position_ms = {
                    let pct = self.playback_scrub_percent.get();
                    let dur = self.playback_duration_ms.get();
                    (pct / 100.0 * dur as f32).round() as u32
                };

                if position_ms > 3000 {
                    // Restart the current track.
                    self.status.set("Restarting current track...".to_string());
                    cx.emit(PlaybackEvent::Play);
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
                    cx.emit(PlaybackEvent::Play);
                } else if self.queue_tracks.with(|queue| !queue.is_empty()) {
                    // Nothing in recently played — just restart.
                    self.status
                        .set("Replaying from start of queue...".to_string());
                    cx.emit(PlaybackEvent::Play);
                } else {
                    self.status.set("Nothing to go back to.".to_string());
                }
            }
            PlaybackEvent::Toggle => {
                if self.playback_is_playing.get() {
                    cx.emit(PlaybackEvent::Pause);
                } else {
                    if self.local_should_start_from_queue() {
                        cx.emit(PlaybackEvent::Play);
                    } else if self.playback_scrub_percent.get() > 0.0 {
                        cx.emit(PlaybackEvent::Resume);
                    } else {
                        cx.emit(PlaybackEvent::Play);
                    }
                }
            }
            PlaybackEvent::Stop => {
                self.playback_is_playing.set_if_changed(false);
                self.playback_scrub_percent.set(0.0);
                self.playback_track_name.set("".to_string());
                self.playback_track_id.set(None);
                self.playback_track_image_url.set(None);
                worker::playback_stop_local(self.backend.clone(), cx);
            }
            PlaybackEvent::Resume => {
                if self.local_should_start_from_queue() {
                    self.status
                        .set("Starting playback from queue on local device...".to_string());
                    cx.emit(PlaybackEvent::Play);
                    return;
                }

                self.status
                    .set("Resuming playback on local device...".to_string());
                worker::playback_resume_local(self.backend.clone(), cx);
                self.playback_is_playing.set_if_changed(true);
            }
            PlaybackEvent::Play => {
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
                    self.playback_track_id.set(Some(track.id.clone()));
                    self.playback_track_image_url
                        .set(track.album_image_url.clone());
                    self.set_current_track_artwork(cx, &track);

                    self.status
                        .set("Starting playback from queue on local device...".to_string());

                    worker::playback_play_local_track(self.backend.clone(), track, cx);
                    self.playback_is_playing.set_if_changed(true);
                    return;
                }
            }
            PlaybackEvent::Pause => {
                self.status
                    .set("Sending pause command to local device...".to_string());
                worker::playback_pause_local(self.backend.clone(), cx);
                self.playback_is_playing.set_if_changed(false);
            }
            PlaybackEvent::Next => {
                if self.queue_tracks.with(|queue| queue.len() <= 1) {
                    self.status.set("Queue is empty.".to_string());
                    return;
                }

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
                    self.save_queue();
                    cx.emit(PlaybackEvent::Play);
                } else {
                    self.queue_current_index.set(None);
                    self.playback_is_playing.set_if_changed(false);
                    self.save_queue();
                    self.status.set("Reached end of queue.".to_string());
                }
                return;
            }
            PlaybackEvent::ToggleMute => {
                if self.playback_is_muted.get() {
                    // Unmute: restore saved volume
                    self.playback_is_muted.set(false);
                    let restore = self.pre_mute_volume;
                    cx.emit(PlaybackEvent::SetVolume(restore));
                } else {
                    // Mute: save current volume then set to 0
                    self.pre_mute_volume = self.playback_volume.get();
                    self.playback_is_muted.set(true);
                    cx.emit(PlaybackEvent::SetVolume(0.0));
                }
            }
            PlaybackEvent::SetVolume(value) => {
                let clamped = value.clamp(0.0, 100.0);
                self.playback_volume.set(clamped);

                let target = clamped.round() as u8;
                let playback = {
                    let state = self
                        .playback
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    state.local_handle()
                };
                let result = playback.set_volume_percent(target);

                if let Err(err) = result {
                    self.status
                        .set(format!("Failed to set local volume: {err}"));
                } else {
                    let _ = LocalPlaybackSettings {
                        local_volume_percent: target,
                    }
                    .save();
                }
            }
            PlaybackEvent::SetScrub(value) => {
                let clamped = value.clamp(0.0, 100.0);
                self.playback_scrub_percent.set(clamped);
                self.last_scrub_user_input_at = Some(Instant::now());

                let duration = self.playback_duration_ms.get();
                if duration == 0 {
                    return;
                }

                let target_ms = ((duration as f32) * (clamped / 100.0)).round() as u32;

                worker::playback_seek_local(self.backend.clone(), target_ms, cx);
            }
            _ => {}
        });

        event.map(|playback_event, _: &mut _| match playback_event {
            PlaybackEvent::SessionReady => {
                let persisted_local_volume = Self::persisted_local_volume_percent() as f32;
                self.playback_ready.set(true);
                self.playback_volume.set_if_changed(persisted_local_volume);
                self.pre_mute_volume = persisted_local_volume;
                self.playback_is_muted.set_if_changed(false);
                self.status
                    .set("Playback session is ready (OAuth token-based).".to_string());
            }

            PlaybackEvent::LocalTrackEnded => {
                if self
                    .last_local_track_end_handled_at
                    .map(|at| at.elapsed() < Duration::from_millis(700))
                    .unwrap_or(false)
                {
                    return;
                }
                self.last_local_track_end_handled_at = Some(Instant::now());

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
                    self.save_queue();
                    cx.emit(PlaybackEvent::Play);
                } else {
                    self.queue_current_index.set(None);
                    self.playback_is_playing.set_if_changed(false);
                    self.save_queue();
                    self.status.set("Reached end of queue.".to_string());
                }
            }

            PlaybackEvent::Progress {
                position_ms,
                duration_ms,
                is_playing,
            } => {
                if self.playback_is_playing.get() != *is_playing {
                    self.playback_is_playing.set_if_changed(*is_playing);
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
            PlaybackEvent::ArtworkLoaded { image_key } => {
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
            _ => {}
        });
    }
}
