use vizia::prelude::ContextProxy;

use crate::oauth as oauth_api;
use crate::storage::{ClientCredentialStore, clear_persisted_login};
use crate::ui::events::{OAuthAppEvent, SystemAppEvent};

use super::{SharedBackend, apply_token_response};

pub fn start_oauth_login(backend: SharedBackend, client_id: String, proxy: ContextProxy) {
    std::thread::spawn(move || {
        let runtime = {
            let state = backend.lock().unwrap();
            state.runtime.clone()
        };

        let mut proxy = proxy;

        {
            let mut state = backend.lock().unwrap();
            if state.oauth_in_progress {
                let _ = proxy.emit(SystemAppEvent::StatusMessage(
                    "OAuth login is already in progress. Complete it in your browser.".to_string(),
                ));
                return;
            }
            state.oauth_in_progress = true;
        }

        if let Err(err) = (ClientCredentialStore {
            client_id: client_id.clone(),
        })
        .save()
        {
            backend.lock().unwrap().oauth_in_progress = false;
            let _ = proxy.emit(SystemAppEvent::Error(format!(
                "Failed to save client credentials: {err}"
            )));
            return;
        }

        {
            let mut state = backend.lock().unwrap();
            state.client_id = Some(client_id.clone());
        }

        let state_token = oauth_api::generate_state();
        let code_verifier = oauth_api::generate_code_verifier();
        let challenge = oauth_api::code_challenge(&code_verifier);
        let url = oauth_api::auth_url(&client_id, &state_token, &challenge);

        if let Err(err) = webbrowser::open(&url) {
            backend.lock().unwrap().oauth_in_progress = false;
            let _ = proxy.emit(SystemAppEvent::Error(format!(
                "Failed to open browser: {err}"
            )));
            return;
        }

        let _ = proxy.emit(OAuthAppEvent::BrowserOpened);
        let _ = proxy.emit(SystemAppEvent::StatusMessage(
            "Waiting for OAuth callback from browser...".to_string(),
        ));

        let state_token_clone = state_token.clone();
        let code_result =
            std::thread::spawn(move || oauth_api::wait_for_callback(&state_token_clone))
                .join()
                .map_err(|_| "OAuth callback thread panicked".to_string())
                .and_then(|result| result);

        let code = match code_result {
            Ok(code) => code,
            Err(err) => {
                backend.lock().unwrap().oauth_in_progress = false;
                let _ = proxy.emit(SystemAppEvent::Error(format!(
                    "OAuth callback error: {err}"
                )));
                return;
            }
        };

        let _ = proxy.emit(SystemAppEvent::StatusMessage(
            "OAuth callback received. Exchanging code for tokens...".to_string(),
        ));

        runtime.spawn(async move {
            match oauth_api::exchange_code(&client_id, &code, &code_verifier).await {
                Ok(tokens) => {
                    apply_token_response(&backend, &tokens, &client_id, &mut proxy).await;
                    backend.lock().unwrap().oauth_in_progress = false;
                }
                Err(err) => {
                    backend.lock().unwrap().oauth_in_progress = false;
                    let _ = proxy.emit(SystemAppEvent::Error(format!(
                        "Token exchange failed: {err}"
                    )));
                }
            }
        });
    });
}

pub fn refresh_access_token(backend: SharedBackend, proxy: ContextProxy) {
    let runtime = {
        let state = backend.lock().unwrap();
        state.runtime.clone()
    };

    runtime.spawn(async move {
        let mut proxy = proxy;
        let (cid, rt) = {
            let state = backend.lock().unwrap();
            (state.client_id.clone(), state.refresh_token.clone())
        };

        match (cid, rt) {
            (Some(cid), Some(rt)) => match oauth_api::refresh_access_token(&cid, &rt).await {
                Ok(tokens) => {
                    apply_token_response(&backend, &tokens, &cid, &mut proxy).await;
                }
                Err(err) => {
                    let _ = proxy.emit(SystemAppEvent::Error(format!(
                        "Token refresh failed: {err}"
                    )));
                }
            },
            _ => {
                let _ = proxy.emit(SystemAppEvent::Error(
                    "Cannot refresh: no client ID or refresh token stored.".to_string(),
                ));
            }
        }
    });
}

pub fn reset_login(backend: SharedBackend, proxy: ContextProxy) {
    proxy.spawn(move |proxy| {
        if let Err(err) = clear_persisted_login() {
            let _ = proxy.emit(SystemAppEvent::Error(format!(
                "Failed to clear persisted login: {err}"
            )));
            return;
        }

        {
            let mut state = backend.lock().unwrap();
            state.spotify.clear_access_token();
            state.playback.reset();
            state.refresh_token = None;
            state.client_id = None;
            state.token_expires_at = None;
        }

        let _ = proxy.emit(OAuthAppEvent::LoggedOut);
    });
}
