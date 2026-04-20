use super::{SpotifyService, types::{AlbumSearchResponse, ArtistSearchResponse, SearchResponse, RecommendationsResponse}};
use crate::messages::{AlbumResult, ArtistResult, SearchResultsData, Track};

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
                    album_image_url: item.album.images.first().map(|img| img.url.clone()),
                    album_image_key: None,
                })
                .collect())
        }

        async fn fetch_artists(
            service: &SpotifyService,
            query: &str,
        ) -> Result<Vec<ArtistResult>, String> {
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
                .map(|item| ArtistResult {
                    id: item.id,
                    name: item.name,
                    image_url: item.images.first().map(|img| img.url.clone()),
                    image_key: None,
                })
                .collect())
        }

        async fn fetch_albums(
            service: &SpotifyService,
            query: &str,
        ) -> Result<Vec<AlbumResult>, String> {
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
                .map(|item| AlbumResult {
                    id: item.id,
                    name: item.name,
                    artist: item
                        .artists
                        .first()
                        .map(|artist| artist.name.clone())
                        .unwrap_or_else(|| "Unknown artist".to_string()),
                    release_date: item.release_date,
                    image_url: item.images.first().map(|img| img.url.clone()),
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

    #[allow(dead_code)]
    pub async fn get_recommendations(
        &self,
        seed_track_id: &str,
        limit: usize,
    ) -> Result<Vec<Track>, String> {
        let token = self.access_token()?;
        let bounded_limit = limit.clamp(1, 100);
        let limit_str = bounded_limit.to_string();

        let primary = self
            .http
            .get("https://api.spotify.com/v1/recommendations")
            .bearer_auth(token)
            .query(&[
                ("seed_tracks", seed_track_id),
                ("limit", limit_str.as_str()),
                ("market", "from_token"),
            ])
            .send()
            .await
            .map_err(|err| format!("Spotify recommendations failed: {err}"))?;

        let response = if primary.status().as_u16() == 404 {
            self.http
                .get("https://api.spotify.com/v1/recommendations")
                .bearer_auth(token)
                .query(&[
                    ("seed_genres", "pop"),
                    ("limit", limit_str.as_str()),
                    ("market", "from_token"),
                ])
                .send()
                .await
                .map_err(|err| format!("Spotify recommendations retry failed: {err}"))?
        } else {
            primary
        };

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "Spotify recommendations returned status {}: {}",
                status, body
            ));
        }

        let payload = response
            .json::<RecommendationsResponse>()
            .await
            .map_err(|err| format!("Invalid Spotify recommendations payload: {err}"))?;

        Ok(payload
            .tracks
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
                album_image_url: item.album.images.first().map(|img| img.url.clone()),
                album_image_key: None,
            })
            .collect())
    }
}