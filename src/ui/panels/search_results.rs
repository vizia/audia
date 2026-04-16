use crate::messages::{AlbumResult, ArtistResult, Track};
use crate::ui::events::SearchUiEvent;
use vizia::prelude::*;

pub fn search_results_panel(
    cx: &mut Context,
    search_result_rows: Signal<Vec<Track>>,
    search_artist_rows: Signal<Vec<ArtistResult>>,
    search_album_rows: Signal<Vec<AlbumResult>>,
    selected_index: Signal<usize>,
) {
    fn format_time(ms: u32) -> String {
        let total_seconds = ms / 1000;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{minutes}:{seconds:02}")
    }

    VStack::new(cx, move |cx| {
        Label::new(cx, "Search Results").class("panel-title");

        ScrollView::new(cx, move |cx| {
            Label::new(cx, "Tracks").class("search-section-title");
            List::new(cx, search_result_rows, |cx, _index, item| {
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
                .class("result-row");
            })
            .selectable(Selectable::Single)
            .selection(selected_index.map(|idx| vec![*idx]))
            .selection_follows_focus(true)
            .on_select(|cx, idx| cx.emit(SearchUiEvent::SelectResult(idx)))
            .width(Stretch(1.0))
            .height(Auto);

            Label::new(cx, "Artists").class("search-section-title");
            List::new(cx, search_artist_rows, |cx, _index, item| {
                HStack::new(cx, |cx| {
                    let image_key = item.map(|artist| artist.image_key.clone());

                    Binding::new(cx, image_key, move |cx| {
                        if let Some(key) = image_key.get() {
                            Image::new(cx, key).size(Pixels(40.0)).class("album-art");
                        } else {
                            Label::new(cx, "◉")
                                .size(Pixels(40.0))
                                .class("search-result-fallback");
                        }
                    });

                    Label::new(cx, item.map(|artist| artist.name.clone()))
                        .text_wrap(false)
                        .class("search-result-title")
                        .width(Stretch(1.0));
                })
                .class("result-row");
            })
            .selectable(Selectable::None)
            .width(Stretch(1.0))
            .height(Auto);

            Label::new(cx, "Albums").class("search-section-title");
            List::new(cx, search_album_rows, |cx, _index, item| {
                HStack::new(cx, |cx| {
                    let image_key = item.map(|album| album.image_key.clone());

                    Binding::new(cx, image_key, move |cx| {
                        if let Some(key) = image_key.get() {
                            Image::new(cx, key).size(Pixels(40.0)).class("album-art");
                        } else {
                            Label::new(cx, "◫")
                                .size(Pixels(40.0))
                                .class("search-result-fallback");
                        }
                    });

                    VStack::new(cx, |cx| {
                        Label::new(cx, item.map(|album| album.name.clone()))
                            .text_wrap(false)
                            .width(Stretch(1.0))
                            .class("search-result-title");
                        Label::new(cx, item.map(|album| album.artist.clone()))
                            .text_wrap(false)
                            .width(Stretch(1.0))
                            .class("search-result-artist");
                    })
                    .width(Stretch(1.0))
                    .height(Auto)
                    .gap(Pixels(2.0));
                })
                .hoverable(false)
                .class("result-row");
            })
            .selectable(Selectable::Single)
            .selection_follows_focus(true)
            .on_select(|cx, idx| cx.emit(SearchUiEvent::SelectAlbum(idx)))
            .width(Stretch(1.0))
            .height(Auto);
        })
        .gap(Pixels(4.0));
    })
    .class("panel")
    .class("search-results-panel")
    .width(Stretch(1.0))
    .height(Stretch(1.0))
    .padding(Pixels(8.0))
    .gap(Pixels(4.0));
}
