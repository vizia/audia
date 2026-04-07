use crate::messages::{PlaybackDevice, Track};

#[derive(Clone, Debug)]
pub enum PlaybackUiEvent {
    RefreshDevices,
    SelectPlaybackDevice(usize),
    SelectQueueTrack(usize),
    AddToQueue(Vec<Track>),
    ShuffleQueue,
    ClearQueue,
    ClearRecentlyPlayed,
    Previous,
    Toggle,
    Resume,
    Play,
    Stop,
    Pause,
    Next,
    SetVolume(f32),
    SetScrub(f32),
}

#[derive(Clone, Debug)]
pub enum PlaybackProgressSource {
    Local,
    Remote,
}

#[derive(Clone, Debug)]
pub enum PlaybackAppEvent {
    SessionReady,
    Devices(Vec<PlaybackDevice>),
    LocalTrackEnded,

    Progress {
        source: PlaybackProgressSource,
        position_ms: u32,
        duration_ms: u32,
        is_playing: bool,
    },
    ArtworkLoaded {
        image_key: Option<String>,
    },
}
