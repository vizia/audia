use serde::{Deserialize, Serialize};

use super::{
    SpotifyPlaylist, SpotifyService,
    types::{PlaylistListResponse, SearchArtist, SpotifyImage},
};
use crate::messages::Track;

impl SpotifyService {
    async fn ensure_playlist_writable(&self, playlist_id: &str) -> Result<(), String> {
        #[derive(Deserialize)]
        struct PlaylistOwner {
            id: String,
        }

        #[derive(Deserialize)]
        struct PlaylistPermissions {
            owner: Option<PlaylistOwner>,
            collaborative: Option<bool>,
        }

        let current_user_id = self.current_user_id().await?;
        let token = self.access_token()?;
        let url = format!(
            "https://api.spotify.com/v1/playlists/{}",
            urlencoding::encode(playlist_id)
        );

        let response = self
            .http
            .get(&url)
            .bearer_auth(token)
            .query(&[("fields", "owner(id),collaborative")])
            .send()
            .await
            .map_err(|err| format!("Spotify playlist permission check failed: {err}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "Spotify playlist permission check returned status {status}: {body}"
            ));
        }

        let payload = response
            .json::<PlaylistPermissions>()
            .await
            .map_err(|err| format!("Invalid Spotify playlist permission payload: {err}"))?;

        let is_owner = payload
            .owner
            .as_ref()
            .map(|owner| owner.id.as_str() == current_user_id.as_str())
            .unwrap_or(false);
        let is_collaborative = payload.collaborative.unwrap_or(false);

        if !is_owner && !is_collaborative {
            return Err(
                "Playlist is not writable by this account (not owner and not collaborative)."
                    .to_string(),
            );
        }

        Ok(())
    }

    async fn current_user_id(&self) -> Result<String, String> {
        #[derive(Deserialize)]
        struct MeResponse {
            id: String,
        }

        let token = self.access_token()?;
        let response = self
            .http
            .get("https://api.spotify.com/v1/me")
            .bearer_auth(token)
            .send()
            .await
            .map_err(|err| format!("Spotify /me request failed while listing playlists: {err}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "Spotify /me returned status {status} while listing playlists: {body}"
            ));
        }

        response
            .json::<MeResponse>()
            .await
            .map(|me| me.id)
            .map_err(|err| format!("Invalid Spotify /me payload while listing playlists: {err}"))
    }

    pub async fn list_user_playlists(&self, limit: usize) -> Result<Vec<SpotifyPlaylist>, String> {
        let current_user_id = self.current_user_id().await?;
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

        let body = response
            .text()
            .await
            .map_err(|err| format!("Failed to read Spotify playlist list body: {err}"))?;

        let payload = serde_json::from_str::<PlaylistListResponse>(&body)
            .map_err(|err| format!("Invalid Spotify playlist list payload: {err}\nBody: {body}"))?;

        Ok(payload
            .items
            .into_iter()
            .flatten()
            .filter_map(|playlist| {
                let id = playlist.id?;
                let is_owner = playlist
                    .owner
                    .as_ref()
                    .map(|owner| owner.id.as_str() == current_user_id.as_str())
                    .unwrap_or(false);
                let is_collaborative = playlist.collaborative.unwrap_or(false);
                let is_public = playlist.public.unwrap_or(false);
                let _needs_public_modify_scope = is_owner && is_public;

                // Spotify returns followed playlists too; keep only likely-writable targets.
                if !is_owner && !is_collaborative {
                    return None;
                }

                let name = playlist.name.unwrap_or_default();
                Some(SpotifyPlaylist {
                    id,
                    name,
                    image_url: playlist.images.first().map(|img| img.url.clone()),
                    track_count: playlist.tracks.and_then(|tracks| tracks.total).unwrap_or(0),
                })
            })
            .collect())
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

    pub async fn create_playlist(&self, name: &str) -> Result<SpotifyPlaylist, String> {
        #[derive(Serialize)]
        struct CreatePlaylistRequest<'a> {
            name: &'a str,
            public: bool,
            collaborative: bool,
        }

        let token = self.access_token()?;

        let create_response = self
            .http
            .post("https://api.spotify.com/v1/me/playlists")
            .bearer_auth(token)
            .json(&CreatePlaylistRequest {
                name,
                public: false,
                collaborative: false,
            })
            .send()
            .await
            .map_err(|err| format!("Spotify create playlist request failed: {err}"))?;

        if !create_response.status().is_success() {
            let status = create_response.status();
            let body = create_response.text().await.unwrap_or_default();
            return Err(format!(
                "Spotify create playlist returned status {status}: {body}"
            ));
        }

        let playlist = create_response
            .json::<super::types::PlaylistItem>()
            .await
            .map_err(|err| format!("Invalid Spotify create playlist payload: {err}"))?;

        let id = playlist
            .id
            .ok_or_else(|| "Spotify create playlist returned no playlist ID".to_string())?;
        let name = playlist.name.unwrap_or_default();

        Ok(SpotifyPlaylist {
            id,
            name,
            image_url: playlist.images.first().map(|img| img.url.clone()),
            track_count: playlist.tracks.and_then(|tracks| tracks.total).unwrap_or(0),
        })
    }

    pub async fn rename_playlist(&self, playlist_id: &str, name: &str) -> Result<(), String> {
        #[derive(Serialize)]
        struct RenameRequest<'a> {
            name: &'a str,
        }

        let token = self.access_token()?;
        let url = format!(
            "https://api.spotify.com/v1/playlists/{}",
            urlencoding::encode(playlist_id)
        );

        let response = self
            .http
            .put(&url)
            .bearer_auth(token)
            .json(&RenameRequest { name })
            .send()
            .await
            .map_err(|err| format!("Spotify rename playlist request failed: {err}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "Spotify rename playlist returned status {status}: {body}"
            ));
        }

        Ok(())
    }

    pub async fn unfollow_playlist(&self, playlist_id: &str) -> Result<(), String> {
        let token = self.access_token()?;
        let url = format!(
            "https://api.spotify.com/v1/playlists/{}/followers",
            urlencoding::encode(playlist_id)
        );

        let response = self
            .http
            .delete(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|err| format!("Spotify unfollow playlist request failed: {err}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "Spotify unfollow playlist returned status {status}: {body}"
            ));
        }

        Ok(())
    }

    pub async fn add_tracks_to_playlist(
        &self,
        playlist_id: &str,
        track_uris: Vec<String>,
    ) -> Result<(), String> {
        #[derive(Serialize)]
        struct AddTracksRequest {
            uris: Vec<String>,
        }

        #[derive(Deserialize)]
        struct AddTracksResponse {
            snapshot_id: String,
        }

        if track_uris.is_empty() {
            return Err("Spotify add tracks requires at least one track URI".to_string());
        }

        if track_uris.len() > 100 {
            return Err("Spotify add tracks supports up to 100 track URIs per request".to_string());
        }

        self.ensure_playlist_writable(playlist_id).await?;

        let token = self.access_token()?;
        let url = format!(
            "https://api.spotify.com/v1/playlists/{}/items",
            urlencoding::encode(playlist_id)
        );

        let response = self
            .http
            .post(&url)
            .bearer_auth(token)
            .json(&AddTracksRequest { uris: track_uris })
            .send()
            .await
            .map_err(|err| format!("Spotify add tracks request failed: {err}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "Spotify add tracks returned status {status}: {body}"
            ));
        }

        let payload = response
            .json::<AddTracksResponse>()
            .await
            .map_err(|err| format!("Invalid Spotify add tracks payload: {err}"))?;

        if payload.snapshot_id.is_empty() {
            return Err("Spotify add tracks returned an empty snapshot_id".to_string());
        }

        Ok(())
    }

    pub async fn remove_tracks_from_playlist(
        &self,
        playlist_id: &str,
        track_uris: Vec<String>,
    ) -> Result<(), String> {
        #[derive(Serialize)]
        struct RemoveTrackItem {
            uri: String,
        }

        #[derive(Serialize)]
        struct RemoveTracksRequest {
            items: Vec<RemoveTrackItem>,
        }

        #[derive(Deserialize)]
        struct RemoveTracksResponse {
            snapshot_id: String,
        }

        if track_uris.is_empty() {
            return Err("Spotify remove tracks requires at least one track URI".to_string());
        }

        self.ensure_playlist_writable(playlist_id).await?;

        let token = self.access_token()?;
        let url = format!(
            "https://api.spotify.com/v1/playlists/{}/items",
            urlencoding::encode(playlist_id)
        );

        let items = track_uris
            .into_iter()
            .map(|uri| RemoveTrackItem { uri })
            .collect::<Vec<_>>();

        let response = self
            .http
            .delete(&url)
            .bearer_auth(token)
            .json(&RemoveTracksRequest { items })
            .send()
            .await
            .map_err(|err| format!("Spotify remove tracks request failed: {err}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "Spotify remove tracks returned status {status}: {body}"
            ));
        }

        let payload = response
            .json::<RemoveTracksResponse>()
            .await
            .map_err(|err| format!("Invalid Spotify remove tracks payload: {err}"))?;

        if payload.snapshot_id.is_empty() {
            return Err("Spotify remove tracks returned an empty snapshot_id".to_string());
        }

        Ok(())
    }
}
