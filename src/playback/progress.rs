use std::sync::atomic::Ordering;

use super::LocalPlaybackHandle;

impl LocalPlaybackHandle {
    pub fn playback_progress(&self) -> Option<(u32, u32, bool)> {
        Some((
            self.progress_position_ms.load(Ordering::Relaxed),
            self.progress_duration_ms.load(Ordering::Relaxed),
            self.progress_is_playing.load(Ordering::Relaxed),
        ))
    }

    pub fn mark_track_finished_if_stalled(&self) -> bool {
        let position_ms = self.progress_position_ms.load(Ordering::Relaxed);
        let duration_ms = self.progress_duration_ms.load(Ordering::Relaxed);
        let is_playing = self.progress_is_playing.load(Ordering::Relaxed);

        if is_playing
            || self.loading_track.load(Ordering::Relaxed)
            || duration_ms == 0
            || position_ms < duration_ms.saturating_sub(900)
        {
            return false;
        }

        !self.progress_track_finished.swap(true, Ordering::Relaxed)
    }

    pub fn consume_track_finished(&self) -> bool {
        self.progress_track_finished.swap(false, Ordering::Relaxed)
    }
}

impl super::PlaybackService {
    pub fn reset(&mut self) {
        self.session_ready = false;
        self.mixer = None;
        self.player = None;
        self.session = None;
        self.progress_position_ms.store(0, Ordering::Relaxed);
        self.progress_duration_ms.store(0, Ordering::Relaxed);
        self.progress_is_playing.store(false, Ordering::Relaxed);
        self.progress_track_finished.store(false, Ordering::Relaxed);
        self.loading_track.store(false, Ordering::Relaxed);
    }
}