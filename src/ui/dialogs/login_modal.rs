use crate::ui::events::OAuthEvents;
use vizia::prelude::*;

pub fn login_modal(
    cx: &mut Context,
    show_login_modal: Signal<bool>,
    login_client_id_input: Signal<String>,
) {
    Binding::new(cx, show_login_modal, move |cx| {
        let is_open = show_login_modal.get();
        if is_open {
            Window::popup(cx, true, move |cx| {
                VStack::new(cx, |cx| {
                    VStack::new(cx, |cx| {
                        Label::new(cx, Localized::new("login_to_spotify"))
                            .class("login-title")
                            .width(Stretch(1.0))
                            .alignment(Alignment::Center);

                        Label::new(cx, Localized::new("login_description"))
                            .alignment(Alignment::Center)
                            .width(Stretch(1.0));

                        Label::new(cx, Localized::new("spotify_premium_required"))
                            .alignment(Alignment::Center)
                            .width(Stretch(1.0));

                        Textbox::new(cx, login_client_id_input)
                            .placeholder(Localized::new("spotify_client_id_placeholder"))
                            .on_edit(|cx, value| cx.emit(OAuthEvents::SetLoginClientId(value)))
                            .on_submit(|cx, _value, enter_key| {
                                if enter_key {
                                    cx.emit(OAuthEvents::StartOAuthLogin);
                                }
                            })
                            .width(Stretch(1.0));
                    })
                    .padding_top(Pixels(12.0))
                    .alignment(Alignment::TopCenter)
                    .gap(Pixels(8.0));

                    HStack::new(cx, |cx| {
                        Button::new(cx, |cx| {
                            Label::new(cx, Localized::new("login_with_spotify"))
                        })
                        .on_press(|cx| cx.emit(OAuthEvents::StartOAuthLogin))
                        .width(Pixels(170.0));

                        Button::new(cx, |cx| Label::new(cx, Localized::new("close")))
                            .variant(ButtonVariant::Secondary)
                            .on_press(|cx| cx.emit(OAuthEvents::CloseLoginModal))
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
            .on_close(|cx| cx.emit(OAuthEvents::CloseLoginModal))
            .title("Spotify Login")
            .inner_size((520, 270))
            .anchor(Anchor::Center);
        }
    });

    Element::new(cx)
        .size(Stretch(1.0))
        .position_type(PositionType::Absolute)
        .backdrop_filter(Filter::Blur(Pixels(20.0).into()))
        .display(show_login_modal);
}
