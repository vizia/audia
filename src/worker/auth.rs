use std::sync::{Arc, Mutex};

use vizia::prelude::{Context, Task, TaskResult};

use crate::oauth as oauth_api;
use crate::storage::{ClientCredentialStore, TokenStore};
use crate::ui::events::{PlaybackEvents, SystemEvents};

use super::{
    BackendState, SharedBackend, apply_token_response, bootstrap_playback_from_token,
    emit_login_profile_event, lock_backend,
};

pub fn init_backend(cx: &Context) -> SharedBackend {
    let backend: SharedBackend = Arc::new(Mutex::new(BackendState::default()));
    let backend_clone = Arc::clone(&backend);
    let proxy = cx.get_proxy();

    cx.add_task(
        Task::new(move |_| {
            let mut proxy = proxy.clone();
            let backend_clone = backend_clone.clone();
            async move {
                if let Ok(Some(token)) = TokenStore::load() {
                    {
                        let mut state = match lock_backend(&backend_clone) {
                            Ok(state) => state,
                            Err(err) => {
                                let _ = proxy.emit(SystemEvents::Error(err));
                                let _ = proxy.emit(SystemEvents::Ready);
                                return Ok::<(), String>(());
                            }
                        };
                        state.spotify.set_access_token(token.access_token.clone());
                        state.refresh_token = token.refresh_token.clone();
                        state.token_expires_at = token.expires_at;
                    }

                    if let Ok(Some(creds)) = ClientCredentialStore::load() {
                        let mut state = match lock_backend(&backend_clone) {
                            Ok(state) => state,
                            Err(err) => {
                                let _ = proxy.emit(SystemEvents::Error(err));
                                let _ = proxy.emit(SystemEvents::Ready);
                                return Ok::<(), String>(());
                            }
                        };
                        state.client_id = Some(creds.client_id);
                    }

                    let (needs_refresh, cid, rt) = {
                        let state = match lock_backend(&backend_clone) {
                            Ok(state) => state,
                            Err(err) => {
                                let _ = proxy.emit(SystemEvents::Error(err));
                                let _ = proxy.emit(SystemEvents::Ready);
                                return Ok::<(), String>(());
                            }
                        };
                        (
                            state.token_needs_refresh(),
                            state.client_id.clone(),
                            state.refresh_token.clone(),
                        )
                    };

                    if needs_refresh {
                        if let (Some(cid), Some(rt)) = (cid, rt) {
                            match oauth_api::refresh_access_token(&cid, &rt).await {
                                Ok(tokens) => {
                                    if let Err(err) = apply_token_response(
                                        &backend_clone,
                                        &tokens,
                                        &cid,
                                        &mut proxy,
                                    )
                                    .await
                                    {
                                        let _ = proxy.emit(SystemEvents::Error(err));
                                    }
                                    let _ = proxy.emit(SystemEvents::Ready);
                                }
                                Err(err) => {
                                    let _ = proxy.emit(SystemEvents::Error(format!(
                                        "Silent token refresh failed: {err}"
                                    )));
                                    let _ = proxy.emit(SystemEvents::Ready);
                                }
                            }
                            return Ok::<(), String>(());
                        }
                    } else {
                        let spotify = match lock_backend(&backend_clone) {
                            Ok(state) => state.spotify.clone(),
                            Err(err) => {
                                let _ = proxy.emit(SystemEvents::Error(err));
                                let _ = proxy.emit(SystemEvents::Ready);
                                return Ok::<(), String>(());
                            }
                        };
                        let access_token = token.access_token.clone();

                        let valid = spotify.validate_token().await.unwrap_or(false);

                        if valid {
                            let profile = spotify.fetch_profile().await.ok();
                            emit_login_profile_event(profile, &mut proxy);

                            if bootstrap_playback_from_token(&backend_clone, &access_token)
                                .await
                                .is_ok()
                            {
                                let _ = proxy.emit(PlaybackEvents::SessionReady);
                            }
                        } else {
                            let _ = proxy.emit(SystemEvents::StatusMessage(
                                "Saved token is invalid. Please log in again.".to_string(),
                            ));
                        }

                        let _ = proxy.emit(SystemEvents::Ready);
                        return Ok::<(), String>(());
                    }
                }

                let _ = proxy.emit(SystemEvents::Ready);
                Ok::<(), String>(())
            }
        })
        .name("init-backend")
        .on_result(|result, proxy| {
            if let TaskResult::Error(err) = result {
                let _ = proxy.emit(SystemEvents::Error(err));
                let _ = proxy.emit(SystemEvents::Ready);
            }
        }),
    );

    backend
}
