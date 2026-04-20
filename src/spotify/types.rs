use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct SearchResponse {
    pub(super) tracks: SearchTrackContainer,
}

#[derive(Debug, Deserialize)]
pub(super) struct ArtistSearchResponse {
    pub(super) artists: ArtistSearchContainer,
}

#[derive(Debug, Deserialize)]
pub(super) struct ArtistSearchContainer {
    pub(super) items: Vec<ArtistSearchItem>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ArtistSearchItem {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) images: Vec<SpotifyImage>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AlbumSearchResponse {
    pub(super) albums: AlbumSearchContainer,
}

#[derive(Debug, Deserialize)]
pub(super) struct AlbumSearchContainer {
    pub(super) items: Vec<AlbumSearchItem>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AlbumSearchItem {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) artists: Vec<SearchArtist>,
    pub(super) images: Vec<SpotifyImage>,
}

#[derive(Debug, Deserialize)]
pub(super) struct SearchTrackContainer {
    pub(super) items: Vec<SearchTrackItem>,
}

#[derive(Debug, Deserialize)]
pub(super) struct SearchTrackItem {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) artists: Vec<SearchArtist>,
    pub(super) duration_ms: u32,
    pub(super) album: AlbumSummary,
}

#[derive(Debug, Deserialize)]
pub(super) struct AlbumSummary {
    pub(super) images: Vec<SpotifyImage>,
}

#[derive(Debug, Deserialize)]
pub(super) struct SpotifyImage {
    pub(super) url: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct SearchArtist {
    pub(super) name: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct DeviceListResponse {
    pub(super) devices: Vec<DeviceItem>,
}

#[derive(Debug, Deserialize)]
pub(super) struct DeviceItem {
    pub(super) id: Option<String>,
    pub(super) name: String,
    pub(super) is_active: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct PlaylistListResponse {
    pub(super) items: Vec<PlaylistItem>,
}

#[derive(Debug, Deserialize)]
pub(super) struct PlaylistItem {
    pub(super) id: String,
    pub(super) name: String,
    #[serde(default)]
    pub(super) images: Vec<SpotifyImage>,
    #[serde(default)]
    pub(super) tracks: Option<PlaylistTrackCount>,
}

#[derive(Debug, Deserialize)]
pub(super) struct PlaylistTrackCount {
    pub(super) total: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RecommendationsResponse {
    pub(super) tracks: Vec<RecommendationTrackItem>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RecommendationTrackItem {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) artists: Vec<SearchArtist>,
    pub(super) duration_ms: u32,
    pub(super) album: AlbumSummary,
}

#[derive(Debug, Deserialize)]
pub(super) struct CurrentPlaybackResponse {
    pub(super) is_playing: bool,
    pub(super) progress_ms: Option<u32>,
    pub(super) item: Option<PlaybackItem>,
}

#[derive(Debug, Deserialize)]
pub(super) struct PlaybackItem {
    pub(super) duration_ms: u32,
}