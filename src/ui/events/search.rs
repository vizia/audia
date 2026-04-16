use crate::messages::{SearchResultsData, Track};

#[derive(Clone, Debug)]
pub enum SearchUiEvent {
    SelectResult(usize),
    SelectAlbum(usize),
    SetInput(String),
    SubmitQuery(String),
}

#[derive(Clone, Debug)]
pub enum AlbumUiEvent {
    BackFromAlbum,
    AlbumTrackSelected(usize),
    PlayAlbumTrack(usize),
    PlayAlbum,
    ShuffleAlbum,
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
}
