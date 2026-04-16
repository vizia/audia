use crate::messages::Track;
use crate::ui::events::{PlaybackUiEvent, SearchUiEvent};
use vizia::icons::{ICON_ARROW_LEFT, ICON_PLAYER_PLAY_FILLED};
use vizia::prelude::*;

pub fn album_tracks_panel(
    cx: &mut Context,
    album_name: Signal<String>,
    album_artist: Signal<String>,
    album_image_key: Signal<Option<String>>,
    album_tracks: Signal<Vec<Track>>,
    album_selected_index: Signal<usize>,
) {
    fn format_time(ms: u32) -> String {
        let total_seconds = ms / 1000;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{minutes}:{seconds:02}")
    }

    VStack::new(cx, move |cx| {
        // Header: back button, album art, album name + artist
        HStack::new(cx, |cx| {
            Button::new(cx, |cx| Svg::new(cx, ICON_ARROW_LEFT))
                .class("back-button")
                .name("Back to Search")
                .on_press(|cx| cx.emit(SearchUiEvent::BackFromAlbum));

            Binding::new(cx, album_image_key, move |cx| {
                if let Some(key) = album_image_key.get() {
                    Image::new(cx, key).size(Pixels(60.0)).class("album-art");
                } else {
                    Label::new(cx, "◫")
                        .size(Pixels(60.0))
                        .class("search-result-fallback");
                }
            });

            VStack::new(cx, |cx| {
                Label::new(cx, album_name)
                    .text_wrap(false)
                    .class("panel-title");
                Label::new(cx, album_artist)
                    .text_wrap(false)
                    .class("playlist-meta");
            })
            .width(Stretch(1.0))
            .height(Auto)
            .gap(Pixels(4.0));

            Button::new(cx, |cx| Svg::new(cx, ICON_PLAYER_PLAY_FILLED))
                .class("playback-toggle")
                .name("Play all")
                .on_press(move |cx| {
                    let tracks = album_tracks.get();
                    if !tracks.is_empty() {
                        cx.emit(PlaybackUiEvent::AddToQueue(tracks));
                    }
                });
        })
        .height(Auto)
        .width(Stretch(1.0))
        .alignment(Alignment::Center)
        .gap(Pixels(8.0));

        // Track list
        List::new(cx, album_tracks, |cx, index, item| {
            HStack::new(cx, |cx| {
                Label::new(cx, format!("{}.", index + 1))
                    .class("search-result-index")
                    .width(Pixels(20.0));

                VStack::new(cx, |cx| {
                    Label::new(cx, item.map(|track| track.name.clone()))
                        .text_wrap(false)
                        .class("search-result-title");
                    Label::new(cx, item.map(|track| track.artist.clone()))
                        .text_wrap(false)
                        .class("search-result-artist");
                })
                .width(Stretch(1.0))
                .height(Auto)
                .gap(Pixels(2.0));

                Label::new(cx, item.map(|track| format_time(track.duration_ms)))
                    .class("search-result-duration");
            })
            .hoverable(false)
            .class("result-row")
            .width(Stretch(1.0))
            .alignment(Alignment::Center)
            .gap(Pixels(8.0));
        })
        .selectable(Selectable::Single)
        .selection(album_selected_index.map(|idx| vec![*idx]))
        .selection_follows_focus(true)
        .on_select(|cx, idx| cx.emit(SearchUiEvent::AlbumTrackSelected(idx)))
        .width(Stretch(1.0))
        .height(Stretch(1.0));
    })
    .class("panel")
    .class("album-tracks-panel");
}
