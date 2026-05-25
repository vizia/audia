use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub duration_ms: u32,
    pub album_image_url: Option<String>,
    #[serde(default)]
    pub album_playback_image_url: Option<String>,
    pub album_image_key: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
    pub image_key: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub release_date: Option<String>,
    pub image_url: Option<String>,
    pub image_key: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SearchResultsData {
    pub tracks: Vec<Track>,
    pub artists: Vec<Artist>,
    pub albums: Vec<Album>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlaylistEntry {
    pub id: String,
    pub name: String,
    pub image_key: Option<String>,
    pub track_count: usize,
    pub total_duration_ms: u64,
}
