use librespot_core::session::Session;
use librespot_playback::mixer::Mixer;
use librespot_playback::player::Player;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32};

mod bootstrap;
mod control;
mod progress;

pub struct PlaybackService {
    pub(super) session_ready: bool,
    pub(super) session: Option<Session>,
    pub(super) mixer: Option<Arc<dyn Mixer>>,
    pub(super) player: Option<Arc<Player>>,
    pub(super) progress_position_ms: Arc<AtomicU32>,
    pub(super) progress_duration_ms: Arc<AtomicU32>,
    pub(super) progress_is_playing: Arc<AtomicBool>,
    pub(super) progress_track_finished: Arc<AtomicBool>,
    pub(super) loading_track: Arc<AtomicBool>,
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
            loading_track: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl PlaybackService {
    pub(super) fn percent_to_librespot_volume(percent: u8) -> u16 {
        let clamped = percent.min(100) as u32;
        ((clamped * u16::MAX as u32) / 100) as u16
    }
}