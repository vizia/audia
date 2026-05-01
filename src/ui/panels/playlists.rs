use crate::messages::PlaylistEntry;
use crate::ui::events::PlaylistsUiEvent;
use vizia::icons::{ICON_DOTS, ICON_MUSIC, ICON_PLUS};
use vizia::prelude::*;

pub fn playlists_panel(cx: &mut Context, playlist_rows: Signal<Vec<PlaylistEntry>>) {
    VStack::new(cx, |cx| {
        HStack::new(cx, |cx| {
            Label::new(cx, "Playlists").class("panel-title");
            Spacer::new(cx);
            Button::new(cx, |cx| Svg::new(cx, ICON_PLUS))
                .class("playlist-shuffle-toggle")
                .tooltip(|cx| {
                    Tooltip::new(cx, |cx| {
                        Label::new(cx, "Create a new playlist");
                    })
                })
                .on_press(|cx| cx.emit(PlaylistsUiEvent::OpenCreatePlaylistModal));
        })
        .class("panel-header");

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
                            Svg::new(cx, ICON_MUSIC).class("status");
                        })
                        .size(Pixels(36.0));
                    }
                });

                Label::new(cx, item.map(|p| p.name.clone())).class("playlist-name");

                let playlist_id = item.map(|p| p.id.clone());
                let playlist_name = item.map(|p| p.name.clone());

                Submenu::new(
                    cx,
                    |cx| Svg::new(cx, ICON_DOTS).class("playlist-menu-trigger"),
                    move |cx| {
                        let id_for_rename = playlist_id.get();
                        let name_for_rename = playlist_name.get();
                        MenuButton::new(
                            cx,
                            move |cx| {
                                cx.emit(PlaylistsUiEvent::OpenRenamePlaylistModal {
                                    id: id_for_rename.clone(),
                                    name: name_for_rename.clone(),
                                })
                            },
                            |cx| Label::new(cx, "Rename"),
                        );

                        let id_for_delete = playlist_id.get();
                        MenuButton::new(
                            cx,
                            move |cx| {
                                cx.emit(PlaylistsUiEvent::DeletePlaylist(id_for_delete.clone()))
                            },
                            |cx| Label::new(cx, "Delete"),
                        );
                    },
                )
                .class("track-menu")
                .pointer_events(PointerEvents::Auto);
            })
            .pointer_events(PointerEvents::None)
            .class("result-row");
        })
        .selectable(Selectable::Single)
        .on_select(|cx, idx| cx.emit(PlaylistsUiEvent::SelectPlaylist(idx)))
        .height(Stretch(1.0));
    })
    .class("panel");
}
