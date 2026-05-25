use serde::{Deserialize, Deserializer};

pub(super) fn null_as_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

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
    pub(super) release_date: Option<String>,
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
    pub(super) width: Option<u32>,
    pub(super) height: Option<u32>,
}

pub(super) fn pick_image_url(images: &[SpotifyImage], target_px: u32) -> Option<String> {
    if images.is_empty() {
        return None;
    }

    let best = images
        .iter()
        .filter_map(|image| {
            let dim = image.width.or(image.height)?;
            let undersized = dim < target_px;
            let delta = if undersized {
                target_px.saturating_sub(dim)
            } else {
                dim.saturating_sub(target_px)
            };
            Some(((undersized, delta), image))
        })
        .min_by_key(|(score, _)| *score)
        .map(|(_, image)| image);

    best.or_else(|| images.first())
        .map(|image| image.url.clone())
}

pub(super) fn pick_largest_image_url(images: &[SpotifyImage]) -> Option<String> {
    if images.is_empty() {
        return None;
    }

    images
        .iter()
        .max_by_key(|image| image.width.or(image.height).unwrap_or(0))
        .or_else(|| images.first())
        .map(|image| image.url.clone())
}

#[derive(Debug, Deserialize)]
pub(super) struct SearchArtist {
    pub(super) name: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct PlaylistListResponse {
    pub(super) items: Vec<Option<PlaylistItem>>,
}

#[derive(Debug, Deserialize)]
pub(super) struct PlaylistItem {
    pub(super) id: Option<String>,
    pub(super) name: Option<String>,
    pub(super) owner: Option<PlaylistOwner>,
    pub(super) collaborative: Option<bool>,
    pub(super) public: Option<bool>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub(super) images: Vec<SpotifyImage>,
    #[serde(default, rename = "items")]
    pub(super) tracks: Option<PlaylistTrackCount>,
}

#[derive(Debug, Deserialize)]
pub(super) struct PlaylistOwner {
    pub(super) id: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct PlaylistTrackCount {
    pub(super) total: Option<usize>,
}
