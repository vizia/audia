use vizia::prelude::{EventContext, Task, TaskResult};

use crate::oauth as oauth_api;
use crate::storage::{ClientCredentialStore, clear_persisted_login};
use crate::ui::events::{OAuthEvents, SystemEvents};

use super::{
    SharedBackend, apply_token_response, lock_backend, lock_playback, set_oauth_in_progress,
    shared_playback,
};

pub fn start_oauth_login(backend: SharedBackend, client_id: String, cx: &EventContext<'_>) {
    let proxy = cx.get_proxy();
    cx.add_task(
        Task::new(move |_| {
            let mut proxy = proxy.clone();
            let backend = backend.clone();
            let client_id = client_id.clone();
            async move {
                {
                    let mut state = match lock_backend(&backend) {
                        Ok(state) => state,
                        Err(err) => {
                            let _ = proxy.emit(SystemEvents::Error(err));
                            return Ok::<(), String>(());
                        }
                    };
                    if state.oauth_in_progress {
                        let _ = proxy.emit(SystemEvents::StatusMessage(
                            "OAuth login is already in progress. Complete it in your browser."
                                .to_string(),
                        ));
                        return Ok::<(), String>(());
                    }
                    state.oauth_in_progress = true;
                }

                if let Err(err) = (ClientCredentialStore {
                    client_id: client_id.clone(),
                })
                .save()
                {
                    if let Err(lock_err) = set_oauth_in_progress(&backend, false) {
                        let _ = proxy.emit(SystemEvents::Error(lock_err));
                    }
                    let _ = proxy.emit(SystemEvents::Error(format!(
                        "Failed to save client credentials: {err}"
                    )));
                    return Ok::<(), String>(());
                }

                {
                    let mut state = match lock_backend(&backend) {
                        Ok(state) => state,
                        Err(err) => {
                            let _ = proxy.emit(SystemEvents::Error(err));
                            return Ok::<(), String>(());
                        }
                    };
                    state.client_id = Some(client_id.clone());
                }

                let state_token = oauth_api::generate_state();
                let code_verifier = oauth_api::generate_code_verifier();
                let challenge = oauth_api::code_challenge(&code_verifier);
                let url = oauth_api::auth_url(&client_id, &state_token, &challenge);

                if let Err(err) = webbrowser::open(&url) {
                    if let Err(lock_err) = set_oauth_in_progress(&backend, false) {
                        let _ = proxy.emit(SystemEvents::Error(lock_err));
                    }
                    let _ = proxy.emit(SystemEvents::Error(format!(
                        "Failed to open browser: {err}"
                    )));
                    return Ok::<(), String>(());
                }

                let _ = proxy.emit(OAuthEvents::BrowserOpened);
                let _ = proxy.emit(SystemEvents::StatusMessage(
                    "Waiting for OAuth callback from browser...".to_string(),
                ));

                let state_token_clone = state_token.clone();
                let code_result = tokio::task::spawn_blocking(move || {
                    oauth_api::wait_for_callback(&state_token_clone)
                })
                .await
                .map_err(|_| "OAuth callback worker panicked".to_string())
                .and_then(|result| result);

                let code = match code_result {
                    Ok(code) => code,
                    Err(err) => {
                        if let Err(lock_err) = set_oauth_in_progress(&backend, false) {
                            let _ = proxy.emit(SystemEvents::Error(lock_err));
                        }
                        let _ = proxy.emit(SystemEvents::Error(format!(
                            "OAuth callback error: {err}"
                        )));
                        return Ok::<(), String>(());
                    }
                };

                let _ = proxy.emit(SystemEvents::StatusMessage(
                    "OAuth callback received. Exchanging code for tokens...".to_string(),
                ));

                match oauth_api::exchange_code(&client_id, &code, &code_verifier).await {
                    Ok(tokens) => {
                        if let Err(err) =
                            apply_token_response(&backend, &tokens, &client_id, &mut proxy).await
                        {
                            let _ = proxy.emit(SystemEvents::Error(err));
                        }
                        if let Err(err) = set_oauth_in_progress(&backend, false) {
                            let _ = proxy.emit(SystemEvents::Error(err));
                        }
                    }
                    Err(err) => {
                        if let Err(lock_err) = set_oauth_in_progress(&backend, false) {
                            let _ = proxy.emit(SystemEvents::Error(lock_err));
                        }
                        let _ = proxy.emit(SystemEvents::Error(format!(
                            "Token exchange failed: {err}"
                        )));
                    }
                }

                Ok::<(), String>(())
            }
        })
        .name("start-oauth-login")
        .on_result(|result, proxy| {
            if let TaskResult::Error(err) = result {
                let _ = proxy.emit(SystemEvents::Error(err));
            }
        }),
    );
}

pub fn refresh_access_token(backend: SharedBackend, cx: &EventContext<'_>) {
    let proxy = cx.get_proxy();
    cx.add_task(
        Task::new(move |_| {
            let mut proxy = proxy.clone();
            let backend = backend.clone();
            async move {
                let (cid, rt) = {
                    let state = match lock_backend(&backend) {
                        Ok(state) => state,
                        Err(err) => {
                            let _ = proxy.emit(SystemEvents::Error(err));
                            return Ok::<(), String>(());
                        }
                    };
                    (state.client_id.clone(), state.refresh_token.clone())
                };

                match (cid, rt) {
                    (Some(cid), Some(rt)) => match oauth_api::refresh_access_token(&cid, &rt).await
                    {
                        Ok(tokens) => {
                            if let Err(err) =
                                apply_token_response(&backend, &tokens, &cid, &mut proxy).await
                            {
                                let _ = proxy.emit(SystemEvents::Error(err));
                            }
                        }
                        Err(err) => {
                            let _ = proxy.emit(SystemEvents::Error(format!(
                                "Token refresh failed: {err}"
                            )));
                        }
                    },
                    _ => {
                        let _ = proxy.emit(SystemEvents::Error(
                            "Cannot refresh: no client ID or refresh token stored.".to_string(),
                        ));
                    }
                }

                Ok::<(), String>(())
            }
        })
        .name("refresh-access-token")
        .on_result(|result, proxy| {
            if let TaskResult::Error(err) = result {
                let _ = proxy.emit(SystemEvents::Error(err));
            }
        }),
    );
}

pub fn reset_login(backend: SharedBackend, cx: &EventContext<'_>) {
    let proxy = cx.get_proxy();
    cx.add_task(
        Task::new(move |_| {
            let mut proxy = proxy.clone();
            let backend = backend.clone();
            async move {
                if let Err(err) = clear_persisted_login() {
                    let _ = proxy.emit(SystemEvents::Error(format!(
                        "Failed to clear persisted login: {err}"
                    )));
                    return Ok::<(), String>(());
                }

                {
                    let mut state = match lock_backend(&backend) {
                        Ok(state) => state,
                        Err(err) => {
                            let _ = proxy.emit(SystemEvents::Error(err));
                            return Ok::<(), String>(());
                        }
                    };
                    state.spotify.clear_access_token();
                    state.refresh_token = None;
                    state.client_id = None;
                    state.token_expires_at = None;
                }

                {
                    let playback = match shared_playback(&backend) {
                        Ok(playback) => playback,
                        Err(err) => {
                            let _ = proxy.emit(SystemEvents::Error(err));
                            return Ok::<(), String>(());
                        }
                    };
                    let mut state = match lock_playback(&playback) {
                        Ok(state) => state,
                        Err(err) => {
                            let _ = proxy.emit(SystemEvents::Error(err));
                            return Ok::<(), String>(());
                        }
                    };
                    state.reset();
                }

                let _ = proxy.emit(OAuthEvents::LoggedOut);
                Ok::<(), String>(())
            }
        })
        .name("reset-login")
        .on_result(|result, proxy| {
            if let TaskResult::Error(err) = result {
                let _ = proxy.emit(SystemEvents::Error(err));
            }
        }),
    );
}
