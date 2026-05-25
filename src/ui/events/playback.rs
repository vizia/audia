use crate::messages::Track;

#[derive(Clone, Debug)]
pub enum PlaybackEvent {
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
    ToggleMute,
    SetScrub(f32),
    OpenAlbumFromPlayback {
        track_id: Option<String>,
        image_key: Option<String>,
        image_url: Option<String>,
    },
    SessionReady,
    LocalTrackEnded,

    Progress {
        position_ms: u32,
        duration_ms: u32,
        is_playing: bool,
    },
    ArtworkLoaded {
        image_key: Option<String>,
    },
}
