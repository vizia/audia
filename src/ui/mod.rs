use crate::ui::data::{PanelEvent, PanelState};
use crate::worker;
use vizia::prelude::*;

pub mod data;
pub mod dialogs;
pub mod event_handling;
pub mod events;
pub mod model_data;
pub mod panels;

use model_data::{CenterPage, RightPanelPage, UiModel};

pub fn run() -> Result<(), ApplicationError> {
    let icon = image::ImageReader::new(std::io::Cursor::new(include_bytes!(
        "../../resources/icons/icon_32.png"
    )))
    .with_guessed_format()
    .unwrap()
    .decode()
    .unwrap();

    let mut panel_state = PanelState::new();
    panel_state.load();

    Application::new(move |cx| {
        cx.add_stylesheet(include_style!("resources/stylesheets/theme.css"))
            .expect("Failed to load theme stylesheet");

        cx.add_translation(
            langid!("en-GB"),
            include_str!("../../resources/translations/en-GB/strings.ftl"),
        )
        .expect("Failed to load en-GB translation");

        cx.add_translation(
            langid!("en-US"),
            include_str!("../../resources/translations/en-US/strings.ftl"),
        )
        .expect("Failed to load en-US translation");

        let backend = worker::init_backend(cx);

        let app_state = UiModel::new(cx, backend.clone(), panel_state);

        app_state.clone().build(cx);

        worker::start_playback_progress_poller(backend.clone(), cx);

        // Dialogs
        dialogs::login_modal(
            cx,
            app_state.oauth_state.show_login_modal,
            app_state.oauth_state.login_client_id_input,
        );

        dialogs::preferences_dialog(cx, icon, app_state.preferences_data);

        dialogs::create_playlist_dialog(
            cx,
            app_state.playlists_state.show_create_playlist_modal,
            app_state.playlists_state.create_playlist_name,
            app_state.playlists_state.is_creating_playlist,
        );

        dialogs::rename_playlist_dialog(
            cx,
            app_state.playlists_state.show_rename_playlist_modal,
            app_state.playlists_state.rename_playlist_name,
            app_state.playlists_state.is_renaming_playlist,
        );

        // Main blurred background image layer
        Binding::new(
            cx,
            app_state.playback_state.playback_track_image_key,
            move |cx| {
                if let Some(image_key) = app_state.playback_state.playback_track_image_key.get() {
                    ZStack::new(cx, |cx| {
                        Image::new(cx, image_key)
                            .size(Stretch(1.0))
                            .class("blurred-artwork");
                    })
                    .size(Stretch(1.0))
                    .position_type(PositionType::Absolute)
                    .top(Pixels(0.0))
                    .right(Pixels(0.0))
                    .bottom(Pixels(0.0))
                    .left(Pixels(0.0))
                    .pointer_events(PointerEvents::None);
                }
            },
        );

        // Crossfade overlay for smooth image transitions
        Binding::new(
            cx,
            app_state.playback_state.playback_overlay_image_key,
            move |cx| {
                if let Some(image_key) = app_state.playback_state.playback_overlay_image_key.get() {
                    ZStack::new(cx, |cx| {
                        Image::new(cx, image_key)
                            .size(Stretch(1.0))
                            .class("blurred-artwork");
                    })
                    .id("artwork-overlay")
                    .size(Stretch(1.0))
                    .position_type(PositionType::Absolute)
                    .top(Pixels(0.0))
                    .right(Pixels(0.0))
                    .bottom(Pixels(0.0))
                    .left(Pixels(0.0))
                    .opacity(0.0)
                    .pointer_events(PointerEvents::None);
                }
            },
        );

        VStack::new(cx, |cx| {
            panels::header_panel(
                cx,
                app_state.search_state.search_input,
                app_state.oauth_state.auth_username,
                app_state.oauth_state.profile_image_key,
                app_state.center_state.can_go_back,
                app_state.center_state.can_go_forward,
            );

            // Textbox::new(cx, app_state.status)
            //     .class("status")
            //     .read_only(true)
            //     .on_edit(|_, _| {});

            HStack::new(cx, |cx| {
                Resizable::new(
                    cx,
                    app_state.panel_state.left_width.map(|w| Pixels(*w)),
                    ResizeStackDirection::Right,
                    |cx, w| {
                        cx.emit(PanelEvent::SetLeftPanelWidth(w));
                    },
                    move |cx| {
                        panels::playlists_panel(cx, app_state.playlists_state.playlist_rows);
                    },
                )
                .class("left-panel");

                Binding::new(
                    cx,
                    app_state.center_state.current_page,
                    move |cx| match app_state.center_state.current_page.get() {
                        CenterPage::PlaylistTracks => {
                            panels::playlist_tracks_panel(
                                cx,
                                app_state.playlists_state.active_playlist_id,
                                app_state.playlists_state.active_playlist_name,
                                app_state.playlists_state.active_playlist_track_count,
                                app_state.playlists_state.active_playlist_duration_ms,
                                app_state.playlists_state.active_playlist_image_key,
                                app_state.playlists_state.playlist_track_filter_input,
                                app_state.playlists_state.filtered_playlist_tracks,
                                app_state.playlists_state.shuffle_mode,
                                app_state.playlists_state.playlist_rows,
                            );
                        }
                        CenterPage::AlbumTracks => {
                            panels::album_tracks_panel(
                                cx,
                                app_state.album_state.album_name,
                                app_state.album_state.album_artist,
                                app_state.album_state.album_release_year,
                                app_state.album_state.album_track_count,
                                app_state.album_state.album_total_duration_ms,
                                app_state.album_state.album_image_key,
                                app_state.album_state.album_tracks,
                                app_state.album_state.album_selected_index,
                                app_state.album_state.album_shuffle_mode,
                                app_state.playlists_state.playlist_rows,
                            );
                        }
                        CenterPage::Search => {
                            panels::search_results_panel(
                                cx,
                                app_state.search_state.search_track_rows,
                                app_state.search_state.search_artist_rows,
                                app_state.search_state.search_album_rows,
                                app_state.search_state.search_tabs,
                                app_state.search_state.selected_search_tab,
                                app_state.playlists_state.playlist_rows,
                            );
                        }
                        CenterPage::Artist => {
                            panels::artist_panel(
                                cx,
                                app_state.artist_state.artist_name,
                                app_state.artist_state.artist_image_key,
                                app_state.artist_state.artist_albums,
                            );
                        }
                    },
                );

                Resizable::new(
                    cx,
                    app_state.panel_state.right_width.map(|w| Pixels(*w)),
                    ResizeStackDirection::Left,
                    |cx, w| {
                        cx.emit(PanelEvent::SetRightPanelWidth(w));
                    },
                    move |cx| {
                        Binding::new(cx, app_state.right_panel_state.current_page, move |cx| {
                            match app_state.right_panel_state.current_page.get() {
                                RightPanelPage::Queue => {
                                    panels::queue_panel(
                                        cx,
                                        app_state.playback_state.queue_tracks,
                                        app_state.playback_state.queue_current_index,
                                    );
                                }
                                RightPanelPage::RecentlyPlayed => {
                                    panels::recently_played_panel(
                                        cx,
                                        app_state.playback_state.recently_played,
                                    );
                                }
                            }
                        });
                    },
                )
                .class("right-panel");
            })
            .width(Stretch(1.0))
            .height(Stretch(1.0))
            .gap(Pixels(4.0));

            panels::playback_controls_panel(
                cx,
                app_state.playback_state.playback_is_playing,
                app_state.playback_state.playback_volume,
                app_state.playback_state.playback_is_muted,
                app_state.playback_state.playback_scrub_percent,
                app_state.playback_state.playback_duration_ms,
                app_state.playback_state.playback_track_name,
                app_state.playback_state.playback_track_artist,
                app_state.playback_state.playback_track_image_key,
                app_state.playback_state.playback_track_id,
                app_state.playback_state.playback_track_image_url,
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
    .run()
}
