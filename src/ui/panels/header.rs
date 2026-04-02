use crate::ui::data::PreferencesEvent;
use crate::ui::events::SearchUiEvent;
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
                .class("preferences-button")
                .variant(ButtonVariant::Text)
                .on_press(|cx| cx.emit(PreferencesEvent::Show));
            Binding::new(cx, profile_image_key, move |cx| {
                if let Some(image_key) = profile_image_key.get() {
                    Avatar::new(cx, |cx| {
                        Image::new(cx, image_key).size(Stretch(1.0));
                    })
                    .size(Pixels(36.0));
                } else {
                    let initials = auth_username.map(|name| {
                        name.chars()
                            .find(|c| c.is_alphanumeric())
                            .map(|c| c.to_ascii_uppercase().to_string())
                            .unwrap_or_else(|| "?".to_string())
                    });

                    Avatar::new(cx, |cx| {
                        Label::new(cx, initials);
                    })
                    .size(Pixels(36.0));
                }
            });
        })
        .gap(Pixels(8.0))
        .height(Auto)
        .alignment(Alignment::Right);
    })
    .class("header");
}
