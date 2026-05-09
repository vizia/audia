use image::DynamicImage;
use vizia::{
    icons::{ICON_BRUSH, ICON_GLOBE, ICON_PLAYER_PLAY},
    prelude::*,
};

use crate::ui::data::{Preference, PreferencesData, PreferencesEvent, PreferencesPage};

fn page_tab<'a>(
    cx: &'a mut Context,
    page: PreferencesPage,
    selected: Signal<PreferencesPage>,
) -> Handle<'a, HStack> {
    HStack::new(cx, move |cx| {
        Svg::new(cx, page.icon())
            .size(Pixels(24.0))
            .class("icon")
            .hoverable(false);
        Label::new(cx, page.localized_name()).hoverable(false);
    })
    .alignment(Alignment::Left)
    .height(Pixels(32.0))
    .padding_left(Pixels(8.0))
    .padding_right(Pixels(8.0))
    .gap(Pixels(12.0))
    .class("page-tab")
    .toggle_class("selected", selected.map(move |x| *x == page))
    .on_press(move |cx| {
        cx.emit(PreferencesEvent::SetSelectedPage(page));
    })
}

fn nav_bar(cx: &mut Context, selected_page: Signal<PreferencesPage>) -> Handle<'_, VStack> {
    VStack::new(cx, |cx| {
        page_tab(cx, PreferencesPage::General, selected_page);
        page_tab(cx, PreferencesPage::Appearance, selected_page);
        page_tab(cx, PreferencesPage::Playback, selected_page);
    })
    .gap(Pixels(8.0))
}

fn settings_card_theme(cx: &mut Context, data: PreferencesData) {
    VStack::new(cx, |cx| {
        HStack::new(cx, |cx| {
            Svg::new(cx, ICON_BRUSH).class("icon");
            Label::new(cx, Localized::new("current_theme"));
            Spacer::new(cx);
            Select::new(cx, data.theme, data.selected_theme, true)
                .on_select(|cx, index| cx.emit(PreferencesEvent::SetSelectedTheme(index)))
                .width(Pixels(150.0));
        })
        .height(Auto)
        .class("settings-card-top");

        HStack::new(cx, |cx| {
            Element::new(cx).class("icon");
            Label::new(cx, Localized::new("follow_system_theme"));
            Spacer::new(cx);
            Switch::new(cx, data.follow_system_theme)
                .on_toggle(|cx| cx.emit(PreferencesEvent::ToggleUseSystemTheme));
        })
        .height(Auto)
        .class("settings-card-bottom");
    })
    .height(Auto)
    .gap(Pixels(1.0));
}

fn settings_card_language(cx: &mut Context, data: PreferencesData, preference: Preference) {
    HStack::new(cx, |cx| {
        Svg::new(cx, ICON_GLOBE).class("icon");
        Label::new(cx, preference.localized_name());
        Spacer::new(cx);
        Select::new(cx, data.language, data.selected_language, true)
            .on_select(|cx, index| cx.emit(PreferencesEvent::SetSelectedLanguage(index)))
            .width(Pixels(150.0));
    })
    .class("settings-card");
}

fn settings_card_autoplay(cx: &mut Context, data: PreferencesData) {
    HStack::new(cx, |cx| {
        Svg::new(cx, ICON_PLAYER_PLAY).class("icon");
        Label::new(cx, Localized::new("autoplay_on_queue_add"));
        Spacer::new(cx);
        Switch::new(cx, data.autoplay_on_queue_add)
            .on_toggle(|cx| cx.emit(PreferencesEvent::ToggleAutoplayOnQueueAdd));
    })
    .class("settings-card");
}

fn settings_card_restore_queue(cx: &mut Context, data: PreferencesData) {
    HStack::new(cx, |cx| {
        Svg::new(cx, ICON_PLAYER_PLAY).class("icon");
        Label::new(cx, Localized::new("restore_queue_on_startup"));
        Spacer::new(cx);
        Switch::new(cx, data.restore_queue_on_startup)
            .on_toggle(|cx| cx.emit(PreferencesEvent::ToggleRestoreQueueOnStartup));
    })
    .class("settings-card");
}

fn settings_page(
    cx: &mut Context,
    page: PreferencesPage,
    content: impl 'static + Fn(&mut Context),
) {
    VStack::new(cx, |cx| {
        Label::new(cx, page.localized_name()).class("page-title");
        ScrollView::new(cx, move |cx| {
            VStack::new(cx, |cx| {
                content(cx);
            })
            .height(Auto)
            .gap(Pixels(12.0))
            .padding_left(Pixels(24.0))
            .padding_right(Pixels(24.0));
        });
    })
    .padding_top(Pixels(12.0));
}

pub fn preferences_dialog(cx: &mut Context, icon: DynamicImage, data: PreferencesData) {
    Binding::new(cx, data.show, move |cx| {
        if data.show.get() {
            Window::popup(cx, true, move |cx| {
                HStack::new(cx, move |cx| {
                    VStack::new(cx, move |cx| {
                        Textbox::new(cx, data.search_string)
                            .on_edit(|cx, text| cx.emit(PreferencesEvent::SetSearch(text)))
                            .placeholder(Localized::new("search"))
                            .width(Stretch(1.0));
                        nav_bar(cx, data.selected_page)
                            .disabled(data.search_string.map(|s| !s.is_empty()));
                    })
                    .class("sidebar")
                    .padding(Pixels(12.0))
                    .gap(Pixels(12.0))
                    .width(Pixels(200.0));

                    let search_is_empty = data.search_string.map(|s| s.is_empty());
                    Binding::new(cx, search_is_empty, move |cx| {
                        if search_is_empty.get() {
                            Binding::new(cx, data.selected_page, move |cx| {
                                let selected_page = data.selected_page.get();
                                match selected_page {
                                    PreferencesPage::General => {
                                        settings_page(cx, selected_page, move |cx| {
                                            settings_card_language(cx, data, Preference::Language);
                                        });
                                    }
                                    PreferencesPage::Appearance => {
                                        settings_page(cx, selected_page, move |cx| {
                                            Label::new(cx, Localized::new("theming"));
                                            settings_card_theme(cx, data);
                                        });
                                    }
                                    PreferencesPage::Playback => {
                                        settings_page(cx, selected_page, move |cx| {
                                            settings_card_autoplay(cx, data);
                                            settings_card_restore_queue(cx, data);
                                        });
                                    }
                                }
                            });
                        } else {
                            Binding::new(cx, data.filtered_preferences, move |cx| {
                                let filtered = data.filtered_preferences.get();
                                VStack::new(cx, move |cx| {
                                    ScrollView::new(cx, move |cx| {
                                        VStack::new(cx, move |cx| {
                                            Label::new(cx, Localized::new("no_results_found")).display(
                                                data.filtered_preferences.map(|fp| {
                                                    if fp.is_empty() {
                                                        Display::Flex
                                                    } else {
                                                        Display::None
                                                    }
                                                }),
                                            );
                                            for preference in &filtered {
                                                match preference {
                                                    Preference::Language => {
                                                        settings_card_language(
                                                            cx,
                                                            data,
                                                            Preference::Language,
                                                        );
                                                    }
                                                    Preference::Theme => {
                                                        settings_card_theme(cx, data);
                                                    }
                                                    Preference::AutoplayOnQueueAdd => {
                                                        settings_card_autoplay(cx, data);
                                                    }
                                                    Preference::RestoreQueueOnStartup => {
                                                        settings_card_restore_queue(cx, data);
                                                    }
                                                }
                                            }
                                        })
                                        .height(Auto)
                                        .padding_left(Pixels(24.0))
                                        .padding_right(Pixels(24.0))
                                        .gap(Pixels(12.0));
                                    });
                                })
                                .padding_top(Pixels(12.0));
                            });
                        }
                    });
                });
                HStack::new(cx, |cx| {
                    Button::new(cx, |cx| Label::new(cx, Localized::new("ok")))
                        .on_press(|cx| {
                            cx.emit(PreferencesEvent::Hide);
                        })
                        .variant(ButtonVariant::Primary)
                        .width(Pixels(100.0));
                    Button::new(cx, |cx| Label::new(cx, Localized::new("cancel")))
                        .on_press(|cx| {
                            cx.emit(PreferencesEvent::Hide);
                        })
                        .variant(ButtonVariant::Outline)
                        .width(Pixels(100.0));
                })
                .height(Auto)
                .padding(Pixels(24.0))
                .gap(Pixels(12.0))
                .alignment(Alignment::Right);
            })
            .on_close(|cx| {
                cx.emit(PreferencesEvent::Hide);
            })
            .class("dialog")
            .title("Preferences")
            .inner_size((800, 600))
            .anchor(Anchor::Center)
            .enabled_window_buttons(WindowButtons::CLOSE)
            .icon(icon.width(), icon.height(), icon.clone().into_bytes());
        }
    });
}
