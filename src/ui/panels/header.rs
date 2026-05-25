use crate::ui::data::PreferencesEvent;
use crate::ui::events::{CenterEvents, OAuthEvents, SearchEvents};
use vizia::icons::{ICON_CHEVRON_LEFT, ICON_CHEVRON_RIGHT};
use vizia::{icons::ICON_SETTINGS, prelude::*};

pub fn header_panel(
    cx: &mut Context,
    search_input: Signal<String>,
    auth_username: Signal<String>,
    profile_image_key: Signal<Option<String>>,
    can_go_back: Signal<bool>,
    can_go_forward: Signal<bool>,
) {
    HStack::new(cx, |cx| {
        HStack::new(cx, |cx| {
            Button::new(cx, |cx| Svg::new(cx, ICON_CHEVRON_LEFT))
                .class("playback-skip-back")
                .disabled(can_go_back.map(|enabled| !*enabled))
                .tooltip(|cx| {
                    Tooltip::new(cx, |cx| {
                        Label::new(cx, Localized::new("back"));
                    })
                })
                .on_press(|cx| cx.emit(CenterEvents::NavigateBack));

            Button::new(cx, |cx| Svg::new(cx, ICON_CHEVRON_RIGHT))
                .class("playback-skip-forward")
                .disabled(can_go_forward.map(|enabled| !*enabled))
                .tooltip(|cx| {
                    Tooltip::new(cx, |cx| {
                        Label::new(cx, Localized::new("forward"));
                    })
                })
                .on_press(|cx| cx.emit(CenterEvents::NavigateForward));
        })
        .height(Auto)
        .width(Stretch(1.0))
        .alignment(Alignment::Right)
        .gap(Pixels(8.0));

        Textbox::new(cx, search_input)
            .placeholder(Localized::new("search"))
            .on_edit(|cx, value| cx.emit(SearchEvents::SetInput(value)))
            .on_submit(|cx, value, enter_key| {
                if enter_key {
                    cx.emit(SearchEvents::SubmitQuery(value));
                }
            })
            .width(Stretch(2.0))
            .class("search-box");
        HStack::new(cx, |cx| {
            Button::new(cx, |cx| Svg::new(cx, ICON_SETTINGS).class("icon"))
                .class("playlist-shuffle-toggle")
                .tooltip(|cx| {
                    Tooltip::new(cx, |cx| {
                        Label::new(cx, Localized::new("open_preferences"));
                    })
                })
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
                        |cx| cx.emit(OAuthEvents::OpenLoginModal),
                        |cx| Label::new(cx, Localized::new("open_login")),
                    )
                    .tooltip(|cx| {
                        Tooltip::new(cx, |cx| {
                            Label::new(cx, Localized::new("open_login_dialog"));
                        })
                    });
                    MenuButton::new(
                        cx,
                        |cx| cx.emit(OAuthEvents::RefreshToken),
                        |cx| Label::new(cx, Localized::new("refresh_token")),
                    )
                    .tooltip(|cx| {
                        Tooltip::new(cx, |cx| {
                            Label::new(cx, Localized::new("refresh_oauth_token"));
                        })
                    });
                    MenuButton::new(
                        cx,
                        |cx| cx.emit(OAuthEvents::ResetLogin),
                        |cx| Label::new(cx, Localized::new("reset_login")),
                    )
                    .tooltip(|cx| {
                        Tooltip::new(cx, |cx| {
                            Label::new(cx, Localized::new("reset_login_state"));
                        })
                    });
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
