use crate::ui::events::PlaylistsUiEvent;
use vizia::prelude::*;

pub fn playlist_context_menu(
    cx: &mut Context,
    show_playlist_context_menu: Signal<bool>,
    context_menu_playlist_id: Signal<String>,
    context_menu_playlist_name: Signal<String>,
) {
    Binding::new(cx, show_playlist_context_menu, move |cx| {
        if show_playlist_context_menu.get() {
            Window::popup(cx, true, move |cx| {
                VStack::new(cx, move |cx| {
                    let name = context_menu_playlist_name.get();
                    Label::new(cx, name)
                        .class("login-title")
                        .width(Stretch(1.0))
                        .alignment(Alignment::Center);

                    Button::new(cx, move |cx| Label::new(cx, "Rename"))
                        .width(Stretch(1.0))
                        .on_press(move |cx| {
                            cx.emit(PlaylistsUiEvent::OpenRenamePlaylistModal {
                                id: context_menu_playlist_id.get(),
                                name: context_menu_playlist_name.get(),
                            });
                        });

                    Button::new(cx, |cx| Label::new(cx, "Delete"))
                        .variant(ButtonVariant::Outline)
                        .width(Stretch(1.0))
                        .on_press(move |cx| {
                            cx.emit(PlaylistsUiEvent::DeletePlaylist(
                                context_menu_playlist_id.get(),
                            ));
                        });

                    Button::new(cx, |cx| Label::new(cx, "Cancel"))
                        .variant(ButtonVariant::Secondary)
                        .width(Stretch(1.0))
                        .on_press(|cx| cx.emit(PlaylistsUiEvent::ClosePlaylistContextMenu));
                })
                .alignment(Alignment::TopCenter)
                .vertical_gap(Pixels(8.0))
                .padding(Pixels(16.0));
            })
            .on_close(|cx| cx.emit(PlaylistsUiEvent::ClosePlaylistContextMenu))
            .title("")
            .inner_size((260, 170))
            .anchor(Anchor::Center);
        }
    });
}
