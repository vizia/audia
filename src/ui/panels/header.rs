use crate::ui::data::PreferencesEvent;
use crate::ui::events::{OAuthUiEvent, PlaybackUiEvent, SearchUiEvent};
use vizia::{icons::ICON_SETTINGS, prelude::*};

pub fn header_panel(
    cx: &mut Context,
    search_input: Signal<String>,
    auth_username: Signal<String>,
    profile_image_key: Signal<Option<String>>,
) {
    HStack::new(cx, |cx| {
        Spacer::new(cx);
        Textbox::new(cx, search_input)
            .placeholder(Localized::new("search"))
            .on_edit(|cx, value| cx.emit(SearchUiEvent::SetInput(value)))
            .on_submit(|cx, value, _| cx.emit(SearchUiEvent::SubmitQuery(value)))
            .width(Stretch(2.0))
            .class("search-box");
        HStack::new(cx, |cx| {
            Button::new(cx, |cx| Svg::new(cx, ICON_SETTINGS).class("icon"))
                .class("playlist-shuffle-toggle")
                .on_press(|cx| cx.emit(PreferencesEvent::Show));

            let initials = auth_username.map(|name| {
                name.chars()
                    .find(|c| c.is_alphanumeric())
                    .map(|c| c.to_ascii_uppercase().to_string())
                    .unwrap_or_else(|| "?".to_string())
            });

            Submenu::new(
                cx,
                move |cx| {
                    Avatar::new(cx, move |cx| {
                        Binding::new(cx, profile_image_key, move |cx| {
                            if let Some(image_key) = profile_image_key.get() {
                                Image::new(cx, image_key).size(Stretch(1.0));
                            } else {
                                Label::new(cx, initials);
                            }
                        });
                    })
                    .size(Pixels(36.0))
                },
                |cx| {
                    MenuButton::new(
                        cx,
                        |cx| cx.emit(OAuthUiEvent::OpenLoginModal),
                        |cx| Label::new(cx, "Open Login"),
                    );
                    MenuButton::new(
                        cx,
                        |cx| cx.emit(OAuthUiEvent::RefreshToken),
                        |cx| Label::new(cx, "Refresh Token"),
                    );
                    MenuButton::new(
                        cx,
                        |cx| cx.emit(OAuthUiEvent::ResetLogin),
                        |cx| Label::new(cx, "Reset Login"),
                    );
                },
            )
            .class("profile-submenu");
        })
        .gap(Pixels(8.0))
        .height(Auto)
        .alignment(Alignment::Right);
    })
    .class("header");
}
