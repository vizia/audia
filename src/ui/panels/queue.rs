use crate::messages::Track;
use crate::ui::events::PlaybackUiEvent;
use vizia::icons::{ICON_ARROWS_SHUFFLE, ICON_CLEAR_ALL};
use vizia::prelude::*;

pub fn queue_panel(
    cx: &mut Context,
    queue_tracks: Signal<Vec<Track>>,
    queue_current_index: Signal<Option<usize>>,
    recently_played: Signal<Vec<Track>>,
) {
    VStack::new(cx, |cx| {
        HStack::new(cx, |cx| {
            Label::new(cx, "Queue")
                .class("panel-title")
                .width(Stretch(1.0));

            Button::new(cx, |cx| Svg::new(cx, ICON_ARROWS_SHUFFLE))
                .class("playlist-shuffle-toggle")
                .on_press(|cx| cx.emit(PlaybackUiEvent::ShuffleQueue));

            Button::new(cx, |cx| Svg::new(cx, ICON_CLEAR_ALL))
                .class("playlist-shuffle-toggle")
                .on_press(|cx| cx.emit(PlaybackUiEvent::ClearQueue));
        })
        .class("queue-controls")
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
                        .class("search-result-title");
                    Label::new(cx, item.map(|track| track.artist.clone()))
                        .text_wrap(false)
                        .class("search-result-artist");
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

        // Recently played section
        Binding::new(
            cx,
            recently_played.map(|list| !list.is_empty()),
            move |cx| {
                if recently_played.map(|list| !list.is_empty()).get() {
                    HStack::new(cx, |cx| {
                        Label::new(cx, "Recently Played")
                            .class("panel-title")
                            .width(Stretch(1.0));

                        Button::new(cx, |cx| Svg::new(cx, ICON_CLEAR_ALL))
                            .class("playlist-shuffle-toggle")
                            .on_press(|cx| cx.emit(PlaybackUiEvent::ClearRecentlyPlayed));
                    })
                    .class("queue-controls")
                    .width(Stretch(1.0))
                    .height(Auto)
                    .alignment(Alignment::Center);

                    // Show most recent first
                    let reversed =
                        recently_played.map(|list| list.iter().rev().cloned().collect::<Vec<_>>());
                    List::new(cx, reversed, |cx, _index, item| {
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
                                    .class("search-result-title");
                                Label::new(cx, item.map(|track| track.artist.clone()))
                                    .text_wrap(false)
                                    .class("search-result-artist");
                            })
                            .width(Stretch(1.0))
                            .height(Auto)
                            .gap(Pixels(2.0));
                        })
                        .hoverable(false)
                        .class("result-row")
                        .width(Stretch(1.0))
                        .alignment(Alignment::Center)
                        .gap(Pixels(8.0));
                    })
                    .selectable(Selectable::None)
                    .height(Stretch(1.0));
                }
            },
        );
    })
    .class("panel")
    .class("queue-panel");
}
