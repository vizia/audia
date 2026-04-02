use crate::messages::{PlaylistEntry, Track};

#[derive(Clone, Debug)]
pub enum PlaylistsUiEvent {
    SelectPlaylist(usize),
    BackToSearch,
    AddPlaylistToQueue,
    PlaylistTrackSelected(usize),
}

#[derive(Clone, Debug)]
pub enum PlaylistsAppEvent {
    Playlists(Vec<PlaylistEntry>),
    PlaylistTracks { name: String, tracks: Vec<Track> },
}
