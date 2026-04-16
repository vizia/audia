use crate::messages::Track;
use crate::ui::events::PlaylistsUiEvent;
use vizia::icons::{ICON_ARROWS_SHUFFLE, ICON_PLAYER_PLAY_FILLED};
use vizia::prelude::*;

pub fn playlist_tracks_panel(
    cx: &mut Context,
    playlist_name: Signal<String>,
    playlist_track_count: Signal<usize>,
    playlist_duration_ms: Signal<u64>,
    playlist_image_key: Signal<Option<String>>,
    track_filter_input: Signal<String>,
    filtered_playlist_tracks: Signal<Vec<Track>>,
    playlist_selected_index: Signal<usize>,
    shuffle_mode: Signal<bool>,
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
                Label::new(cx, playlist_name)
                    .text_wrap(false)
                    .class("playlist-title")
                    .width(Stretch(1.0));

                HStack::new(cx, |cx| {
                    Label::new(cx, playlist_track_count.map(|n| {
                        if *n == 1 { "1 song".to_string() } else { format!("{n} songs") }
                    }))
                    .class("playlist-meta");

                    Label::new(cx, " • ").class("playlist-meta");

                    Label::new(cx, playlist_duration_ms.map(|ms| {
                        let total_seconds = ms / 1000;
                        let hours = total_seconds / 3600;
                        let minutes = (total_seconds % 3600) / 60;
                        let seconds = total_seconds % 60;
                        if hours > 0 {
                            format!("{hours}:{minutes:02}:{seconds:02}")
                        } else {
                            format!("{minutes}:{seconds:02}")
                        }
                    }))
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
                .on_press(|cx| cx.emit(PlaylistsUiEvent::PlayPlaylist));
            ToggleButton::new(cx, shuffle_mode, |cx| Svg::new(cx, ICON_ARROWS_SHUFFLE))
                .class("playlist-shuffle-toggle")
                .on_press(|cx| cx.emit(PlaylistsUiEvent::ShufflePlaylist));
            Textbox::new(cx, track_filter_input)
                //.placeholder("Search tracks")
                .on_edit(|cx, value| cx.emit(PlaylistsUiEvent::SetTrackFilter(value)))
                .width(Stretch(1.0));
        })
        .height(Auto)
        .width(Stretch(1.0))
        .alignment(Alignment::Center)
        .gap(Pixels(8.0));

        // Track list
        List::new(cx, filtered_playlist_tracks, |cx, index, item| {
            HStack::new(cx, |cx| {
                // Song number
                Label::new(cx, format!("{}.", index + 1)).class("playlist-track-index");

                let image_key = item.map(|track| track.album_image_key.clone());

                Binding::new(cx, image_key, move |cx| {
                    if let Some(key) = image_key.get() {
                        Image::new(cx, key).class("playlist-track-album-art");
                    } else {
                        Label::new(cx, "♪").class("playlist-track-album-art");
                    }
                });

                VStack::new(cx, |cx| {
                    Label::new(cx, item.map(|track| track.name.clone()))
                        .text_wrap(false)
                        .class("playlist-track-title");
                    Label::new(cx, item.map(|track| track.artist.clone()))
                        .text_wrap(false)
                        .class("playlist-track-artist");
                })
                .width(Stretch(1.0))
                .height(Auto)
                .gap(Pixels(2.0));

                Label::new(cx, item.map(|track| format_time(track.duration_ms)))
                    .class("playlist-track-duration");
            })
            .hoverable(false)
            .class("playlist-track-row");
        })
        .selectable(Selectable::Single)
        .selection(playlist_selected_index.map(|idx| vec![*idx]))
        .selection_follows_focus(true)
        .on_select(|cx, idx| cx.emit(PlaylistsUiEvent::PlaylistTrackSelected(idx)))
        .width(Stretch(1.0))
        .height(Stretch(1.0));
    })
    .class("panel")
    .class("playlist-tracks-panel");
}
