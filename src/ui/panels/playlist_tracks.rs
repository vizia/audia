use crate::messages::Track;
use crate::ui::events::PlaylistsUiEvent;
use vizia::prelude::*;

pub fn playlist_tracks_panel(
    cx: &mut Context,
    playlist_name: Signal<String>,
    playlist_tracks: Signal<Vec<Track>>,
    playlist_selected_index: Signal<usize>,
) {
    fn format_time(ms: u32) -> String {
        let total_seconds = ms / 1000;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{minutes}:{seconds:02}")
    }

    VStack::new(cx, move |cx| {
        Label::new(cx, playlist_name)
            .class("panel-title")
            .width(Stretch(1.0));

        HStack::new(cx, |cx| {
            Button::new(cx, |cx| Label::new(cx, "← Back"))
                .on_press(|cx| cx.emit(PlaylistsUiEvent::BackToSearch))
                .class("back-button");

            Button::new(cx, |cx| Label::new(cx, "Add All to Queue"))
                .on_press(|cx| cx.emit(PlaylistsUiEvent::AddPlaylistToQueue))
                .class("add-to-queue-button");
        })
        .height(Auto)
        .width(Stretch(1.0))
        .alignment(Alignment::Center)
        .gap(Pixels(8.0));

        List::new(cx, playlist_tracks, |cx, _index, item| {
            HStack::new(cx, |cx| {
                let image_key = item.map(|track| track.album_image_key.clone());

                Binding::new(cx, image_key, move |cx| {
                    if let Some(key) = image_key.get() {
                        Image::new(cx, key).size(Pixels(48.0)).class("album-art");
                    } else {
                        Label::new(cx, "♪")
                            .size(Pixels(48.0))
                            .class("search-result-fallback");
                    }
                });

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
        .selected(playlist_selected_index.map(|idx| vec![*idx]))
        .selection_follows_focus(true)
        .on_select(|cx, idx| cx.emit(PlaylistsUiEvent::PlaylistTrackSelected(idx)))
        .width(Stretch(1.0))
        .height(Stretch(1.0));
    })
    .class("panel")
    .width(Stretch(1.0))
    .height(Stretch(1.0))
    .padding(Pixels(8.0))
    .gap(Pixels(4.0));
}
