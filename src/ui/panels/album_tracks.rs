use crate::messages::{PlaylistEntry, Track};
use crate::ui::events::{AlbumEvents, PlaylistsEvents};
use vizia::icons::{ICON_ARROWS_SHUFFLE, ICON_DOTS, ICON_PLAYER_PLAY_FILLED};
use vizia::prelude::*;

pub fn album_tracks_panel(
    cx: &mut Context,
    album_name: Signal<String>,
    album_artist: Signal<String>,
    album_release_year: Signal<Option<u32>>,
    album_track_count: Signal<usize>,
    album_total_duration_ms: Signal<u64>,
    album_image_key: Signal<Option<String>>,
    album_tracks: Signal<Vec<Track>>,
    album_selected_index: Signal<usize>,
    album_shuffle_mode: Signal<bool>,
    playlist_rows: Signal<Vec<PlaylistEntry>>,
) {
    fn format_time(ms: u32) -> String {
        let total_seconds = ms / 1000;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{minutes}:{seconds:02}")
    }

    VStack::new(cx, move |cx| {
        HStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                Binding::new(cx, album_image_key, move |cx| {
                    if let Some(key) = album_image_key.get() {
                        Image::new(cx, key).class("album-art");
                    } else {
                        Label::new(cx, "◫").class("album-art");
                    }
                });

                VStack::new(cx, |cx| {
                    Label::new(cx, album_name)
                        .text_wrap(false)
                        .class("album-title");

                    HStack::new(cx, move |cx| {
                        Label::new(cx, album_artist)
                            .text_wrap(false)
                            .class("album-meta");

                        Label::new(cx, " • ").class("album-meta");

                        let album_release_year_signal = album_release_year;
                        Binding::new(cx, album_release_year_signal, move |cx| {
                            if let Some(year) = album_release_year_signal.get() {
                                Label::new(cx, format!("{year}"))
                                    .text_wrap(false)
                                    .class("album-meta");
                                Label::new(cx, " • ").class("album-meta");
                            }
                        });

                        Label::new(
                            cx,
                            album_track_count.map(|count| format!("{} songs", count)),
                        )
                        .text_wrap(false)
                        .class("album-meta");

                        Label::new(cx, " • ").class("album-meta");

                        Label::new(
                            cx,
                            album_total_duration_ms.map(|duration_ms| {
                                let total_minutes = duration_ms / 60_000;
                                format!("{}m", total_minutes)
                            }),
                        )
                        .text_wrap(false)
                        .class("album-meta");
                    })
                    .class("album-meta-row");
                })
                .class("album-info");
            })
            .class("album-info-row");

            Button::new(cx, |cx| Svg::new(cx, ICON_PLAYER_PLAY_FILLED))
                .class("playback-toggle")
                .name("Play all")
                .tooltip(|cx| {
                    Tooltip::new(cx, |cx| {
                        Label::new(cx, Localized::new("play_album"));
                    })
                })
                .on_press(|cx| cx.emit(AlbumEvents::PlayAlbum));

            ToggleButton::new(cx, album_shuffle_mode, |cx| {
                Svg::new(cx, ICON_ARROWS_SHUFFLE)
            })
            .class("playlist-shuffle-toggle")
            .tooltip(|cx| {
                Tooltip::new(cx, |cx| {
                    Label::new(cx, Localized::new("shuffle_album"));
                })
            })
            .on_press(|cx| cx.emit(AlbumEvents::ShuffleAlbum));
        })
        .class("album-header");

        // Track list
        List::new(cx, album_tracks, move |cx, index, item| {
            HStack::new(cx, |cx| {
                Label::new(cx, format!("{}", index + 1)).class("track-index");

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

                Label::new(cx, item.map(|track| format_time(track.duration_ms)))
                    .class("track-duration");

                // Add to Playlist menu
                let track_id_for_menu = item.map(|track| track.id.clone());
                let track_id_copy = track_id_for_menu.get();
                let playlists_copy = playlist_rows.get();

                Submenu::new(
                    cx,
                    |cx| Svg::new(cx, ICON_DOTS),
                    move |cx| {
                        for playlist in &playlists_copy {
                            let pid = playlist.id.clone();
                            let tid = track_id_copy.clone();
                            let pname = playlist.name.clone();

                            MenuButton::new(
                                cx,
                                move |cx| {
                                    if !pid.is_empty() && !tid.is_empty() {
                                        cx.emit(PlaylistsEvents::AddTrackToPlaylist {
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
            .class("album-track-row");
        })
        .selectable(Selectable::Single)
        .selection(album_selected_index.map(|idx| vec![*idx]))
        .selection_follows_focus(true)
        .on_select(|cx, idx| cx.emit(AlbumEvents::AlbumTrackSelected(idx)))
        .width(Stretch(1.0))
        .height(Stretch(1.0));
    })
    .class("panel")
    .class("album-tracks-panel");
}
