use crate::messages::{Album, SearchResultsData, Track};

#[derive(Clone, Debug)]
pub enum SearchUiEvent {
    SelectTab(usize),
    SelectResult(usize),
    SelectArtist(usize),
    SelectAlbum(usize),
    OpenAlbumFromTrack(String),
    OpenArtistFromTrack(String),
    SetInput(String),
    SubmitQuery(String),
}

#[derive(Clone, Debug)]
pub enum AlbumUiEvent {
    AlbumTrackSelected(usize),
    PlayAlbum,
    ShuffleAlbum,
}

#[derive(Clone, Debug)]
pub enum ArtistUiEvent {
    ArtistAlbumSelected(usize),
}

#[derive(Clone, Debug)]
pub enum SearchAppEvent {
    Results(SearchResultsData),
    AlbumTracks {
        id: String,
        name: String,
        artist: String,
        image_key: Option<String>,
        tracks: Vec<Track>,
        release_year: Option<u32>,
        track_count: usize,
        total_duration_ms: u64,
    },
    ArtistView {
        id: String,
        name: String,
        image_key: Option<String>,
        albums: Vec<Album>,
    },
}
