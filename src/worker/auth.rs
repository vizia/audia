use std::sync::{Arc, Mutex};

use vizia::prelude::ContextProxy;

use crate::oauth as oauth_api;
use crate::storage::{ClientCredentialStore, TokenStore};
use crate::ui::events::{PlaybackAppEvent, SystemAppEvent};

use super::{BackendState, SharedBackend, apply_token_response, emit_login_profile_event};

pub fn init_backend(proxy: ContextProxy) -> SharedBackend {
    let backend: SharedBackend = Arc::new(Mutex::new(BackendState::default()));
    let backend_clone = Arc::clone(&backend);

    proxy.spawn(move |proxy| {
        let runtime = {
            let state = backend_clone.lock().unwrap();
            state.runtime.clone()
        };

        if let Ok(Some(token)) = TokenStore::load() {
            {
                let mut state = backend_clone.lock().unwrap();
                state.spotify.set_access_token(token.access_token.clone());
                state.refresh_token = token.refresh_token.clone();
                state.token_expires_at = token.expires_at;
            }

            if let Ok(Some(creds)) = ClientCredentialStore::load() {
                let mut state = backend_clone.lock().unwrap();
                state.client_id = Some(creds.client_id);
            }

            let (needs_refresh, cid, rt) = {
                let state = backend_clone.lock().unwrap();
                (
                    state.token_needs_refresh(),
                    state.client_id.clone(),
                    state.refresh_token.clone(),
                )
            };

            if needs_refresh {
                if let (Some(cid), Some(rt)) = (cid, rt) {
                    match runtime.block_on(oauth_api::refresh_access_token(&cid, &rt)) {
                        Ok(tokens) => {
                            apply_token_response(
                                &backend_clone,
                                &tokens,
                                &cid,
                                proxy,
                                runtime.as_ref(),
                            );
                            let _ = proxy.emit(SystemAppEvent::Ready);
                            return;
                        }
                        Err(err) => {
                            let _ = proxy.emit(SystemAppEvent::Error(format!(
                                "Silent token refresh failed: {err}"
                            )));
                        }
                    }
                }
            } else {
                let valid = {
                    let state = backend_clone.lock().unwrap();
                    runtime
                        .block_on(state.spotify.validate_token())
                        .unwrap_or(false)
                };

                if valid {
                    emit_login_profile_event(&backend_clone, runtime.as_ref(), proxy);

                    let mut state = backend_clone.lock().unwrap();
                    if state
                        .playback
                        .bootstrap_from_access_token(runtime.as_ref(), &token.access_token)
                        .is_ok()
                    {
                        let _ = proxy.emit(PlaybackAppEvent::SessionReady);
                    }
                } else {
                    let _ = proxy.emit(SystemAppEvent::StatusMessage(
                        "Saved token is invalid. Please log in again.".to_string(),
                    ));
                }
            }
        }

        let _ = proxy.emit(SystemAppEvent::Ready);
    });

    backend
}
