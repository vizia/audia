use crate::messages::{PlaylistEntry, Track};

#[derive(Clone, Debug)]
pub enum PlaylistsUiEvent {
    SelectPlaylist(usize),
    AddPlaylistToQueue,
    PlayPlaylist,
    PlaylistTrackSelected(usize),
    ShufflePlaylist,
}

#[derive(Clone, Debug)]
pub enum PlaylistsAppEvent {
    Playlists(Vec<PlaylistEntry>),
    PlaylistTracks {
        id: String,
        name: String,
        tracks: Vec<Track>,
        track_count: usize,
        total_duration_ms: u64,
    },
}
