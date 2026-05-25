use crate::messages::{PlaylistEntry, Track};

#[derive(Clone, Debug)]
pub enum PlaylistsEvent {
    OpenCreatePlaylistModal,
    CloseCreatePlaylistModal,
    SetCreatePlaylistName(String),
    SubmitCreatePlaylist,
    OpenRenamePlaylistModal {
        id: String,
        name: String,
    },
    CloseRenamePlaylistModal,
    SetRenamePlaylistName(String),
    SubmitRenamePlaylist,
    DeletePlaylist(String),
    AddTrackToPlaylist {
        track_id: String,
        playlist_id: String,
    },
    RemoveTrackFromPlaylist {
        track_id: String,
        playlist_id: String,
    },
    SelectPlaylist(usize),
    AddPlaylistToQueue,
    PlayPlaylist,
    PlaylistTrackSelected(usize),
    ShufflePlaylist,
    SetTrackFilter(String),
    Playlists(Vec<PlaylistEntry>),
    HydrateUserPlaylistsArtwork {
        playlists: Vec<PlaylistEntry>,
        artwork_urls: Vec<Option<String>>,
    },
    RefreshUserPlaylists,
    RefreshPlaylistTracks {
        request_id: u64,
        id: String,
        name: String,
    },
    PlaylistCreated {
        id: String,
        name: String,
    },
    PlaylistCreateFailed(String),
    PlaylistRenamed {
        id: String,
        name: String,
    },
    PlaylistRenameFailed(String),
    PlaylistDeleted(String),
    PlaylistDeleteFailed(String),
    PlaylistTracks {
        request_id: u64,
        id: String,
        name: String,
        tracks: Vec<Track>,
        track_count: usize,
        total_duration_ms: u64,
    },
}
