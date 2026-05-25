use super::{
    SpotifyService,
    types::{
        AlbumSearchResponse, ArtistSearchResponse, SearchResponse, pick_image_url,
        pick_largest_image_url,
    },
};
use crate::messages::{Album, Artist, SearchResultsData, Track};

const SEARCH_TRACK_LIST_IMAGE_TARGET_PX: u32 = 64;
const SEARCH_LIST_IMAGE_TARGET_PX: u32 = 64;
const SEARCH_ALBUM_IMAGE_TARGET_PX: u32 = 160;

impl SpotifyService {
    pub async fn search_catalog(&self, query: &str) -> Result<SearchResultsData, String> {
        async fn fetch_track_page(
            service: &SpotifyService,
            query: &str,
            offset: u32,
        ) -> Result<Vec<Track>, String> {
            let token = service.access_token()?;
            let offset = offset.to_string();
            let response = service
                .http
                .get("https://api.spotify.com/v1/search")
                .bearer_auth(token)
                .query(&[
                    ("q", query),
                    ("type", "track"),
                    ("limit", "10"),
                    ("offset", offset.as_str()),
                ])
                .send()
                .await
                .map_err(|err| format!("Spotify track search failed: {err}"))?;

            if !response.status().is_success() {
                return Err(format!(
                    "Spotify track search returned status {}",
                    response.status()
                ));
            }

            let payload = response
                .json::<SearchResponse>()
                .await
                .map_err(|err| format!("Invalid Spotify track search payload: {err}"))?;

            Ok(payload
                .tracks
                .items
                .into_iter()
                .map(|item| Track {
                    id: item.id,
                    name: item.name,
                    artist: item
                        .artists
                        .first()
                        .map(|artist| artist.name.clone())
                        .unwrap_or_else(|| "Unknown artist".to_string()),
                    duration_ms: item.duration_ms,
                    album_image_url: pick_image_url(
                        &item.album.images,
                        SEARCH_TRACK_LIST_IMAGE_TARGET_PX,
                    ),
                    album_playback_image_url: pick_largest_image_url(&item.album.images),
                    album_image_key: None,
                })
                .collect())
        }

        async fn fetch_artists(
            service: &SpotifyService,
            query: &str,
        ) -> Result<Vec<Artist>, String> {
            let token = service.access_token()?;
            let response = service
                .http
                .get("https://api.spotify.com/v1/search")
                .bearer_auth(token)
                .query(&[("q", query), ("type", "artist"), ("limit", "10")])
                .send()
                .await
                .map_err(|err| format!("Spotify artist search failed: {err}"))?;

            if !response.status().is_success() {
                return Err(format!(
                    "Spotify artist search returned status {}",
                    response.status()
                ));
            }

            let payload = response
                .json::<ArtistSearchResponse>()
                .await
                .map_err(|err| format!("Invalid Spotify artist search payload: {err}"))?;

            Ok(payload
                .artists
                .items
                .into_iter()
                .map(|item| Artist {
                    id: item.id,
                    name: item.name,
                    image_url: pick_image_url(&item.images, SEARCH_LIST_IMAGE_TARGET_PX),
                    image_key: None,
                })
                .collect())
        }

        async fn fetch_albums(service: &SpotifyService, query: &str) -> Result<Vec<Album>, String> {
            let token = service.access_token()?;
            let response = service
                .http
                .get("https://api.spotify.com/v1/search")
                .bearer_auth(token)
                .query(&[("q", query), ("type", "album"), ("limit", "10")])
                .send()
                .await
                .map_err(|err| format!("Spotify album search failed: {err}"))?;

            if !response.status().is_success() {
                return Err(format!(
                    "Spotify album search returned status {}",
                    response.status()
                ));
            }

            let payload = response
                .json::<AlbumSearchResponse>()
                .await
                .map_err(|err| format!("Invalid Spotify album search payload: {err}"))?;

            Ok(payload
                .albums
                .items
                .into_iter()
                .map(|item| Album {
                    id: item.id,
                    name: item.name,
                    artist: item
                        .artists
                        .first()
                        .map(|artist| artist.name.clone())
                        .unwrap_or_else(|| "Unknown artist".to_string()),
                    release_date: item.release_date,
                    image_url: pick_image_url(&item.images, SEARCH_ALBUM_IMAGE_TARGET_PX),
                    image_key: None,
                })
                .collect())
        }

        let (tracks, artists, albums) = tokio::try_join!(
            fetch_track_page(self, query, 0),
            fetch_artists(self, query),
            fetch_albums(self, query),
        )?;

        Ok(SearchResultsData {
            tracks,
            artists,
            albums,
        })
    }
}
