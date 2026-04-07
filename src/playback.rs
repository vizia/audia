use crate::messages::Track;
use librespot_core::authentication::Credentials;
use librespot_core::config::SessionConfig;
use librespot_core::session::Session;
use librespot_core::spotify_uri::SpotifyUri;
use librespot_playback::audio_backend;
use librespot_playback::config::{AudioFormat, PlayerConfig};
use librespot_playback::mixer::{self, Mixer, MixerConfig};
use librespot_playback::player::{Player, PlayerEvent};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;

pub struct PlaybackService {
    session_ready: bool,
    session: Option<Session>,
    mixer: Option<Arc<dyn Mixer>>,
    player: Option<Arc<Player>>,
    progress_position_ms: Arc<AtomicU32>,
    progress_duration_ms: Arc<AtomicU32>,
    progress_is_playing: Arc<AtomicBool>,
    progress_track_finished: Arc<AtomicBool>,
}

impl Default for PlaybackService {
    fn default() -> Self {
        Self {
            session_ready: false,
            session: None,
            mixer: None,
            player: None,
            progress_position_ms: Arc::new(AtomicU32::new(0)),
            progress_duration_ms: Arc::new(AtomicU32::new(0)),
            progress_is_playing: Arc::new(AtomicBool::new(false)),
            progress_track_finished: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl PlaybackService {
    pub fn bootstrap_from_access_token(
        &mut self,
        runtime: &Runtime,
        access_token: &str,
    ) -> Result<(), String> {
        if access_token.trim().is_empty() {
            return Err("Access token is required for playback bootstrap".to_string());
        }

        let credentials = Credentials::with_access_token(access_token.to_string());
        let session = {
            let _guard = runtime.enter();
            Session::new(SessionConfig::default(), None)
        };

        runtime
            .block_on(session.connect(credentials, false))
            .map_err(|err| format!("Failed to connect librespot session: {err}"))?;

        let sink_builder = audio_backend::find(Some("rodio".to_string()))
            .or_else(|| audio_backend::find(None))
            .ok_or_else(|| "No librespot audio sink backend is available".to_string())?;

        let mixer_builder = mixer::find(Some("softvol"))
            .or_else(|| mixer::find(None))
            .ok_or_else(|| "No librespot mixer backend is available".to_string())?;

        let mixer = mixer_builder(MixerConfig::default())
            .map_err(|err| format!("Failed to initialize mixer: {err}"))?;
        mixer.set_volume(Self::percent_to_librespot_volume(80));

        let player_config = PlayerConfig {
            position_update_interval: Some(Duration::from_millis(500)),
            ..Default::default()
        };

        let player = {
            let _guard = runtime.enter();
            Player::new(
                player_config,
                session.clone(),
                mixer.get_soft_volume(),
                move || sink_builder(None, AudioFormat::default()),
            )
        };

        let position = Arc::clone(&self.progress_position_ms);
        let duration = Arc::clone(&self.progress_duration_ms);
        let is_playing = Arc::clone(&self.progress_is_playing);
        let track_finished = Arc::clone(&self.progress_track_finished);
        let mut events = player.get_player_event_channel();
        thread::spawn(move || {
            while let Some(event) = events.blocking_recv() {
                match event {
                    PlayerEvent::Playing { position_ms, .. }
                    | PlayerEvent::PositionChanged { position_ms, .. }
                    | PlayerEvent::PositionCorrection { position_ms, .. }
                    | PlayerEvent::Seeked { position_ms, .. } => {
                        position.store(position_ms, Ordering::Relaxed);
                        is_playing.store(true, Ordering::Relaxed);
                    }
                    PlayerEvent::Paused { position_ms, .. } => {
                        position.store(position_ms, Ordering::Relaxed);
                        is_playing.store(false, Ordering::Relaxed);
                    }
                    PlayerEvent::Stopped { .. } => {
                        is_playing.store(false, Ordering::Relaxed);

                        // Some backends can stop at track end without reliably sending EndOfTrack.
                        // If we are at (or very near) the known duration, treat this as a finished track.
                        let d = duration.load(Ordering::Relaxed);
                        let p = position.load(Ordering::Relaxed);
                        if d > 0 && p >= d.saturating_sub(750) {
                            track_finished.store(true, Ordering::Relaxed);
                        }
                    }
                    PlayerEvent::EndOfTrack { .. } => {
                        is_playing.store(false, Ordering::Relaxed);
                        track_finished.store(true, Ordering::Relaxed);
                    }
                    _ => {}
                }

                let d = duration.load(Ordering::Relaxed);
                let p = position.load(Ordering::Relaxed);
                if d > 0 && p > d {
                    position.store(d, Ordering::Relaxed);
                }
            }
        });

        self.session_ready = true;
        self.session = Some(session);
        self.mixer = Some(mixer);
        self.player = Some(player);
        Ok(())
    }

    fn percent_to_librespot_volume(percent: u8) -> u16 {
        let clamped = percent.min(100) as u32;
        ((clamped * u16::MAX as u32) / 100) as u16
    }

    pub fn set_volume_percent(&self, percent: u8) -> Result<(), String> {
        let mixer = self
            .mixer
            .as_ref()
            .ok_or_else(|| "Local mixer is not initialized".to_string())?;
        mixer.set_volume(Self::percent_to_librespot_volume(percent));
        Ok(())
    }

    pub fn seek_to(&self, position_ms: u32) -> Result<(), String> {
        let player = self
            .player
            .as_ref()
            .ok_or_else(|| "Local player is not initialized".to_string())?;

        player.seek(position_ms);
        Ok(())
    }

    pub fn play_track(&mut self, track: &Track) -> Result<(), String> {
        let player = self
            .player
            .as_ref()
            .ok_or_else(|| "Local player is not initialized".to_string())?;

        self.progress_duration_ms
            .store(track.duration_ms, Ordering::Relaxed);

        self.progress_position_ms.store(0, Ordering::Relaxed);
        self.progress_track_finished.store(false, Ordering::Relaxed);

        let track_id = &track.id;

        let uri = SpotifyUri::from_uri(&format!("spotify:track:{track_id}"))
            .map_err(|err| format!("Invalid Spotify track id: {err}"))?;

        player.load(uri, true, 0);

        Ok(())
    }

    pub fn resume(&self) -> Result<(), String> {
        let player = self
            .player
            .as_ref()
            .ok_or_else(|| "Local player is not initialized".to_string())?;
        player.play();
        Ok(())
    }

    pub fn pause(&self) -> Result<(), String> {
        let player = self
            .player
            .as_ref()
            .ok_or_else(|| "Local player is not initialized".to_string())?;

        player.pause();
        Ok(())
    }

    pub fn stop(&self) -> Result<(), String> {
        let player = self
            .player
            .as_ref()
            .ok_or_else(|| "Local player is not initialized".to_string())?;

        player.stop();
        self.progress_position_ms.store(0, Ordering::Relaxed);
        self.progress_is_playing.store(false, Ordering::Relaxed);
        Ok(())
    }

    pub fn playback_progress(&self) -> Option<(u32, u32, bool)> {
        Some((
            self.progress_position_ms.load(Ordering::Relaxed),
            self.progress_duration_ms.load(Ordering::Relaxed),
            self.progress_is_playing.load(Ordering::Relaxed),
        ))
    }

    pub fn consume_track_finished(&self) -> bool {
        self.progress_track_finished.swap(false, Ordering::Relaxed)
    }

    pub fn reset(&mut self) {
        self.session_ready = false;
        self.mixer = None;
        self.player = None;
        self.session = None;
        self.progress_position_ms.store(0, Ordering::Relaxed);
        self.progress_duration_ms.store(0, Ordering::Relaxed);
        self.progress_is_playing.store(false, Ordering::Relaxed);
        self.progress_track_finished.store(false, Ordering::Relaxed);
    }
}
