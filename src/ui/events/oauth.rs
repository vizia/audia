#[derive(Clone, Debug)]
pub enum OAuthEvent {
    OpenLoginModal,
    CloseLoginModal,
    ResetLogin,
    SetLoginClientId(String),
    StartOAuthLogin,
    RefreshToken,
    BrowserOpened,
    LoginComplete {
        username: String,
        profile_image_key: Option<String>,
    },
    LoggedOut,
}
