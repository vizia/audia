use crate::messages::Track;
use crate::ui::{
    events::{PlaybackUiEvent, RightPanelUiEvent},
    model_data::RightPanelPage,
};
use vizia::icons::{ICON_ARROWS_SHUFFLE, ICON_LIST, ICON_X};
use vizia::prelude::*;

pub fn queue_panel(
    cx: &mut Context,
    queue_tracks: Signal<Vec<Track>>,
    queue_current_index: Signal<Option<usize>>,
) {
    VStack::new(cx, |cx| {
        HStack::new(cx, |cx| {
            Label::new(cx, Localized::new("queue"))
                .class("panel-title")
                .width(Stretch(1.0));

            Button::new(cx, |cx| Svg::new(cx, ICON_ARROWS_SHUFFLE))
                .class("playlist-shuffle-toggle")
                .tooltip(|cx| {
                    Tooltip::new(cx, |cx| {
                        Label::new(cx, Localized::new("shuffle_queue"));
                    })
                })
                .on_press(|cx| cx.emit(PlaybackUiEvent::ShuffleQueue));

            Button::new(cx, |cx| Svg::new(cx, ICON_LIST))
                .class("playlist-shuffle-toggle")
                .tooltip(|cx| {
                    Tooltip::new(cx, |cx| {
                        Label::new(cx, Localized::new("show_recently_played"));
                    })
                })
                .on_press(|cx| {
                    cx.emit(RightPanelUiEvent::NavigateTo(
                        RightPanelPage::RecentlyPlayed,
                    ))
                });

            Button::new(cx, |cx| Svg::new(cx, ICON_X))
                .class("playlist-shuffle-toggle")
                .tooltip(|cx| {
                    Tooltip::new(cx, |cx| {
                        Label::new(cx, Localized::new("clear_queue"));
                    })
                })
                .on_press(|cx| cx.emit(PlaybackUiEvent::ClearQueue));
        })
        .class("panel-header")
        .width(Stretch(1.0))
        .height(Auto)
        .alignment(Alignment::Center);

        List::new(cx, queue_tracks, move |cx, index, item| {
            HStack::new(cx, |cx| {
                let image_key = item.map(|track| track.album_image_key.clone());

                Binding::new(cx, image_key, move |cx| {
                    if let Some(key) = image_key.get() {
                        Image::new(cx, key).size(Pixels(40.0)).class("album-art");
                    } else {
                        Label::new(cx, "♪")
                            .size(Pixels(40.0))
                            .class("search-result-fallback");
                    }
                });

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
            })
            .hoverable(false)
            .class("result-row")
            .toggle_class(
                "playing",
                queue_current_index.map(move |idx| idx.is_some_and(|i| i == index)),
            )
            .width(Stretch(1.0))
            .alignment(Alignment::Center)
            .gap(Pixels(8.0));
        })
        .selectable(Selectable::Single)
        .selection(queue_current_index.map(|idx| idx.map_or_else(Vec::new, |i| vec![i])))
        .selection_follows_focus(true)
        .on_select(|cx, idx| cx.emit(PlaybackUiEvent::SelectQueueTrack(idx)))
        .height(Stretch(1.0));
    })
    .class("panel")
    .class("queue-panel");
}
