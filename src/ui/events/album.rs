use super::search::AlbumTracksData;

#[derive(Clone, Debug)]
pub enum AlbumEvent {
    AlbumTracks(AlbumTracksData),
    AlbumTrackSelected(usize),
    PlayAlbum,
    ToggleShuffleAlbum,
}
