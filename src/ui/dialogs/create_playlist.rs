use crate::ui::events::PlaylistsEvents;
use vizia::prelude::*;

pub fn create_playlist_dialog(
    cx: &mut Context,
    show_create_playlist_modal: Signal<bool>,
    create_playlist_name: Signal<String>,
    is_creating_playlist: Signal<bool>,
) {
    Binding::new(cx, show_create_playlist_modal, move |cx| {
        if show_create_playlist_modal.get() {
            Window::popup(cx, true, move |cx| {
                VStack::new(cx, |cx| {
                    Label::new(cx, Localized::new("create_playlist"))
                        .class("login-title")
                        .width(Stretch(1.0))
                        .alignment(Alignment::Center);

                    Textbox::new(cx, create_playlist_name)
                        .placeholder(Localized::new("playlist_name_placeholder"))
                        .on_edit(|cx, value| {
                            cx.emit(PlaylistsEvents::SetCreatePlaylistName(value))
                        })
                        .on_submit(|cx, _value, enter_key| {
                            if enter_key {
                                cx.emit(PlaylistsEvents::SubmitCreatePlaylist);
                            }
                        })
                        .width(Stretch(1.0));

                    HStack::new(cx, |cx| {
                        Button::new(cx, |cx| Label::new(cx, Localized::new("create")))
                            .on_press(|cx| cx.emit(PlaylistsEvents::SubmitCreatePlaylist))
                            .disabled(is_creating_playlist)
                            .width(Pixels(120.0));

                        Button::new(cx, |cx| Label::new(cx, Localized::new("cancel")))
                            .variant(ButtonVariant::Secondary)
                            .on_press(|cx| cx.emit(PlaylistsEvents::CloseCreatePlaylistModal))
                            .disabled(is_creating_playlist)
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
            .on_close(|cx| cx.emit(PlaylistsEvents::CloseCreatePlaylistModal))
            .title("Create playlist")
            .inner_size((420, 170))
            .anchor(Anchor::Center);
        }
    });

    Element::new(cx)
        .size(Stretch(1.0))
        .position_type(PositionType::Absolute)
        .backdrop_filter(Filter::Blur(Pixels(20.0).into()))
        .display(show_create_playlist_modal);
}
