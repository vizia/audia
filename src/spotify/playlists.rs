use serde::Deserialize;

use super::{
    SpotifyPlaylist, SpotifyService,
    types::{PlaylistListResponse, SearchArtist, SpotifyImage},
};
use crate::messages::Track;

impl SpotifyService {
    pub async fn list_user_playlists(&self, limit: usize) -> Result<Vec<SpotifyPlaylist>, String> {
        let token = self.access_token()?;
        let bounded_limit = limit.clamp(1, 50);
        let limit_str = bounded_limit.to_string();

        let response = self
            .http
            .get("https://api.spotify.com/v1/me/playlists")
            .bearer_auth(token)
            .query(&[("limit", limit_str.as_str())])
            .send()
            .await
            .map_err(|err| format!("Spotify playlist list failed: {err}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "Spotify playlist list returned status {status}: {body}"
            ));
        }

        let payload = response
            .json::<PlaylistListResponse>()
            .await
            .map_err(|err| format!("Invalid Spotify playlist list payload: {err}"))?;

        Ok(payload
            .items
            .into_iter()
            .map(|playlist| SpotifyPlaylist {
                id: playlist.id,
                name: playlist.name,
                image_url: playlist.images.first().map(|img| img.url.clone()),
                track_count: playlist.tracks.and_then(|tracks| tracks.total).unwrap_or(0),
            })
            .collect())
    }

    pub async fn get_playlist_tracks(
        &self,
        playlist_id: &str,
        _limit: usize,
    ) -> Result<Vec<Track>, String> {
        let mut all_tracks = Vec::new();
        let mut offset = 0;
        const PAGE_SIZE: usize = 50;

        loop {
            let (mut page_tracks, total) = self
                .get_playlist_tracks_page(playlist_id, PAGE_SIZE, offset)
                .await?;

            let page_size = page_tracks.len();
            all_tracks.append(&mut page_tracks);

            offset += page_size;
            if offset >= total || page_size == 0 {
                break;
            }
        }

        Ok(all_tracks)
    }

    pub async fn get_playlist_tracks_page(
        &self,
        playlist_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<Track>, usize), String> {
        let token = self.access_token()?;
        let encoded_playlist_id = urlencoding::encode(playlist_id);
        let url = format!(
            "https://api.spotify.com/v1/playlists/{}/items",
            encoded_playlist_id
        );

        #[derive(Deserialize)]
        struct PlaylistTracksResponse {
            items: Vec<PlaylistTrackWrapper>,
            total: usize,
        }

        #[derive(Deserialize)]
        struct PlaylistTrackWrapper {
            item: Option<PlaylistTrackObject>,
            track: Option<PlaylistTrackObject>,
        }

        #[derive(Deserialize)]
        struct PlaylistTrackObject {
            #[serde(rename = "type")]
            item_type: Option<String>,
            id: Option<String>,
            name: String,
            artists: Vec<SearchArtist>,
            duration_ms: u32,
            album: PlaylistTrackAlbum,
        }

        #[derive(Deserialize)]
        struct PlaylistTrackAlbum {
            images: Vec<SpotifyImage>,
        }

        let bounded_limit = limit.clamp(1, 50);
        let limit_str = bounded_limit.to_string();
        let offset_str = offset.to_string();

        let response = self
            .http
            .get(&url)
            .bearer_auth(token)
            .query(&[
                ("limit", limit_str.as_str()),
                ("offset", offset_str.as_str()),
                ("market", "from_token"),
                ("additional_types", "track"),
                (
                    "fields",
                    "items(item(type,id,name,duration_ms,artists(name),album(images(url))),track(type,id,name,duration_ms,artists(name),album(images(url)))),total",
                ),
            ])
            .send()
            .await
            .map_err(|err| format!("Spotify playlist tracks request failed: {err}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            return Err(format!(
                "Spotify playlist tracks returned status {status}: {body}"
            ));
        }

        let payload = response
            .json::<PlaylistTracksResponse>()
            .await
            .map_err(|err| format!("Invalid Spotify playlist tracks payload: {err}"))?;

        let tracks = payload
            .items
            .into_iter()
            .filter_map(|wrapper| {
                let track = wrapper.item.or(wrapper.track)?;
                if track.item_type.as_deref() != Some("track") {
                    return None;
                }

                let track_id = track.id?;
                Some(Track {
                    id: track_id,
                    name: track.name,
                    artist: track
                        .artists
                        .into_iter()
                        .map(|artist| artist.name)
                        .collect::<Vec<_>>()
                        .join(", "),
                    duration_ms: track.duration_ms,
                    album_image_url: track.album.images.first().map(|img| img.url.clone()),
                    album_image_key: None,
                })
            })
            .collect::<Vec<_>>();

        Ok((tracks, payload.total))
    }
}
