use crate::messages::{AlbumResult, ArtistResult, PlaybackDevice, PlaylistEntry, Track};
use crate::ui::data::{
    OAuthState, PanelEvent, PanelState, PlaybackState, PlaylistsState, PreferencesData, SearchState,
};
use crate::ui::events::{OAuthUiEvent, PlaybackUiEvent};
use crate::worker;
use vizia::prelude::*;

pub mod data;
pub mod dialogs;
pub mod event_handling;
pub mod events;
pub mod model_data;
pub mod panels;

use model_data::{PlaybackTarget, UiModel};

pub fn run() {
    let icon = image::ImageReader::new(std::io::Cursor::new(include_bytes!(
        "../../resources/icons/icon_32.png"
    )))
    .with_guessed_format()
    .unwrap()
    .decode()
    .unwrap();

    let _ = Application::new(|cx| {
        cx.add_stylesheet(include_style!("resources/stylesheets/theme.css"))
            .expect("failed to load theme stylesheet");

        cx.add_translation(
            langid!("en-GB"),
            include_str!("../../resources/translations/en-GB/strings.ftl"),
        );

        let auth_valid = Signal::new(false);
        let playback_ready = Signal::new(false);
        let show_login_modal = Signal::new(true);
        let search_input = Signal::new(String::new());
        let auth_username = Signal::new(String::new());
        let profile_image_key = Signal::new(None::<String>);
        let playback_track_name = Signal::new("Nothing playing".to_string());
        let playback_track_artist = Signal::new(String::new());
        let playback_track_image_key = Signal::new(None::<String>);
        let playback_overlay_image_key = Signal::new(None::<String>);

        let artwork_fade_animation = cx.add_animation(
            AnimationBuilder::new()
                .keyframe(0.0, |key| key.opacity(1.0))
                .keyframe(1.0, |key| key.opacity(0.0)),
        );
        let status = Signal::new("Initialising...".to_string());
        let playback_devices = Signal::new(Vec::<PlaybackDevice>::new());
        let playback_device_options = Signal::new(vec!["Local Device [unavailable]".to_string()]);
        let selected_playback_device_index = Signal::new(Some(0usize));
        let selected_playback_target = Signal::new(Some(PlaybackTarget::Local));
        let playback_is_playing = Signal::new(false);
        let playback_volume = Signal::new(80.0f32);
        let playback_scrub_percent = Signal::new(0.0f32);
        let playback_duration_ms = Signal::new(0u32);
        let search_result_rows = Signal::new(Vec::<Track>::new());
        let search_artist_rows = Signal::new(Vec::<ArtistResult>::new());
        let search_album_rows = Signal::new(Vec::<AlbumResult>::new());
        let playlist_rows = Signal::new(Vec::<PlaylistEntry>::new());
        let playlist_tracks = Signal::new(Vec::<Track>::new());
        let filtered_playlist_tracks = Signal::new(Vec::<Track>::new());
        let filtered_track_indices = Signal::new(Vec::<usize>::new());
        let playlist_track_filter_input = Signal::new(String::new());
        let active_playlist_name = Signal::new(String::new());
        let active_playlist_meta = Signal::new(String::new());
        let showing_playlist = Signal::new(false);
        let playlist_selected_index = Signal::new(0usize);
        let queue_tracks = Signal::new(Vec::<Track>::new());
        let queue_current_index = Signal::new(None::<usize>);
        let recently_played = Signal::new(Vec::<Track>::new());
        let mut panel_state = PanelState::new(cx);
        panel_state.load();
        let left_panel_width = panel_state.left_width;
        let right_panel_width = panel_state.right_width;
        let selected_index = Signal::new(0usize);
        let selected_summary = Signal::new("Selected: none".to_string());
        let shuffle_mode = Signal::new(false);

        let backend = worker::init_backend(cx.get_proxy());

        let mut preferences_data = PreferencesData::new(cx);
        preferences_data.load(cx);
        dialogs::preferences_dialog(cx, icon, preferences_data);

        UiModel {
            status,
            oauth_state: OAuthState {
                backend: backend.clone(),
                status,
                auth_valid,
                show_login_modal,
                auth_username,
                profile_image_key,
            },
            preferences_data,
            panel_state,
            playback_state: PlaybackState {
                playback_track_name,
                playback_track_artist,
                playback_track_image_key,
                playback_overlay_image_key,
                last_remote_volume_sent: None,
                last_remote_volume_sent_at: None,
                last_remote_seek_sent_ms: None,
                last_remote_seek_sent_at: None,
                last_scrub_user_input_at: None,
                backend: backend.clone(),
                status,
                playback_devices,
                playback_device_options,
                selected_playback_device_index,
                playback_ready,
                playback_is_playing,
                queue_tracks,
                playback_scrub_percent,
                queue_current_index,
                selected_playback_target,
                playback_volume,
                recently_played,
                playback_duration_ms,
                artwork_fade_animation,
            },
            search_state: SearchState {
                backend: backend.clone(),
                status,
                search_input,
                search_result_rows,
                search_artist_rows,
                search_album_rows,
                selected_index,
                selected_summary,
                showing_playlist,
            },
            playlists_state: PlaylistsState {
                backend: backend.clone(),
                status,
                playlist_rows,
                playlist_tracks,
                filtered_playlist_tracks,
                filtered_track_indices,
                track_filter_input: playlist_track_filter_input,
                active_playlist_name,
                active_playlist_meta,
                showing_playlist,
                playlist_selected_index,
                shuffle_mode,
            },
        }
        .build(cx);

        worker::start_playback_progress_poller(backend.clone(), cx.get_proxy());

        dialogs::login_modal(cx, show_login_modal);

        // Main blurred background image layer
        Binding::new(cx, playback_track_image_key, move |cx| {
            if let Some(image_key) = playback_track_image_key.get() {
                ZStack::new(cx, |cx| {
                    Image::new(cx, image_key)
                        .size(Stretch(1.0))
                        .class("blurred-artwork");
                })
                .position_type(PositionType::Absolute)
                .top(Pixels(0.0))
                .right(Pixels(0.0))
                .bottom(Pixels(0.0))
                .left(Pixels(0.0))
                .pointer_events(PointerEvents::None);
            }
        });

        // Crossfade overlay for smooth image transitions
        Binding::new(cx, playback_overlay_image_key, move |cx| {
            if let Some(image_key) = playback_overlay_image_key.get() {
                ZStack::new(cx, |cx| {
                    Image::new(cx, image_key)
                        .size(Stretch(1.0))
                        .class("blurred-artwork");
                })
                .id("artwork-overlay")
                .position_type(PositionType::Absolute)
                .top(Pixels(0.0))
                .right(Pixels(0.0))
                .bottom(Pixels(0.0))
                .left(Pixels(0.0))
                .opacity(0.0)
                .pointer_events(PointerEvents::None);
            }
        });

        VStack::new(cx, |cx| {
            panels::header_panel(cx, search_input, auth_username, profile_image_key);

            HStack::new(cx, |cx| {
                Button::new(cx, |cx| Label::new(cx, "Open Login"))
                    .on_press(|cx| cx.emit(OAuthUiEvent::OpenLoginModal));

                Button::new(cx, |cx| Label::new(cx, "Refresh Token"))
                    .on_press(|cx| cx.emit(OAuthUiEvent::RefreshToken));

                Button::new(cx, |cx| Label::new(cx, "Refresh Devices"))
                    .on_press(|cx| cx.emit(PlaybackUiEvent::RefreshDevices));

                Button::new(cx, |cx| Label::new(cx, "Reset Login"))
                    .on_press(|cx| cx.emit(OAuthUiEvent::ResetLogin));
            })
            .height(Auto)
            .horizontal_gap(Pixels(8.0));

            Label::new(cx, selected_summary).class("status");
            Label::new(cx, status).class("status");

            HStack::new(cx, |cx| {
                ResizableStack::new(
                    cx,
                    left_panel_width.map(|w| Pixels(*w)),
                    ResizeStackDirection::Right,
                    |cx, w| {
                        cx.emit(PanelEvent::SetLeftPanelWidth(w));
                    },
                    move |cx| {
                        panels::playlists_panel(cx, playlist_rows);
                    },
                )
                .class("left-panel");

                Binding::new(cx, showing_playlist, move |cx| {
                    if showing_playlist.get() {
                        panels::playlist_tracks_panel(
                            cx,
                            active_playlist_name,
                            active_playlist_meta,
                            playlist_track_filter_input,
                            filtered_playlist_tracks,
                            playlist_selected_index,
                            shuffle_mode,
                        );
                    } else {
                        panels::search_results_panel(
                            cx,
                            search_result_rows,
                            search_artist_rows,
                            search_album_rows,
                            selected_index,
                        );
                    }
                });

                ResizableStack::new(
                    cx,
                    right_panel_width.map(|w| Pixels(*w)),
                    ResizeStackDirection::Left,
                    |cx, w| {
                        cx.emit(PanelEvent::SetRightPanelWidth(w));
                    },
                    move |cx| {
                        panels::queue_panel(cx, queue_tracks, queue_current_index, recently_played);
                    },
                )
                .class("right-panel");
            })
            .width(Stretch(1.0))
            .height(Stretch(1.0))
            .gap(Pixels(4.0));

            panels::playback_controls_panel(
                cx,
                playback_device_options,
                selected_playback_device_index,
                playback_is_playing,
                playback_volume,
                playback_scrub_percent,
                playback_duration_ms,
                playback_track_name,
                playback_track_artist,
                playback_track_image_key,
            );
        })
        .width(Stretch(1.0))
        .height(Stretch(1.0))
        .padding(Pixels(8.0))
        .gap(Pixels(8.0));
    })
    .title("Audia")
    .inner_size((760, 600))
    .run();
}
