use crate::messages::{PlaylistEntry, Track};

#[derive(Clone, Debug)]
pub enum PlaylistsUiEvent {
    SelectPlaylist(usize),
    AddPlaylistToQueue,
    PlayPlaylist,
    PlaylistTrackSelected(usize),
    ShufflePlaylist,
    SetTrackFilter(String),
}

#[derive(Clone, Debug)]
pub enum PlaylistsAppEvent {
    Playlists(Vec<PlaylistEntry>),
    PlaylistTracks {
        request_id: u64,
        id: String,
        name: String,
        tracks: Vec<Track>,
        track_count: usize,
        total_duration_ms: u64,
    },
}
