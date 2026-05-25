use crate::playback::PlaybackService;
use crate::ui::data::{
    AlbumState, ArtistState, CenterState, OAuthState, PanelState, PlaybackState, PlaylistsState,
    PreferencesData, RightPanelState, SearchState,
};
use crate::worker;
use std::sync::{Arc, Mutex};
use vizia::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CenterPage {
    Search,
    PlaylistTracks,
    AlbumTracks,
    Artist,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RightPanelPage {
    Queue,
    RecentlyPlayed,
}

#[derive(Clone)]
pub struct UiModel {
    pub(crate) status: Signal<String>,
    pub(crate) oauth_state: OAuthState,
    pub(crate) preferences_data: PreferencesData,
    pub(crate) panel_state: PanelState,
    pub(crate) center_state: CenterState,
    pub(crate) right_panel_state: RightPanelState,
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

        let mut preferences_data = PreferencesData::new();
        preferences_data.load(cx);

        panel_state.load();

        let playback = match worker::shared_playback(&backend) {
            Ok(playback) => playback,
            Err(err) => {
                status.set(format!("Initialization warning: {err}"));
                Arc::new(Mutex::new(PlaybackService::default()))
            }
        };

        let mut playback_state = PlaybackState::new(
            backend.clone(),
            playback,
            status.clone(),
            artwork_fade_animation,
        );

        // Share the same preference signals with playback state so UI and behavior stay in lockstep.
        playback_state.autoplay_on_queue_add = preferences_data.autoplay_on_queue_add;
        playback_state.restore_queue_on_startup = preferences_data.restore_queue_on_startup;

        // Restore queue if enabled
        playback_state.restore_queue();

        Self {
            status: status.clone(),
            oauth_state: OAuthState::new(backend.clone(), status.clone()),
            preferences_data: preferences_data,
            panel_state,
            center_state: CenterState::new(),
            right_panel_state: RightPanelState::new(),
            playback_state,
            search_state: SearchState::new(backend.clone(), status.clone()),
            album_state: AlbumState::new(),
            artist_state: ArtistState::new(backend.clone(), status.clone()),
            playlists_state: PlaylistsState::new(backend.clone(), status.clone()),
        }
    }
}
