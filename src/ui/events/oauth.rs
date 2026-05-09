#[derive(Clone, Debug)]
pub enum OAuthUiEvent {
    OpenLoginModal,
    CloseLoginModal,
    ResetLogin,
    SetLoginClientId(String),
    StartOAuthLogin,
    RefreshToken,
}

#[derive(Clone, Debug)]
pub enum OAuthAppEvent {
    BrowserOpened,
    LoginComplete {
        username: String,
        profile_image_key: Option<String>,
    },
    LoggedOut,
}
