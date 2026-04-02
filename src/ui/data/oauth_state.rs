use vizia::prelude::*;

use crate::{
    storage::configured_client_id,
    ui::events::{OAuthAppEvent, OAuthUiEvent},
    worker,
};

pub struct OAuthState {
    pub backend: crate::worker::SharedBackend,
    pub status: Signal<String>,
    pub auth_valid: Signal<bool>,
    pub show_login_modal: Signal<bool>,
    pub auth_username: Signal<String>,
    pub profile_image_key: Signal<Option<String>>,
}

impl Model for OAuthState {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|oauth_event: &OAuthAppEvent, _| match oauth_event {
            OAuthAppEvent::BrowserOpened => {
                self.status
                    .set("Browser opened — complete login in your browser.".to_string());
            }
            OAuthAppEvent::LoginComplete {
                username,
                profile_image_key,
            } => {
                self.auth_valid.set(true);
                self.show_login_modal.set(false);
                self.auth_username.set(username.clone());
                self.profile_image_key.set(profile_image_key.clone());
                self.status.set(format!("Logged in as {}.", username));
                worker::refresh_user_playlists(self.backend.clone(), cx.get_proxy());
            }
            OAuthAppEvent::LoggedOut => {
                self.auth_valid.set(false);
                self.show_login_modal.set(true);
                self.auth_username.set(String::new());
                self.profile_image_key.set(None);
                self.status
                    .set("Logged out. Please log in again.".to_string());
            }
        });

        event.map(|oauth_event: &OAuthUiEvent, _| match oauth_event {
            OAuthUiEvent::OpenLoginModal => {
                self.show_login_modal.set(true);
            }
            OAuthUiEvent::CloseLoginModal => {
                self.show_login_modal.set(false);
            }
            OAuthUiEvent::ResetLogin => {
                self.status.set("Resetting saved login...".to_string());
                worker::reset_login(self.backend.clone(), cx.get_proxy());
            }
            OAuthUiEvent::StartOAuthLogin => {
                let client_id = configured_client_id();

                let Some(client_id) = client_id else {
                    self.status.set(
                        "No Spotify client ID is configured. Set AUDIA_SPOTIFY_CLIENT_ID or ship a saved app config."
                            .to_string(),
                    );
                    return;
                };

                self.status
                    .set("Opening Spotify authorization in browser...".to_string());
                worker::start_oauth_login(self.backend.clone(), client_id, cx.get_proxy());
            }
            OAuthUiEvent::RefreshToken => {
                self.status.set("Refreshing access token...".to_string());
                worker::refresh_access_token(self.backend.clone(), cx.get_proxy());
            }
        });
    }
}
