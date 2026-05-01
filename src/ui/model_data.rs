use crate::ui::data::{
    AlbumState, ArtistState, CenterState, OAuthState, PanelState, PlaybackState, PlaylistsState,
    PreferencesData, SearchState,
};
use crate::worker;
use vizia::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PlaybackTarget {
    Local,
    Remote(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CenterPage {
    Search,
    PlaylistTracks,
    AlbumTracks,
    Artist,
}

#[derive(Clone)]
pub struct UiModel {
    pub(crate) status: Signal<String>,
    pub(crate) oauth_state: OAuthState,
    pub(crate) preferences_data: PreferencesData,
    pub(crate) panel_state: PanelState,
    pub(crate) center_state: CenterState,
    pub(crate) playback_state: PlaybackState,
    pub(crate) search_state: SearchState,
    pub(crate) album_state: AlbumState,
    pub(crate) artist_state: ArtistState,
    pub(crate) playlists_state: PlaylistsState,
}

impl UiModel {
    pub fn new(
        cx: &mut Context,
        backend: crate::worker::SharedBackend,
        mut panel_state: PanelState,
    ) -> Self {
        let status = Signal::new("Initializing...".to_string());

        let artwork_fade_animation = cx.add_animation(
            AnimationBuilder::new()
                .keyframe(0.0, |key| key.opacity(1.0))
                .keyframe(1.0, |key| key.opacity(0.0)),
        );

        let mut preferences_data = PreferencesData::new(cx);
        preferences_data.load(cx);

        panel_state.load();

        let playback = worker::shared_playback(&backend);

        Self {
            status: status.clone(),
            oauth_state: OAuthState::new(backend.clone(), status.clone()),
            preferences_data: preferences_data,
            panel_state,
            center_state: CenterState::new(),
            playback_state: PlaybackState::new(
                backend.clone(),
                playback,
                status.clone(),
                artwork_fade_animation,
            ),
            search_state: SearchState::new(backend.clone(), status.clone()),
            album_state: AlbumState::new(),
            artist_state: ArtistState::new(backend.clone(), status.clone()),
            playlists_state: PlaylistsState::new(backend.clone(), status.clone()),
        }
    }
}
