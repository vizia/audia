use librespot_core::spotify_uri::SpotifyUri;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::messages::Track;

use super::{LocalPlaybackHandle, PlaybackService};

impl LocalPlaybackHandle {
    fn mixer(&self) -> Result<&Arc<dyn librespot_playback::mixer::Mixer>, String> {
        self.mixer
            .as_ref()
            .ok_or_else(|| "Local mixer is not initialized".to_string())
    }

    fn player(&self) -> Result<&Arc<librespot_playback::player::Player>, String> {
        self.player
            .as_ref()
            .ok_or_else(|| "Local player is not initialized".to_string())
    }

    pub fn set_volume_percent(&self, percent: u8) -> Result<(), String> {
        let mixer = self.mixer()?;
        mixer.set_volume(PlaybackService::percent_to_librespot_volume(percent));
        Ok(())
    }

    pub fn seek_to(&self, position_ms: u32) -> Result<(), String> {
        let player = self.player()?;

        player.seek(position_ms);
        Ok(())
    }

    pub fn play_track(&self, track: &Track) -> Result<(), String> {
        let player = self.player()?;

        self.progress_duration_ms
            .store(track.duration_ms, Ordering::Relaxed);

        self.progress_position_ms.store(0, Ordering::Relaxed);
        self.progress_is_playing.store(false, Ordering::Relaxed);
        self.progress_track_finished.store(false, Ordering::Relaxed);
        self.loading_track.store(true, Ordering::Relaxed);

        let uri = SpotifyUri::from_uri(&format!("spotify:track:{}", track.id))
            .map_err(|err| format!("Invalid Spotify track id: {err}"))?;

        player.stop();
        player.load(uri, true, 0);
        player.play();
        Ok(())
    }

    pub fn resume(&self) -> Result<(), String> {
        let player = self.player()?;
        player.play();
        Ok(())
    }

    pub fn pause(&self) -> Result<(), String> {
        let player = self.player()?;

        player.pause();
        Ok(())
    }

    pub fn stop(&self) -> Result<(), String> {
        let player = self.player()?;

        player.stop();
        self.progress_position_ms.store(0, Ordering::Relaxed);
        self.progress_is_playing.store(false, Ordering::Relaxed);
        Ok(())
    }
}
