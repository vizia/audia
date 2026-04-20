use reqwest::Client;

mod albums;
mod artists;
mod auth;
mod playback;
mod playlists;
mod profile;
mod search;
mod types;

pub struct SpotifyProfile {
    pub display_name: Option<String>,
    pub image_bytes: Option<Vec<u8>>,
}

pub struct SpotifyPlaylist {
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
    pub track_count: usize,
}

#[derive(Default, Clone)]
pub struct SpotifyService {
    pub(super) http: Client,
    pub(super) access_token: Option<String>,
}

impl SpotifyService {
    pub(super) fn access_token(&self) -> Result<&str, String> {
        self.access_token
            .as_deref()
            .ok_or_else(|| "No token provided".to_string())
    }
}
