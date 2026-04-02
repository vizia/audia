use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const DEFAULT_SPOTIFY_CLIENT_ID: &str = "1db90c8f99ab424cb22c69af1ca9c242";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TokenStore {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_at: Option<u64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ClientCredentialStore {
    pub client_id: String,
}

impl TokenStore {
    pub fn load() -> io::Result<Option<Self>> {
        let path = token_file_path()?;
        if !path.exists() {
            return Ok(None);
        }

        let data = fs::read_to_string(path)?;
        let token = serde_json::from_str::<TokenStore>(&data)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        Ok(Some(token))
    }

    pub fn save(&self) -> io::Result<()> {
        let path = token_file_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let data = serde_json::to_string_pretty(self)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        fs::write(path, data)
    }
}


fn token_file_path() -> io::Result<PathBuf> {
    let base = dirs::config_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Config directory unavailable"))?;
    Ok(base.join("audia").join("token.json"))
}

fn client_credential_file_path() -> io::Result<PathBuf> {
    let base = dirs::config_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Config directory unavailable"))?;
    Ok(base.join("audia").join("client_credentials.json"))
}

impl ClientCredentialStore {
    pub fn load() -> io::Result<Option<Self>> {
        let path = client_credential_file_path()?;
        if !path.exists() {
            return Ok(None);
        }
        let data = fs::read_to_string(path)?;
        serde_json::from_str::<ClientCredentialStore>(&data)
            .map(Some)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }

    pub fn save(&self) -> io::Result<()> {
        let path = client_credential_file_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        fs::write(path, data)
    }
}

pub fn clear_persisted_login() -> io::Result<()> {
    let token_path = token_file_path()?;
    let client_path = client_credential_file_path()?;

    match fs::remove_file(&token_path) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }

    match fs::remove_file(&client_path) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }

    Ok(())
}

pub fn configured_client_id() -> Option<String> {
    if let Some(client_id) = option_env!("AUDIA_SPOTIFY_CLIENT_ID") {
        let client_id = client_id.trim();
        if !client_id.is_empty() {
            return Some(client_id.to_string());
        }
    }

    if let Ok(client_id) = std::env::var("AUDIA_SPOTIFY_CLIENT_ID") {
        let client_id = client_id.trim().to_string();
        if !client_id.is_empty() {
            return Some(client_id);
        }
    }

    if !DEFAULT_SPOTIFY_CLIENT_ID.is_empty() {
        return Some(DEFAULT_SPOTIFY_CLIENT_ID.to_string());
    }

    ClientCredentialStore::load()
        .ok()
        .flatten()
        .map(|credentials| credentials.client_id)
        .filter(|client_id| !client_id.trim().is_empty())
}
