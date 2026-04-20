use reqwest::header::CONTENT_LENGTH;
use serde::Serialize;

use super::{SpotifyService, types::{CurrentPlaybackResponse, DeviceListResponse}};
use crate::messages::PlaybackDevice;

impl SpotifyService {
    pub async fn list_playback_devices(&self) -> Result<Vec<PlaybackDevice>, String> {
        let token = self.access_token()?;

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
            .map(|device| PlaybackDevice {
                id: device.id,
                name: device.name,
                is_active: device.is_active,
            })
            .collect())
    }

    pub async fn playback_play_track(&self, track_id: &str) -> Result<(), String> {
        let token = self.access_token()?;
        let uri = format!("spotify:track:{}", track_id);

        #[derive(Serialize)]
        struct PlayTrackRequest {
            uris: Vec<String>,
        }

        let response = self
            .http
            .put("https://api.spotify.com/v1/me/player/play")
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

    pub async fn playback_resume(&self) -> Result<(), String> {
        let token = self.access_token()?;

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

    pub async fn playback_pause(&self) -> Result<(), String> {
        self.playback_command("pause", reqwest::Method::PUT).await
    }

    pub async fn playback_stop(&self) -> Result<(), String> {
        self.playback_pause().await
    }

    pub async fn playback_next(&self) -> Result<(), String> {
        self.playback_command("next", reqwest::Method::POST).await
    }

    pub async fn playback_previous(&self) -> Result<(), String> {
        self.playback_command("previous", reqwest::Method::POST).await
    }

    pub async fn playback_set_volume(&self, volume_percent: u8) -> Result<(), String> {
        let token = self.access_token()?;
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

    pub async fn playback_seek(&self, position_ms: u32) -> Result<(), String> {
        let token = self.access_token()?;
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

    pub async fn transfer_playback(&self, device_id: &str) -> Result<(), String> {
        let token = self.access_token()?;

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

    pub async fn playback_progress(&self) -> Result<Option<(u32, u32, bool)>, String> {
        let token = self.access_token()?;

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

    async fn playback_command(&self, command: &str, method: reqwest::Method) -> Result<(), String> {
        let token = self.access_token()?;
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
}