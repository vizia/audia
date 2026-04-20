use super::SpotifyService;

impl SpotifyService {
    pub fn set_access_token(&mut self, token: String) {
        self.access_token = Some(token);
    }

    pub fn clear_access_token(&mut self) {
        self.access_token = None;
    }

    pub async fn validate_token(&self) -> Result<bool, String> {
        let token = self.access_token()?;

        let response = self
            .http
            .get("https://api.spotify.com/v1/me")
            .bearer_auth(token)
            .send()
            .await
            .map_err(|err| format!("Spotify request failed: {err}"))?;

        Ok(response.status().is_success())
    }
}