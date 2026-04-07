use reqwest::Client;
use reqwest::header::CONTENT_LENGTH;
use serde::{Deserialize, Serialize};

use crate::messages::{AlbumResult, ArtistResult, PlaybackDevice, SearchResultsData, Track};

pub struct SpotifyProfile {
    pub display_name: Option<String>,
    pub image_bytes: Option<Vec<u8>>,
}

pub struct SpotifyPlaylist {
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
}

#[derive(Default, Clone)]
pub struct SpotifyService {
    http: Client,
    access_token: Option<String>,
}

impl SpotifyService {
    // Sets the current access token for Spotify API requests.
    pub fn set_access_token(&mut self, token: String) {
        self.access_token = Some(token);
    }

    // Clears the current access token, effectively logging out of Spotify.
    pub fn clear_access_token(&mut self) {
        self.access_token = None;
    }

    // Validates the current access token by making a test request to the Spotify API.
    pub async fn validate_token(&self) -> Result<bool, String> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| "No token provided".to_string())?;

        let response = self
            .http
            .get("https://api.spotify.com/v1/me")
            .bearer_auth(token)
            .send()
            .await
            .map_err(|err| format!("Spotify request failed: {err}"))?;

        Ok(response.status().is_success())
    }

    // Fetches the user's profile information, including display name and profile image.
    pub async fn fetch_profile(&self) -> Result<SpotifyProfile, String> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| "No token provided".to_string())?;

        #[derive(Deserialize)]
        struct MeImage {
            url: String,
        }

        #[derive(Deserialize)]
        struct MeResponse {
            display_name: Option<String>,
            images: Vec<MeImage>,
        }

        let response = self
            .http
            .get("https://api.spotify.com/v1/me")
            .bearer_auth(token)
            .send()
            .await
            .map_err(|err| format!("Spotify /me request failed: {err}"))?;

        if !response.status().is_success() {
            return Ok(SpotifyProfile {
                display_name: None,
                image_bytes: None,
            });
        }

        let me = response
            .json::<MeResponse>()
            .await
            .map_err(|err| format!("Invalid /me response: {err}"))?;

        let image_url = me.images.first().map(|image| image.url.clone());

        let image_bytes = if let Some(url) = image_url {
            // Spotify image URLs can be served from different hosts with varying auth behavior.
            // Try unauthenticated first, then fall back to bearer-auth if needed.
            let primary = self.http.get(url.clone()).send().await.ok();
            let response = match primary {
                Some(resp) if resp.status().is_success() => Some(resp),
                _ => self.http.get(url).bearer_auth(token).send().await.ok(),
            };

            if let Some(image_response) = response {
                if !image_response.status().is_success() {
                    None
                } else {
                    let content_type = image_response
                        .headers()
                        .get(reqwest::header::CONTENT_TYPE)
                        .and_then(|value| value.to_str().ok())
                        .map(|value| value.to_string());

                    let is_image = content_type
                        .as_deref()
                        .map(|value| value.starts_with("image/"))
                        .unwrap_or(false);

                    if !is_image {
                        None
                    } else {
                        image_response
                            .bytes()
                            .await
                            .ok()
                            .map(|bytes| bytes.to_vec())
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        Ok(SpotifyProfile {
            display_name: me.display_name,
            image_bytes,
        })
    }

    // Performs a search query against the Spotify catalog, returning matching tracks, artists, and albums.
    pub async fn search_catalog(&self, query: &str) -> Result<SearchResultsData, String> {
        async fn fetch_track_page(
            http: &Client,
            token: &str,
            query: &str,
            offset: u32,
        ) -> Result<Vec<Track>, String> {
            let offset = offset.to_string();
            let response = http
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
                        .map(|a| a.name.clone())
                        .unwrap_or_else(|| "Unknown artist".to_string()),
                    duration_ms: item.duration_ms,
                    album_image_url: item.album.images.first().map(|img| img.url.clone()),
                    album_image_key: None,
                })
                .collect::<Vec<_>>())
        }

        async fn fetch_artists(
            http: &Client,
            token: &str,
            query: &str,
        ) -> Result<Vec<ArtistResult>, String> {
            let response = http
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
                .collect::<Vec<_>>())
        }

        async fn fetch_albums(
            http: &Client,
            token: &str,
            query: &str,
        ) -> Result<Vec<AlbumResult>, String> {
            let response = http
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
                    image_url: item.images.first().map(|img| img.url.clone()),
                    image_key: None,
                })
                .collect::<Vec<_>>())
        }

        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| "No token provided".to_string())?
            .clone();

        let (tracks_page, artists, albums) = tokio::try_join!(
            fetch_track_page(&self.http, &token, query, 0),
            fetch_artists(&self.http, &token, query),
            fetch_albums(&self.http, &token, query),
        )?;

        let tracks = tracks_page;

        Ok(SearchResultsData {
            tracks,
            artists,
            albums,
        })
    }

    // Lists the available playback devices for the user.
    pub async fn list_playback_devices(&self) -> Result<Vec<PlaybackDevice>, String> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| "No token provided".to_string())?;

        let response = self
            .http
            .get("https://api.spotify.com/v1/me/player/devices")
            .bearer_auth(token)
            .send()
            .await
            .map_err(|err| format!("Spotify device list failed: {err}"))?;

        if !response.status().is_success() {
            return Err(format!(
                "Spotify device list returned status {}",
                response.status()
            ));
        }

        let payload = response
            .json::<DeviceListResponse>()
            .await
            .map_err(|err| format!("Invalid Spotify device payload: {err}"))?;

        Ok(payload
            .devices
            .into_iter()
            .map(|d| PlaybackDevice {
                id: d.id,
                name: d.name,
                is_active: d.is_active,
            })
            .collect())
    }

    // Lists the user's playlists with a specified limit.
    pub async fn list_user_playlists(&self, limit: usize) -> Result<Vec<SpotifyPlaylist>, String> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| "No token provided".to_string())?;

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
            .map(|p| SpotifyPlaylist {
                id: p.id,
                name: p.name,
                image_url: p.images.first().map(|img| img.url.clone()),
            })
            .collect())
    }

    // Fetches the tracks in a playlist (up to limit, max 50).
    pub async fn get_playlist_tracks(
        &self,
        playlist_id: &str,
        _limit: usize,
    ) -> Result<Vec<Track>, String> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| "No token provided".to_string())?;

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

            let page_size = payload.items.len();
            all_tracks.extend(payload.items.into_iter().filter_map(|wrapper| {
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
                        .map(|a| a.name)
                        .collect::<Vec<_>>()
                        .join(", "),
                    duration_ms: track.duration_ms,
                    album_image_url: track.album.images.first().map(|img| img.url.clone()),
                    album_image_key: None,
                })
            }));

            // Check if there are more tracks to fetch
            offset += page_size;
            if offset >= payload.total {
                break;
            }
        }

        Ok(all_tracks)
    }

    // Plays a specific track by its Spotify track ID.
    pub async fn playback_play_track(&self, track_id: &str) -> Result<(), String> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| "No token provided".to_string())?;

        let url = "https://api.spotify.com/v1/me/player/play";
        let uri = format!("spotify:track:{}", track_id);

        #[derive(Serialize)]
        struct PlayTrackRequest {
            uris: Vec<String>,
        }

        let response = self
            .http
            .put(url)
            .bearer_auth(token)
            .json(&PlayTrackRequest { uris: vec![uri] })
            .send()
            .await
            .map_err(|err| format!("Spotify play track failed: {err}"))?;

        if response.status().is_success() || response.status().as_u16() == 204 {
            return Ok(());
        }

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("Play track failed ({status}): {body}"))
    }

    // Resumes playback on the user's active device.
    pub async fn playback_resume(&self) -> Result<(), String> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| "No token provided".to_string())?;

        let response = self
            .http
            .put("https://api.spotify.com/v1/me/player/play")
            .bearer_auth(token)
            .header(CONTENT_LENGTH, "0")
            .body(String::new())
            .send()
            .await
            .map_err(|err| format!("Spotify resume failed: {err}"))?;

        if response.status().is_success() || response.status().as_u16() == 204 {
            return Ok(());
        }

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("Resume failed ({status}): {body}"))
    }

    // Pauses playback on the user's active device.
    pub async fn playback_pause(&self) -> Result<(), String> {
        self.playback_command("pause", reqwest::Method::PUT).await
    }

    // Skips to the next track on the user's active device.
    pub async fn playback_next(&self) -> Result<(), String> {
        self.playback_command("next", reqwest::Method::POST).await
    }

    // Skips to the previous track on the user's active device.
    pub async fn playback_previous(&self) -> Result<(), String> {
        self.playback_command("previous", reqwest::Method::POST)
            .await
    }

    // Sets the volume on the user's active device.
    pub async fn playback_set_volume(&self, volume_percent: u8) -> Result<(), String> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| "No token provided".to_string())?;

        let clamped = volume_percent.min(100).to_string();
        let response = self
            .http
            .put("https://api.spotify.com/v1/me/player/volume")
            .bearer_auth(token)
            .query(&[("volume_percent", clamped.as_str())])
            .header(CONTENT_LENGTH, "0")
            .body(String::new())
            .send()
            .await
            .map_err(|err| format!("Spotify set volume failed: {err}"))?;

        if response.status().is_success() || response.status().as_u16() == 204 {
            return Ok(());
        }

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("Set volume failed ({status}): {body}"))
    }

    // Seeks to a specific position in the currently playing track on the user's active device.
    pub async fn playback_seek(&self, position_ms: u32) -> Result<(), String> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| "No token provided".to_string())?;

        let position = position_ms.to_string();
        let response = self
            .http
            .put("https://api.spotify.com/v1/me/player/seek")
            .bearer_auth(token)
            .query(&[("position_ms", position.as_str())])
            .header(CONTENT_LENGTH, "0")
            .body(String::new())
            .send()
            .await
            .map_err(|err| format!("Spotify seek failed: {err}"))?;

        if response.status().is_success() || response.status().as_u16() == 204 {
            return Ok(());
        }

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("Seek failed ({status}): {body}"))
    }

    // Transfers playback to a specific device by its Spotify device ID.
    pub async fn transfer_playback(&self, device_id: &str) -> Result<(), String> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| "No token provided".to_string())?;

        #[derive(Serialize)]
        struct TransferRequest<'a> {
            device_ids: [&'a str; 1],
            play: bool,
        }

        let response = self
            .http
            .put("https://api.spotify.com/v1/me/player")
            .bearer_auth(token)
            .json(&TransferRequest {
                device_ids: [device_id],
                play: false,
            })
            .send()
            .await
            .map_err(|err| format!("Spotify transfer playback failed: {err}"))?;

        if response.status().is_success() || response.status().as_u16() == 204 {
            return Ok(());
        }

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("Transfer playback failed ({status}): {body}"))
    }

    // Retrieves the current playback progress, including position, duration, and playback state.
    pub async fn playback_progress(&self) -> Result<Option<(u32, u32, bool)>, String> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| "No token provided".to_string())?;

        let response = self
            .http
            .get("https://api.spotify.com/v1/me/player")
            .bearer_auth(token)
            .send()
            .await
            .map_err(|err| format!("Spotify playback state failed: {err}"))?;

        if response.status().as_u16() == 204 {
            return Ok(None);
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Playback state failed ({status}): {body}"));
        }

        let payload = response
            .json::<CurrentPlaybackResponse>()
            .await
            .map_err(|err| format!("Invalid playback state payload: {err}"))?;

        let Some(item) = payload.item else {
            return Ok(None);
        };

        Ok(Some((
            payload.progress_ms.unwrap_or(0),
            item.duration_ms,
            payload.is_playing,
        )))
    }

    // Sends a playback command (e.g., play, pause, next, previous) to the user's active device.
    async fn playback_command(&self, command: &str, method: reqwest::Method) -> Result<(), String> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| "No token provided".to_string())?;

        let url = format!("https://api.spotify.com/v1/me/player/{command}");

        let response = self
            .http
            .request(method, url)
            .bearer_auth(token)
            .header(CONTENT_LENGTH, "0")
            .body(String::new())
            .send()
            .await
            .map_err(|err| format!("Spotify playback command failed: {err}"))?;

        if response.status().is_success() || response.status().as_u16() == 204 {
            return Ok(());
        }

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("Playback command failed ({status}): {body}"))
    }

    // Retrieves a list of recommended tracks based on a seed track ID.
    #[allow(dead_code)]
    pub async fn get_recommendations(
        &self,
        seed_track_id: &str,
        limit: usize,
    ) -> Result<Vec<Track>, String> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| "No token provided".to_string())?;

        let bounded_limit = limit.clamp(1, 100);
        let limit_str = bounded_limit.to_string();

        // First try the documented form with market from the current user token.
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
            // Some accounts/apps reject track-seed recommendations; fall back to a genre-seed request.
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

        let tracks = payload
            .tracks
            .into_iter()
            .map(|item| Track {
                id: item.id,
                name: item.name,
                artist: item
                    .artists
                    .first()
                    .map(|a| a.name.clone())
                    .unwrap_or_else(|| "Unknown artist".to_string()),
                duration_ms: item.duration_ms,
                album_image_url: item.album.images.first().map(|img| img.url.clone()),
                album_image_key: None,
            })
            .collect();

        Ok(tracks)
    }
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    tracks: SearchTrackContainer,
}

#[derive(Debug, Deserialize)]
struct ArtistSearchResponse {
    artists: ArtistSearchContainer,
}

#[derive(Debug, Deserialize)]
struct ArtistSearchContainer {
    items: Vec<ArtistSearchItem>,
}

#[derive(Debug, Deserialize)]
struct ArtistSearchItem {
    id: String,
    name: String,
    images: Vec<SpotifyImage>,
}

#[derive(Debug, Deserialize)]
struct AlbumSearchResponse {
    albums: AlbumSearchContainer,
}

#[derive(Debug, Deserialize)]
struct AlbumSearchContainer {
    items: Vec<AlbumSearchItem>,
}

#[derive(Debug, Deserialize)]
struct AlbumSearchItem {
    id: String,
    name: String,
    artists: Vec<SearchArtist>,
    images: Vec<SpotifyImage>,
}

#[derive(Debug, Deserialize)]
struct SearchTrackContainer {
    items: Vec<SearchTrackItem>,
}

#[derive(Debug, Deserialize)]
struct SearchTrackItem {
    id: String,
    name: String,
    artists: Vec<SearchArtist>,
    duration_ms: u32,
    album: AlbumSummary,
}

#[derive(Debug, Deserialize)]
struct AlbumSummary {
    images: Vec<SpotifyImage>,
}

#[derive(Debug, Deserialize)]
struct SpotifyImage {
    url: String,
}

#[derive(Debug, Deserialize)]
struct SearchArtist {
    name: String,
}

#[derive(Debug, Deserialize)]
struct DeviceListResponse {
    devices: Vec<DeviceItem>,
}

#[derive(Debug, Deserialize)]
struct DeviceItem {
    id: Option<String>,
    name: String,
    is_active: bool,
}

#[derive(Debug, Deserialize)]
struct PlaylistListResponse {
    items: Vec<PlaylistItem>,
}

#[derive(Debug, Deserialize)]
struct PlaylistItem {
    id: String,
    name: String,
    images: Vec<SpotifyImage>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RecommendationsResponse {
    tracks: Vec<RecommendationTrackItem>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RecommendationTrackItem {
    id: String,
    name: String,
    artists: Vec<SearchArtist>,
    duration_ms: u32,
    album: AlbumSummary,
}

#[derive(Debug, Deserialize)]
struct CurrentPlaybackResponse {
    is_playing: bool,
    progress_ms: Option<u32>,
    item: Option<PlaybackItem>,
}

#[derive(Debug, Deserialize)]
struct PlaybackItem {
    duration_ms: u32,
}
