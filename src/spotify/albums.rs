use serde::Deserialize;

use super::{
    SpotifyService,
    types::{SearchArtist, SpotifyImage, pick_image_url},
};
use crate::messages::{Album, Track};

const ALBUM_IMAGE_TARGET_PX: u32 = 160;

impl SpotifyService {
    pub async fn get_album_tracks(&self, album_id: &str) -> Result<Vec<Track>, String> {
        let token = self.access_token()?;
        let encoded_id = urlencoding::encode(album_id);
        let url = format!("https://api.spotify.com/v1/albums/{}/tracks", encoded_id);

        #[derive(Deserialize)]
        struct AlbumTracksResponse {
            items: Vec<AlbumTrackObject>,
            total: usize,
        }

        #[derive(Deserialize)]
        struct AlbumTrackObject {
            id: Option<String>,
            name: String,
            artists: Vec<SearchArtist>,
            duration_ms: u32,
            #[serde(rename = "type")]
            item_type: Option<String>,
        }

        let mut all_tracks = Vec::new();
        let mut offset = 0;
        const PAGE_SIZE: usize = 50;

        loop {
            let offset_str = offset.to_string();
            let response = self
                .http
                .get(&url)
                .bearer_auth(token)
                .query(&[
                    ("limit", PAGE_SIZE.to_string().as_str()),
                    ("offset", offset_str.as_str()),
                    ("market", "from_token"),
                ])
                .send()
                .await
                .map_err(|err| format!("Spotify album tracks request failed: {err}"))?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(format!(
                    "Spotify album tracks returned status {status}: {body}"
                ));
            }

            let payload = response
                .json::<AlbumTracksResponse>()
                .await
                .map_err(|err| format!("Invalid Spotify album tracks payload: {err}"))?;

            let page_size = payload.items.len();
            all_tracks.extend(payload.items.into_iter().filter_map(|item| {
                if item.item_type.as_deref() != Some("track") {
                    return None;
                }
                let track_id = item.id?;
                Some(Track {
                    id: track_id,
                    name: item.name,
                    artist: item
                        .artists
                        .into_iter()
                        .map(|artist| artist.name)
                        .collect::<Vec<_>>()
                        .join(", "),
                    duration_ms: item.duration_ms,
                    album_image_url: None,
                    album_playback_image_url: None,
                    album_image_key: None,
                })
            }));

            offset += page_size;
            if offset >= payload.total {
                break;
            }
        }

        Ok(all_tracks)
    }

    pub async fn get_album_for_track(&self, track_id: &str) -> Result<Album, String> {
        let token = self.access_token()?;
        let encoded_id = urlencoding::encode(track_id);
        let url = format!("https://api.spotify.com/v1/tracks/{}", encoded_id);

        #[derive(Deserialize)]
        struct TrackLookupResponse {
            album: TrackLookupAlbum,
        }

        #[derive(Deserialize)]
        struct TrackLookupAlbum {
            id: String,
            name: String,
            artists: Vec<SearchArtist>,
            release_date: Option<String>,
            images: Vec<SpotifyImage>,
        }

        let response = self
            .http
            .get(url)
            .bearer_auth(token)
            .query(&[("market", "from_token")])
            .send()
            .await
            .map_err(|err| format!("Spotify track lookup failed: {err}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "Spotify track lookup returned status {status}: {body}"
            ));
        }

        let payload = response
            .json::<TrackLookupResponse>()
            .await
            .map_err(|err| format!("Invalid Spotify track lookup payload: {err}"))?;

        let artist = payload
            .album
            .artists
            .first()
            .map(|artist| artist.name.clone())
            .unwrap_or_else(|| "Unknown artist".to_string());

        Ok(Album {
            id: payload.album.id,
            name: payload.album.name,
            artist,
            release_date: payload.album.release_date,
            image_url: pick_image_url(&payload.album.images, ALBUM_IMAGE_TARGET_PX),
            image_key: None,
        })
    }

    pub async fn get_album_release_year(&self, album_id: &str) -> Option<u32> {
        let token = self.access_token.as_deref()?;

        #[derive(Deserialize)]
        struct AlbumDetailsResponse {
            release_date: Option<String>,
        }

        let encoded_id = urlencoding::encode(album_id);
        let url = format!("https://api.spotify.com/v1/albums/{}", encoded_id);

        let response = self
            .http
            .get(url)
            .bearer_auth(token)
            .query(&[("fields", "release_date")])
            .send()
            .await
            .ok()?;

        if !response.status().is_success() {
            return None;
        }

        let payload = response.json::<AlbumDetailsResponse>().await.ok()?;

        payload
            .release_date
            .as_deref()
            .and_then(|date| date.split('-').next())
            .and_then(|year| year.parse::<u32>().ok())
    }
}
