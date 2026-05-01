use reqwest::header::CONTENT_LENGTH;
use serde::Serialize;

use super::{SpotifyService, types::CurrentPlaybackResponse};

impl SpotifyService {
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
}
