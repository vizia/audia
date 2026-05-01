use crate::messages::{AlbumResult, ArtistResult, PlaybackDevice, PlaylistEntry, Track};
use crate::ui::data::{
    AlbumState, ArtistState, CenterState, OAuthState, PanelEvent, PanelState, PlaybackState,
    PlaylistsState, PreferencesData, SearchState,
};
use crate::worker;
use vizia::prelude::*;

pub mod data;
pub mod dialogs;
pub mod event_handling;
pub mod events;
pub mod model_data;
pub mod panels;

use model_data::{CenterPage, PlaybackTarget, UiModel};

pub fn run() {
    let icon = image::ImageReader::new(std::io::Cursor::new(include_bytes!(
        "../../resources/icons/icon_32.png"
    )))
    .with_guessed_format()
    .unwrap()
    .decode()
    .unwrap();

    let mut panel_state = PanelState::new();
    panel_state.load();

    let _ = Application::new(move |cx| {
        cx.add_stylesheet(include_style!("resources/stylesheets/theme.css"))
            .expect("Failed to load theme stylesheet");

        cx.add_translation(
            langid!("en-GB"),
            include_str!("../../resources/translations/en-GB/strings.ftl"),
        )
        .expect("Failed to load en-GB translation");

        let auth_valid = Signal::new(false);
        let playback_ready = Signal::new(false);
        let show_login_modal = Signal::new(true);
        let search_input = Signal::new(String::new());
        let auth_username = Signal::new(String::new());
        let profile_image_key = Signal::new(None::<String>);
        let playback_track_name = Signal::new("Nothing playing".to_string());
        let playback_track_artist = Signal::new(String::new());
        let playback_track_id = Signal::new(None::<String>);
        let playback_track_image_key = Signal::new(None::<String>);
        let playback_track_image_url = Signal::new(None::<String>);
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
        let playback_is_muted = Signal::new(false);
        let playback_scrub_percent = Signal::new(0.0f32);
        let playback_duration_ms = Signal::new(0u32);
        let search_result_rows = Signal::new(Vec::<Track>::new());
        let search_artist_rows = Signal::new(Vec::<ArtistResult>::new());
        let search_album_rows = Signal::new(Vec::<AlbumResult>::new());
        let search_tabs = Signal::new(vec!["Songs", "Artists", "Albums"]);
        let playlist_rows = Signal::new(Vec::<PlaylistEntry>::new());
        let show_create_playlist_modal = Signal::new(false);
        let create_playlist_name = Signal::new(String::new());
        let is_creating_playlist = Signal::new(false);
        let show_rename_playlist_modal = Signal::new(false);
        let rename_playlist_id = Signal::new(String::new());
        let rename_playlist_name = Signal::new(String::new());
        let is_renaming_playlist = Signal::new(false);
        let playlist_tracks = Signal::new(Vec::<Track>::new());
        let filtered_playlist_tracks = Signal::new(Vec::<Track>::new());
        let filtered_track_indices = Signal::new(Vec::<usize>::new());
        let playlist_track_filter_input = Signal::new(String::new());
        let active_playlist_id = Signal::new(None::<String>);
        let active_playlist_name = Signal::new(String::new());
        let active_playlist_track_count = Signal::new(0usize);
        let active_playlist_duration_ms = Signal::new(0u64);
        let active_playlist_image_key = Signal::new(None::<String>);
        let playlist_selected_index = Signal::new(0usize);
        let album_tracks = Signal::new(Vec::<Track>::new());
        let album_name = Signal::new(String::new());
        let album_artist = Signal::new(String::new());
        let album_release_year = Signal::new(None::<u32>);
        let album_track_count = Signal::new(0usize);
        let album_total_duration_ms = Signal::new(0u64);
        let album_image_key = Signal::new(None::<String>);
        let album_selected_index = Signal::new(0usize);
        let album_shuffle_mode = Signal::new(false);
        let artist_id = Signal::new(None::<String>);
        let artist_name = Signal::new(String::new());
        let artist_image_key = Signal::new(None::<String>);
        let artist_albums = Signal::new(Vec::<AlbumResult>::new());
        let queue_tracks = Signal::new(Vec::<Track>::new());
        let queue_current_index = Signal::new(None::<usize>);
        let recently_played = Signal::new(Vec::<Track>::new());
        panel_state.build(cx);
        panel_state.load();
        let left_panel_width = panel_state.left_width;
        let right_panel_width = panel_state.right_width;
        let selected_index = Signal::new(0usize);
        let selected_search_tab = Signal::new(0usize);
        let selected_summary = Signal::new("Selected: none".to_string());
        let shuffle_mode = Signal::new(false);
        let current_center_page = Signal::new(CenterPage::Search);
        let page_history = Signal::new(vec![CenterPage::Search]);
        let page_history_index = Signal::new(0usize);
        let can_go_back = Signal::new(false);
        let can_go_forward = Signal::new(false);

        let backend = worker::init_backend(cx.get_proxy());

        let mut preferences_data = PreferencesData::new(cx);
        preferences_data.load(cx);
        dialogs::preferences_dialog(cx, icon, preferences_data);
        dialogs::create_playlist_dialog(
            cx,
            show_create_playlist_modal,
            create_playlist_name,
            is_creating_playlist,
        );
        dialogs::rename_playlist_dialog(
            cx,
            show_rename_playlist_modal,
            rename_playlist_name,
            is_renaming_playlist,
        );

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
            center_state: CenterState {
                current_page: current_center_page,
                page_history,
                page_history_index,
                can_go_back,
                can_go_forward,
            },
            playback_state: PlaybackState {
                playback_track_name,
                playback_track_artist,
                playback_track_id,
                playback_track_image_key,
                playback_track_image_url,
                playback_overlay_image_key,
                search_album_rows,
                album_tracks,
                album_image_key,
                last_remote_volume_sent: None,
                last_remote_volume_sent_at: None,
                last_remote_seek_sent_ms: None,
                last_remote_seek_sent_at: None,
                last_scrub_user_input_at: None,
                last_local_track_end_handled_at: None,
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
                playback_is_muted,
                pre_mute_volume: 0.0,
                recently_played,
                playback_duration_ms,
                artwork_fade_animation,
            },
            search_state: SearchState {
                backend: backend.clone(),
                selected_search_tab,
                status,
                search_input,
                search_result_rows,
                search_artist_rows,
                search_album_rows,
                current_artist_id: artist_id,
                current_artist_albums: artist_albums,
                selected_index,
                selected_summary,
            },
            album_state: AlbumState {
                album_tracks,
                album_name,
                album_artist,
                album_release_year,
                album_track_count,
                album_total_duration_ms,
                album_image_key,
                album_selected_index,
                album_shuffle_mode,
            },
            artist_state: ArtistState {
                backend: backend.clone(),
                status,
                artist_id,
                artist_name,
                artist_image_key,
                artist_albums,
            },
            playlists_state: PlaylistsState {
                backend: backend.clone(),
                status,
                show_create_playlist_modal,
                create_playlist_name,
                is_creating_playlist,
                show_rename_playlist_modal,
                rename_playlist_id,
                rename_playlist_name,
                is_renaming_playlist,
                playlist_rows,
                playlist_tracks,
                filtered_playlist_tracks,
                filtered_track_indices,
                track_filter_input: playlist_track_filter_input,
                active_playlist_id,
                active_playlist_name,
                active_playlist_track_count,
                active_playlist_duration_ms,
                active_playlist_image_key,
                playlist_selected_index,
                shuffle_mode,
                current_playlist_request_id: 0,
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
            panels::header_panel(
                cx,
                search_input,
                auth_username,
                profile_image_key,
                can_go_back,
                can_go_forward,
            );

            Label::new(cx, selected_summary).class("status");
            Textbox::new(cx, status)
                .class("status")
                .read_only(true)
                .on_edit(|_, _| {});

            HStack::new(cx, |cx| {
                Resizable::new(
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

                Binding::new(
                    cx,
                    current_center_page,
                    move |cx| match current_center_page.get() {
                        CenterPage::PlaylistTracks => {
                            panels::playlist_tracks_panel(
                                cx,
                                active_playlist_id,
                                active_playlist_name,
                                active_playlist_track_count,
                                active_playlist_duration_ms,
                                active_playlist_image_key,
                                playlist_track_filter_input,
                                filtered_playlist_tracks,
                                playlist_selected_index,
                                shuffle_mode,
                                playlist_rows,
                            );
                        }
                        CenterPage::AlbumTracks => {
                            panels::album_tracks_panel(
                                cx,
                                album_name,
                                album_artist,
                                album_release_year,
                                album_track_count,
                                album_total_duration_ms,
                                album_image_key,
                                album_tracks,
                                album_selected_index,
                                album_shuffle_mode,
                                playlist_rows,
                            );
                        }
                        CenterPage::Search => {
                            panels::search_results_panel(
                                cx,
                                search_result_rows,
                                search_artist_rows,
                                search_album_rows,
                                selected_index,
                                search_tabs,
                                selected_search_tab,
                                playlist_rows,
                            );
                        }
                        CenterPage::Artist => {
                            panels::artist_panel(cx, artist_name, artist_image_key, artist_albums);
                        }
                    },
                );

                Resizable::new(
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
                playback_is_playing,
                playback_volume,
                playback_is_muted,
                playback_scrub_percent,
                playback_duration_ms,
                playback_track_name,
                playback_track_artist,
                playback_track_image_key,
                playback_track_id,
                playback_track_image_url,
            );
        })
        .width(Stretch(1.0))
        .height(Stretch(1.0))
        .padding(Pixels(8.0))
        .gap(Pixels(8.0));
    })
    .title("Audia")
    .inner_size(
        panel_state
            .window_width
            .map(move |w| (*w, panel_state.window_height.get())),
    )
    .position(
        panel_state
            .window_x
            .map(move |x| (*x, panel_state.window_y.get())),
    )
    .run();
}
