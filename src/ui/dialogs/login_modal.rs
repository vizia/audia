use crate::ui::events::OAuthUiEvent;
use vizia::prelude::*;

pub fn login_modal(cx: &mut Context, show_login_modal: Signal<bool>) {
    Binding::new(cx, show_login_modal, move |cx| {
        let is_open = show_login_modal.get();
        if is_open {
            Window::popup(cx, true, |cx| {
                VStack::new(cx, |cx| {
                    VStack::new(cx, |cx| {
                        Label::new(cx, "Login to Spotify")
                            .class("login-title")
                            .width(Stretch(1.0))
                            .alignment(Alignment::Center);

                        Label::new(
                            cx,
                            "Click Login with Spotify to continue. After approval in the browser, this window will close automatically.",
                        )
                        .alignment(Alignment::Center)
                        .width(Stretch(1.0));

                        Label::new(
                            cx,
                            "A Spotify premium account is required.",
                        )
                        .alignment(Alignment::Center)
                        .width(Stretch(1.0));
                    }).padding_top(Pixels(12.0)).alignment(Alignment::TopCenter).gap(Pixels(8.0));

                    HStack::new(cx, |cx| {
                        Button::new(cx, |cx| Label::new(cx, "Login with Spotify"))
                            .on_press(|cx| cx.emit(OAuthUiEvent::StartOAuthLogin))
                            .width(Pixels(170.0));

                        Button::new(cx, |cx| Label::new(cx, "Close"))
                            .variant(ButtonVariant::Secondary)
                            .on_press(|cx| cx.emit(OAuthUiEvent::CloseLoginModal))
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
            .on_close(|cx| cx.emit(OAuthUiEvent::CloseLoginModal))
            .title("Spotify Login")
            .inner_size((520, 220))
            .anchor(Anchor::Center);
        }
    });

    Element::new(cx)
        .size(Stretch(1.0))
        .position_type(PositionType::Absolute)
        .backdrop_filter(Filter::Blur(Pixels(20.0).into()))
        .display(show_login_modal);
}
