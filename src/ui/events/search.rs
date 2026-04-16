use crate::messages::{SearchResultsData, Track};

#[derive(Clone, Debug)]
pub enum SearchUiEvent {
    SelectResult(usize),
    SetInput(String),
    SubmitQuery(String),
    SelectAlbum(usize),
    OpenAlbumFromPlayback {
        track_id: Option<String>,
        image_key: Option<String>,
        image_url: Option<String>,
    },
    BackFromAlbum,
    AlbumTrackSelected(usize),
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
    },
}
