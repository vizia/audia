use crate::ui::data::{
    AlbumState, OAuthState, PanelState, PlaybackState, PlaylistsState, PreferencesData,
    SearchState,
};
use vizia::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PlaybackTarget {
    Local,
    Remote(String),
}

pub struct UiModel {
    pub(crate) status: Signal<String>,
    pub(crate) oauth_state: OAuthState,
    pub(crate) preferences_data: PreferencesData,
    pub(crate) panel_state: PanelState,
    pub(crate) playback_state: PlaybackState,
    pub(crate) search_state: SearchState,
    pub(crate) album_state: AlbumState,
    pub(crate) playlists_state: PlaylistsState,
}
