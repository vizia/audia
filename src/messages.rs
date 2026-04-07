use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub duration_ms: u32,
    pub album_image_url: Option<String>,
    pub album_image_key: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtistResult {
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
    pub image_key: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AlbumResult {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub image_url: Option<String>,
    pub image_key: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SearchResultsData {
    pub tracks: Vec<Track>,
    pub artists: Vec<ArtistResult>,
    pub albums: Vec<AlbumResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlaybackDevice {
    pub id: Option<String>,
    pub name: String,
    pub is_active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlaylistEntry {
    pub id: String,
    pub name: String,
    pub image_key: Option<String>,
    pub track_count: usize,
    pub total_duration_ms: u64,
}
