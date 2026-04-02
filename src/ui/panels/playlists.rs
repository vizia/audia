use crate::messages::PlaylistEntry;
use crate::ui::events::PlaylistsUiEvent;
use vizia::prelude::*;

pub fn playlists_panel(cx: &mut Context, playlist_rows: Signal<Vec<PlaylistEntry>>) {
    VStack::new(cx, |cx| {
        Label::new(cx, "Playlists").class("panel-title");

        // Binding::new(cx, playlist_rows, move |cx| {
        //     if playlist_rows.get().is_empty() {
        //         Label::new(cx, "No playlists loaded yet.")
        //             .class("status")
        //             .width(Stretch(1.0));
        //         return;
        //     }

        List::new(cx, playlist_rows, |cx, _index, item| {
            HStack::new(cx, |cx| {
                let image_key = item.map(|p| p.image_key.clone());

                Binding::new(cx, image_key, move |cx| {
                    if let Some(key) = image_key.get() {
                        Avatar::new(cx, |cx| {
                            Image::new(cx, key).size(Stretch(1.0));
                        })
                        .size(Pixels(36.0));
                    } else {
                        Avatar::new(cx, |cx| {
                            Label::new(cx, "♪").class("status");
                        })
                        .size(Pixels(36.0));
                    }
                });

                Label::new(cx, item.map(|p| p.name.clone()))
                    .class("playlist-name")
                    .width(Stretch(1.0));
            })
            .hoverable(false)
            .class("result-row");
        })
        .selectable(Selectable::Single)
        .on_select(|cx, idx| cx.emit(PlaylistsUiEvent::SelectPlaylist(idx)))
        .height(Stretch(1.0));
        // });
    })
    .class("panel")
    .width(Stretch(1.0))
    .height(Stretch(1.0))
    .padding(Pixels(8.0))
    .gap(Pixels(4.0));
}
