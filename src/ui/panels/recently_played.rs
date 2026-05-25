use crate::messages::Track;
use crate::ui::{
    events::{PlaybackEvents, RightPanelEvents},
    model_data::RightPanelPage,
};
use vizia::icons::{ICON_PLAYLIST, ICON_X};
use vizia::prelude::*;

pub fn recently_played_panel(cx: &mut Context, recently_played: Signal<Vec<Track>>) {
    VStack::new(cx, |cx| {
        HStack::new(cx, |cx| {
            Label::new(cx, Localized::new("recently_played"))
                .class("panel-title")
                .width(Stretch(1.0));

            Button::new(cx, |cx| Svg::new(cx, ICON_PLAYLIST))
                .class("playlist-shuffle-toggle")
                .tooltip(|cx| {
                    Tooltip::new(cx, |cx| {
                        Label::new(cx, Localized::new("show_queue"));
                    })
                })
                .on_press(|cx| cx.emit(RightPanelEvents::NavigateTo(RightPanelPage::Queue)));

            Button::new(cx, |cx| Svg::new(cx, ICON_X))
                .class("playlist-shuffle-toggle")
                .tooltip(|cx| {
                    Tooltip::new(cx, |cx| {
                        Label::new(cx, Localized::new("clear_recently_played"));
                    })
                })
                .on_press(|cx| cx.emit(PlaybackEvents::ClearRecentlyPlayed));
        })
        .class("panel-header")
        .width(Stretch(1.0))
        .height(Auto)
        .alignment(Alignment::Center);

        Binding::new(cx, recently_played.map(|list| list.is_empty()), move |cx| {
            if recently_played.map(|list| list.is_empty()).get() {
                Label::new(cx, Localized::new("no_recently_played"))
                    .class("search-result-artist")
                    .width(Stretch(1.0));
            } else {
                // Show most recent first.
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
                    .width(Stretch(1.0))
                    .alignment(Alignment::Center)
                    .gap(Pixels(8.0));
                })
                .selectable(Selectable::None)
                .height(Stretch(1.0));
            }
        });
    })
    .class("panel")
    .class("queue-panel");
}
