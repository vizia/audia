use serde::Deserialize;

use super::{SpotifyProfile, SpotifyService, types::{SpotifyImage, pick_image_url}};

const PROFILE_IMAGE_TARGET_PX: u32 = 96;

impl SpotifyService {
    pub async fn fetch_profile(&self) -> Result<SpotifyProfile, String> {
        let token = self.access_token()?;

        #[derive(Deserialize)]
        struct MeResponse {
            display_name: Option<String>,
            images: Vec<SpotifyImage>,
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

        let image_url = pick_image_url(&me.images, PROFILE_IMAGE_TARGET_PX);

        let image_bytes = if let Some(url) = image_url {
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
                        image_response.bytes().await.ok().map(|bytes| bytes.to_vec())
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
}