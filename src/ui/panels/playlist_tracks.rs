use crate::messages::{PlaylistEntry, Track};
use crate::ui::events::{PlaylistsUiEvent, SearchUiEvent};
use vizia::icons::{ICON_ARROWS_SHUFFLE, ICON_DOTS, ICON_PLAYER_PLAY_FILLED};
use vizia::prelude::*;

pub fn playlist_tracks_panel(
    cx: &mut Context,
    active_playlist_id: Signal<Option<String>>,
    playlist_name: Signal<String>,
    playlist_track_count: Signal<usize>,
    playlist_duration_ms: Signal<u64>,
    playlist_image_key: Signal<Option<String>>,
    track_filter_input: Signal<String>,
    filtered_playlist_tracks: Signal<Vec<Track>>,
    shuffle_mode: Signal<bool>,
    playlist_rows: Signal<Vec<PlaylistEntry>>,
) {
    fn format_time(ms: u32) -> String {
        let total_seconds = ms / 1000;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{minutes}:{seconds:02}")
    }

    VStack::new(cx, move |cx| {
        // Header with playlist name and meta info
        HStack::new(cx, |cx| {
            Binding::new(cx, playlist_image_key, move |cx| {
                if let Some(key) = playlist_image_key.get() {
                    Image::new(cx, key).class("album-art");
                } else {
                    Label::new(cx, "♪").class("album-art");
                }
            });

            VStack::new(cx, |cx| {
                Label::new(cx, playlist_name).class("playlist-title");

                HStack::new(cx, |cx| {
                    Label::new(
                        cx,
                        playlist_track_count.map(|n| {
                            if *n == 1 {
                                "1 song".to_string()
                            } else {
                                format!("{n} songs")
                            }
                        }),
                    )
                    .class("playlist-meta");

                    Label::new(cx, " • ").class("playlist-meta");

                    Label::new(
                        cx,
                        playlist_duration_ms.map(|ms| {
                            let total_seconds = ms / 1000;
                            let hours = total_seconds / 3600;
                            let minutes = (total_seconds % 3600) / 60;
                            let seconds = total_seconds % 60;
                            if hours > 0 {
                                format!("{hours}:{minutes:02}:{seconds:02}")
                            } else {
                                format!("{minutes}:{seconds:02}")
                            }
                        }),
                    )
                    .class("playlist-meta");
                })
                .class("playlist-meta-row")
                .height(Auto);
            })
            .class("playlist-info");
        })
        .class("playlist-header");

        // Playlist controls and search
        HStack::new(cx, |cx| {
            Button::new(cx, |cx| Svg::new(cx, ICON_PLAYER_PLAY_FILLED))
                .class("playback-toggle")
                .tooltip(|cx| {
                    Tooltip::new(cx, |cx| {
                        Label::new(cx, Localized::new("play_playlist"));
                    })
                })
                .on_press(|cx| cx.emit(PlaylistsUiEvent::PlayPlaylist));
            ToggleButton::new(cx, shuffle_mode, |cx| Svg::new(cx, ICON_ARROWS_SHUFFLE))
                .class("playlist-shuffle-toggle")
                .tooltip(|cx| {
                    Tooltip::new(cx, |cx| {
                        Label::new(cx, Localized::new("shuffle_playlist"));
                    })
                })
                .on_press(|cx| cx.emit(PlaylistsUiEvent::ShufflePlaylist));
            Textbox::new(cx, track_filter_input)
                .placeholder(Localized::new("search"))
                .on_edit(|cx, value| cx.emit(PlaylistsUiEvent::SetTrackFilter(value)))
                .width(Stretch(1.0));
        })
        .height(Auto)
        .width(Stretch(1.0))
        .alignment(Alignment::Center)
        .gap(Pixels(8.0));

        // Track list
        List::new(cx, filtered_playlist_tracks, move |cx, index, item| {
            HStack::new(cx, |cx| {
                // Song number
                Label::new(cx, format!("{}", index + 1)).class("track-index");

                let image_key = item.map(|track| track.album_image_key.clone());
                let track_id = item.map(|track| track.id.clone());

                // Album art with click to open album
                Binding::new(cx, image_key, move |cx| {
                    let tid = track_id.get();
                    if let Some(key) = image_key.get() {
                        Image::new(cx, key)
                            .class("playlist-track-album-art")
                            .pointer_events(PointerEvents::Auto)
                            .on_press(move |cx| {
                                cx.emit(SearchUiEvent::OpenAlbumFromTrack(tid.clone()))
                            });
                    } else {
                        Element::new(cx)
                            .class("playlist-track-album-art")
                            .pointer_events(PointerEvents::Auto)
                            .on_press(move |cx| {
                                cx.emit(SearchUiEvent::OpenAlbumFromTrack(tid.clone()))
                            });
                    }
                });

                // Track title and artist
                VStack::new(cx, |cx| {
                    Label::new(cx, item.map(|track| track.name.clone()))
                        .text_wrap(false)
                        .class("track-title");
                    Label::new(cx, item.map(|track| track.artist.clone()))
                        .text_wrap(false)
                        .class("track-artist");
                })
                .width(Stretch(1.0))
                .height(Auto)
                .gap(Pixels(2.0));

                // Track duration
                Label::new(cx, item.map(|track| format_time(track.duration_ms)))
                    .hoverable(false)
                    .class("track-duration");

                let track_id_for_menu = item.map(|track| track.id.clone());
                let track_id_copy = track_id_for_menu.get();
                let playlists_copy = playlist_rows.get();

                // Add to Playlist menu
                Submenu::new(
                    cx,
                    |cx| Svg::new(cx, ICON_DOTS),
                    move |cx| {
                        let active_id = active_playlist_id.get().unwrap_or_default();
                        let remove_track_id = track_id_copy.clone();
                        MenuButton::new(
                            cx,
                            move |cx| {
                                if !active_id.is_empty() && !remove_track_id.is_empty() {
                                    cx.emit(PlaylistsUiEvent::RemoveTrackFromPlaylist {
                                        track_id: remove_track_id.clone(),
                                        playlist_id: active_id.clone(),
                                    });
                                }
                            },
                            |cx| Label::new(cx, Localized::new("remove_from_playlist")),
                        );

                        for playlist in &playlists_copy {
                            let pid = playlist.id.clone();
                            let tid = track_id_copy.clone();
                            let pname = playlist.name.clone();

                            MenuButton::new(
                                cx,
                                move |cx| {
                                    if !pid.is_empty() && !tid.is_empty() {
                                        cx.emit(PlaylistsUiEvent::AddTrackToPlaylist {
                                            track_id: tid.clone(),
                                            playlist_id: pid.clone(),
                                        });
                                    }
                                },
                                move |cx| Label::new(cx, format!("{}", pname)),
                            );
                        }
                    },
                )
                .pointer_events(PointerEvents::Auto)
                .class("track-menu");
            })
            .pointer_events(PointerEvents::None)
            .class("playlist-track-row");
        })
        .selectable(Selectable::Single)
        .selection_follows_focus(true)
        .on_select(|cx, idx| cx.emit(PlaylistsUiEvent::PlaylistTrackSelected(idx)))
        .width(Stretch(1.0))
        .height(Stretch(1.0));
    })
    .class("panel")
    .class("playlist-tracks-panel");
}
