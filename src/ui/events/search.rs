use crate::messages::{Album, SearchResultsData, Track};

#[derive(Clone, Debug)]
pub struct AlbumTracksData {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub image_url: Option<String>,
    pub image_key: Option<String>,
    pub tracks: Vec<Track>,
    pub release_year: Option<u32>,
    pub track_count: usize,
    pub total_duration_ms: u64,
}

#[derive(Clone, Debug)]
pub enum SearchEvent {
    SelectTab(usize),
    SelectTrack(usize),
    SelectArtist(usize),
    SelectAlbum(usize),
    OpenAlbumFromTrack(String),
    OpenArtistFromTrack(String),
    SetInput(String),
    SubmitQuery(String),
    Results(SearchResultsData),
    HydrateArtwork(SearchResultsData),
    LoadAlbumTracks(Album),
    HydrateAlbumArtwork(AlbumTracksData),
    HydrateArtistArtwork {
        id: String,
        name: String,
        image_url: Option<String>,
        albums: Vec<Album>,
    },
}

#[derive(Clone, Debug)]
pub enum AlbumEvent {
    AlbumTracks(AlbumTracksData),
    AlbumTrackSelected(usize),
    PlayAlbum,
    ToggleShuffleAlbum,
}

#[derive(Clone, Debug)]
pub enum ArtistEvent {
    ArtistView {
        id: String,
        name: String,
        image_key: Option<String>,
        albums: Vec<Album>,
    },
    ArtistAlbumSelected(usize),
}
