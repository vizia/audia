use librespot_core::authentication::Credentials;
use librespot_core::config::SessionConfig;
use librespot_core::session::Session;
use librespot_playback::audio_backend;
use librespot_playback::config::{AudioFormat, PlayerConfig};
use librespot_playback::mixer::{self, MixerConfig};
use librespot_playback::player::{Player, PlayerEvent};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use super::{DEFAULT_LOCAL_VOLUME_PERCENT, PlaybackService};

impl PlaybackService {
    pub async fn bootstrap_from_access_token(&mut self, access_token: &str) -> Result<(), String> {
        if access_token.trim().is_empty() {
            return Err("Access token is required for playback bootstrap".to_string());
        }

        let credentials = Credentials::with_access_token(access_token.to_string());
        let session = Session::new(SessionConfig::default(), None);

        session
            .connect(credentials, false)
            .await
            .map_err(|err| format!("Failed to connect librespot session: {err}"))?;

        let sink_builder = audio_backend::find(Some("rodio".to_string()))
            .or_else(|| audio_backend::find(None))
            .ok_or_else(|| "No librespot audio sink backend is available".to_string())?;

        let mixer_builder = mixer::find(Some("softvol"))
            .or_else(|| mixer::find(None))
            .ok_or_else(|| "No librespot mixer backend is available".to_string())?;

        let mixer = mixer_builder(MixerConfig::default())
            .map_err(|err| format!("Failed to initialize mixer: {err}"))?;
        mixer.set_volume(Self::percent_to_librespot_volume(
            DEFAULT_LOCAL_VOLUME_PERCENT,
        ));

        let player_config = PlayerConfig {
            position_update_interval: Some(Duration::from_millis(500)),
            ..Default::default()
        };

        let player = Player::new(
            player_config,
            session.clone(),
            mixer.get_soft_volume(),
            move || sink_builder(None, AudioFormat::default()),
        );

        let position = Arc::clone(&self.progress_position_ms);
        let duration = Arc::clone(&self.progress_duration_ms);
        let is_playing = Arc::clone(&self.progress_is_playing);
        let track_finished = Arc::clone(&self.progress_track_finished);
        let loading_track = Arc::clone(&self.loading_track);
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
                        loading_track.store(false, Ordering::Relaxed);
                    }
                    PlayerEvent::Paused { position_ms, .. } => {
                        position.store(position_ms, Ordering::Relaxed);
                        is_playing.store(false, Ordering::Relaxed);
                        if position_ms > 0 {
                            loading_track.store(false, Ordering::Relaxed);
                        }
                    }
                    PlayerEvent::Stopped { .. } => {
                        is_playing.store(false, Ordering::Relaxed);

                        let d = duration.load(Ordering::Relaxed);
                        let p = position.load(Ordering::Relaxed);
                        if !loading_track.load(Ordering::Relaxed)
                            && d > 0
                            && p >= d.saturating_sub(750)
                        {
                            track_finished.store(true, Ordering::Relaxed);
                        }
                    }
                    PlayerEvent::EndOfTrack { .. } => {
                        is_playing.store(false, Ordering::Relaxed);
                        if !loading_track.load(Ordering::Relaxed) {
                            track_finished.store(true, Ordering::Relaxed);
                        }
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
}
