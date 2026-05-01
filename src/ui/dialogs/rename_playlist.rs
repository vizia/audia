use crate::ui::events::PlaylistsUiEvent;
use vizia::prelude::*;

pub fn rename_playlist_dialog(
    cx: &mut Context,
    show_rename_playlist_modal: Signal<bool>,
    rename_playlist_name: Signal<String>,
    is_renaming_playlist: Signal<bool>,
) {
    Binding::new(cx, show_rename_playlist_modal, move |cx| {
        if show_rename_playlist_modal.get() {
            Window::popup(cx, true, move |cx| {
                VStack::new(cx, |cx| {
                    Label::new(cx, "Rename playlist")
                        .class("login-title")
                        .width(Stretch(1.0))
                        .alignment(Alignment::Center);

                    Textbox::new(cx, rename_playlist_name)
                        .placeholder("New playlist name")
                        .on_edit(|cx, value| {
                            cx.emit(PlaylistsUiEvent::SetRenamePlaylistName(value))
                        })
                        .on_submit(|cx, _value, enter_key| {
                            if enter_key {
                                cx.emit(PlaylistsUiEvent::SubmitRenamePlaylist);
                            }
                        })
                        .width(Stretch(1.0));

                    HStack::new(cx, |cx| {
                        Button::new(cx, |cx| Label::new(cx, "Rename"))
                            .on_press(|cx| cx.emit(PlaylistsUiEvent::SubmitRenamePlaylist))
                            .disabled(is_renaming_playlist)
                            .width(Pixels(120.0));

                        Button::new(cx, |cx| Label::new(cx, "Cancel"))
                            .variant(ButtonVariant::Secondary)
                            .on_press(|cx| cx.emit(PlaylistsUiEvent::CloseRenamePlaylistModal))
                            .disabled(is_renaming_playlist)
                            .width(Pixels(120.0));
                    })
                    .gap(Pixels(12.0))
                    .width(Auto)
                    .height(Auto);
                })
                .alignment(Alignment::TopCenter)
                .vertical_gap(Pixels(16.0))
                .padding(Pixels(16.0));
            })
            .on_close(|cx| cx.emit(PlaylistsUiEvent::CloseRenamePlaylistModal))
            .title("Rename playlist")
            .inner_size((420, 170))
            .anchor(Anchor::Center);
        }
    });

    Element::new(cx)
        .size(Stretch(1.0))
        .position_type(PositionType::Absolute)
        .backdrop_filter(Filter::Blur(Pixels(20.0).into()))
        .display(show_rename_playlist_modal);
}
