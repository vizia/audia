use vizia::prelude::*;

use crate::{storage::configured_client_id, ui::events::OAuthEvent, worker};

#[derive(Clone)]
pub struct OAuthState {
    pub backend: crate::worker::SharedBackend,
    pub status: Signal<String>,
    pub auth_valid: Signal<bool>,
    pub show_login_modal: Signal<bool>,
    pub login_client_id_input: Signal<String>,
    pub auth_username: Signal<String>,
    pub profile_image_key: Signal<Option<String>>,
}

impl OAuthState {
    pub fn new(backend: crate::worker::SharedBackend, status: Signal<String>) -> Self {
        Self {
            backend,
            status,
            auth_valid: Signal::new(false),
            show_login_modal: Signal::new(false),
            login_client_id_input: Signal::new(String::new()),
            auth_username: Signal::new(String::new()),
            profile_image_key: Signal::new(None),
        }
    }
}

impl Model for OAuthState {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|oauth_event: &OAuthEvent, _| match oauth_event {
            OAuthEvent::BrowserOpened => {
                self.status
                    .set("Browser opened — complete login in your browser.".to_string());
            }
            OAuthEvent::LoginComplete {
                username,
                profile_image_key,
            } => {
                self.auth_valid.set(true);
                self.show_login_modal.set(false);
                self.auth_username.set(username.clone());
                self.profile_image_key.set(profile_image_key.clone());
                self.status.set(format!("Logged in as {}.", username));
                worker::refresh_user_playlists(self.backend.clone(), cx);
            }
            OAuthEvent::LoggedOut => {
                self.auth_valid.set(false);
                self.show_login_modal.set(true);
                self.auth_username.set(String::new());
                self.profile_image_key.set(None);
                self.status
                    .set("Logged out. Please log in again.".to_string());
            }
            _ => {}
        });

        event.map(|oauth_event: &OAuthEvent, _| match oauth_event {
            OAuthEvent::OpenLoginModal => {
                self.show_login_modal.set(true);
            }
            OAuthEvent::CloseLoginModal => {
                self.show_login_modal.set(false);
            }
            OAuthEvent::ResetLogin => {
                self.status.set("Resetting saved login...".to_string());
                worker::reset_login(self.backend.clone(), cx);
            }
            OAuthEvent::SetLoginClientId(client_id) => {
                self.login_client_id_input.set(client_id.clone());
            }
            OAuthEvent::StartOAuthLogin => {
                let typed_client_id = self.login_client_id_input.get();
                let typed_client_id = typed_client_id.trim();
                let client_id = if typed_client_id.is_empty() {
                    configured_client_id()
                } else {
                    Some(typed_client_id.to_string())
                };

                let Some(client_id) = client_id else {
                    self.status.set(
                        "No Spotify client ID is configured. Set AUDIA_SPOTIFY_CLIENT_ID or ship a saved app config."
                            .to_string(),
                    );
                    return;
                };

                self.status
                    .set("Opening Spotify authorization in browser...".to_string());
                worker::start_oauth_login(self.backend.clone(), client_id, cx);
            }
            OAuthEvent::RefreshToken => {
                self.status.set("Refreshing access token...".to_string());
                worker::refresh_access_token(self.backend.clone(), cx);
            }
            _ => {}
        });
    }
}
