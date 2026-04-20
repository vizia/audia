use crate::messages::AlbumResult;
use crate::ui::events::ArtistUiEvent;
use vizia::prelude::*;

pub fn artist_panel(
    cx: &mut Context,
    artist_name: Signal<String>,
    artist_image_key: Signal<Option<String>>,
    artist_albums: Signal<Vec<AlbumResult>>,
) {
    VStack::new(cx, move |cx| {
        HStack::new(cx, |cx| {
            Binding::new(cx, artist_image_key, move |cx| {
                if let Some(key) = artist_image_key.get() {
                    Image::new(cx, key).class("album-art");
                } else {
                    Label::new(cx, "◉").class("album-art");
                }
            });

            VStack::new(cx, |cx| {
                Label::new(cx, artist_name)
                    .text_wrap(false)
                    .class("album-title");
                Label::new(cx, "Artist").class("album-meta");
            })
            .class("album-info");
        })
        .class("album-info-row");

        Label::new(cx, "Albums").class("panel-title");

        List::new(cx, artist_albums, |cx, _index, item| {
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
                    Label::new(
                        cx,
                        item.map(|album| {
                            if let Some(release_date) = album.release_date.as_ref() {
                                format!("{} • {}", album.artist, release_date)
                            } else {
                                album.artist.clone()
                            }
                        }),
                    )
                    .text_wrap(false)
                    .width(Stretch(1.0))
                    .class("search-result-artist");
                })
                .width(Stretch(1.0))
                .height(Auto)
                .gap(Pixels(2.0));
            })
            .class("result-row");
        })
        .selectable(Selectable::Single)
        .selection_follows_focus(true)
        .on_select(|cx, idx| cx.emit(ArtistUiEvent::ArtistAlbumSelected(idx)))
        .width(Stretch(1.0))
        .height(Stretch(1.0));
    })
    .class("panel")
    .class("album-tracks-panel")
    .width(Stretch(1.0))
    .height(Stretch(1.0));
}
