use serde::Deserialize;

use super::{
    SpotifyService,
    types::{SearchArtist, SpotifyImage},
};
use crate::messages::{Album, Artist};

const ARTIST_ALBUMS_PAGE_SIZE: usize = 10;

fn sort_and_dedup_albums(albums: &mut Vec<Album>) {
    albums.sort_by(|a, b| {
        b.release_date
            .cmp(&a.release_date)
            .then_with(|| a.name.cmp(&b.name))
    });
    albums.dedup_by(|a, b| a.id == b.id);
}

impl SpotifyService {
    pub async fn get_artist(&self, artist_id: &str) -> Result<Artist, String> {
        let token = self.access_token()?;
        let encoded_id = urlencoding::encode(artist_id);
        let url = format!("https://api.spotify.com/v1/artists/{}", encoded_id);

        #[derive(Deserialize)]
        struct ArtistResponse {
            id: String,
            name: String,
            #[serde(default)]
            images: Vec<SpotifyImage>,
        }

        let response = self
            .http
            .get(url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|err| format!("Spotify artist lookup failed: {err}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "Spotify artist lookup returned status {status}: {body}"
            ));
        }

        let payload = response
            .json::<ArtistResponse>()
            .await
            .map_err(|err| format!("Invalid Spotify artist lookup payload: {err}"))?;

        Ok(Artist {
            id: payload.id,
            name: payload.name,
            image_url: payload.images.first().map(|img| img.url.clone()),
            image_key: None,
        })
    }

    pub async fn get_primary_artist_for_track(&self, track_id: &str) -> Result<Artist, String> {
        let token = self.access_token()?;
        let encoded_id = urlencoding::encode(track_id);
        let url = format!("https://api.spotify.com/v1/tracks/{}", encoded_id);

        #[derive(Deserialize)]
        struct TrackLookupResponse {
            artists: Vec<TrackArtist>,
        }

        #[derive(Deserialize)]
        struct TrackArtist {
            id: String,
            name: String,
        }

        let response = self
            .http
            .get(url)
            .bearer_auth(token)
            .query(&[("market", "from_token")])
            .send()
            .await
            .map_err(|err| format!("Spotify track lookup for artist failed: {err}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "Spotify track lookup for artist returned status {status}: {body}"
            ));
        }

        let payload = response
            .json::<TrackLookupResponse>()
            .await
            .map_err(|err| format!("Invalid Spotify track lookup payload: {err}"))?;

        let Some(primary_artist) = payload.artists.into_iter().next() else {
            return Err("Track has no associated artist in Spotify response.".to_string());
        };

        Ok(Artist {
            id: primary_artist.id,
            name: primary_artist.name,
            image_url: None,
            image_key: None,
        })
    }

    pub async fn get_artist_albums(&self, artist_id: &str) -> Result<Vec<Album>, String> {
        let mut albums = Vec::new();
        let mut offset = 0usize;

        loop {
            let (mut page_albums, total) = self
                .get_artist_albums_page(artist_id, ARTIST_ALBUMS_PAGE_SIZE, offset)
                .await?;

            let page_size = page_albums.len();
            albums.append(&mut page_albums);

            offset += page_size;
            if offset >= total || page_size == 0 {
                break;
            }
        }

        sort_and_dedup_albums(&mut albums);
        Ok(albums)
    }

    pub async fn get_artist_albums_page(
        &self,
        artist_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<Album>, usize), String> {
        let token = self.access_token()?;
        let encoded_id = urlencoding::encode(artist_id);
        let url = format!("https://api.spotify.com/v1/artists/{}/albums", encoded_id);

        #[derive(Deserialize)]
        struct ArtistAlbumsResponse {
            items: Vec<ArtistAlbumItem>,
            total: usize,
        }

        #[derive(Deserialize)]
        struct ArtistAlbumItem {
            id: String,
            name: String,
            artists: Vec<SearchArtist>,
            release_date: Option<String>,
            #[serde(default)]
            images: Vec<SpotifyImage>,
            #[serde(default)]
            album_group: Option<String>,
        }

        let offset_str = offset.to_string();
        let bounded_limit = limit.clamp(1, ARTIST_ALBUMS_PAGE_SIZE);
        let limit_str = bounded_limit.to_string();
        let response = self
            .http
            .get(&url)
            .bearer_auth(token)
            .query(&[
                ("include_groups", "album"),
                ("limit", limit_str.as_str()),
                ("offset", offset_str.as_str()),
            ])
            .send()
            .await
            .map_err(|err| format!("Spotify artist albums request failed: {err}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "Spotify artist albums returned status {status}: {body}"
            ));
        }

        let payload = response
            .json::<ArtistAlbumsResponse>()
            .await
            .map_err(|err| format!("Invalid Spotify artist albums payload: {err}"))?;

        let albums = payload
            .items
            .into_iter()
            .filter_map(|item| {
                if item.album_group.as_deref() == Some("appears_on") {
                    return None;
                }

                Some(Album {
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
            })
            .collect::<Vec<_>>();

        Ok((albums, payload.total))
    }
}
