use std::io;
use std::path::PathBuf;

use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use ron::de::from_reader;
use serde::{Deserialize, Serialize};
use vizia::{
    icons::{ICON_PALETTE, ICON_PLAYER_PLAY, ICON_SETTINGS},
    prelude::*,
};

#[derive(Default, Serialize, Deserialize)]
struct PreferencesSnapshot {
    #[serde(default)]
    search_string: String,
    #[serde(default)]
    selected_language: Option<usize>,
    #[serde(default)]
    selected_theme: Option<usize>,
    #[serde(default)]
    follow_system_theme: bool,
    #[serde(default)]
    autoplay_on_queue_add: bool,
    #[serde(default)]
    restore_queue_on_startup: bool,
}

/// Data model for the Preferences Dialog
#[derive(Clone, Copy)]
pub struct PreferencesData {
    pub show: Signal<bool>,

    pub selected_page: Signal<PreferencesPage>,
    pub search_string: Signal<String>,
    pub preferences: Signal<Vec<Preference>>,
    pub filtered_preferences: Signal<Vec<Preference>>,

    // General Page
    pub language: Signal<Vec<String>>,
    pub selected_language: Signal<Option<usize>>,

    // Appearance Page
    pub theme: Signal<Vec<String>>,
    pub selected_theme: Signal<Option<usize>>,
    pub follow_system_theme: Signal<bool>,

    // Playback Page
    pub autoplay_on_queue_add: Signal<bool>,
    pub restore_queue_on_startup: Signal<bool>,
}

impl PreferencesData {
    /// Create a new PreferencesData
    pub fn new() -> Self {
        Self {
            show: Signal::new(false),
            selected_page: Signal::new(PreferencesPage::General),
            search_string: Signal::new(String::new()),
            preferences: Signal::new(vec![
                Preference::Language,
                Preference::Theme,
                Preference::AutoplayOnQueueAdd,
                Preference::RestoreQueueOnStartup,
            ]),
            filtered_preferences: Signal::new(vec![]),

            // General Page
            language: Signal::new(vec![
                String::from("System Default"),
                String::from("English (UK)"),
                String::from("English (US)"),
            ]),
            selected_language: Signal::new(Some(0)),

            // Appearance Page
            theme: Signal::new(vec![String::from("Light"), String::from("Dark")]),
            selected_theme: Signal::new(Some(1)),
            follow_system_theme: Signal::new(false),

            // Playback Page
            autoplay_on_queue_add: Signal::new(true),
            restore_queue_on_startup: Signal::new(false),
        }
    }

    /// Search the preferences
    fn search(&mut self, cx: &mut EventContext) {
        let search = self.search_string.get().to_lowercase();

        let matcher = SkimMatcherV2::default().ignore_case();
        let mut filtered_preferences = self.preferences.with(|prefs| {
            prefs
                .iter()
                .filter_map(|preference| {
                    let name = preference
                        .localized_tags()
                        .to_string_local(cx)
                        .to_lowercase();
                    matcher
                        .fuzzy_match(&name, &search)
                        .map(|score| (*preference, score))
                })
                .collect::<Vec<_>>()
        });

        filtered_preferences.sort_by_key(|a| std::cmp::Reverse(a.1));

        self.filtered_preferences
            .set(filtered_preferences.iter().map(|(p, _)| *p).collect());
    }

    fn preferences_path() -> io::Result<PathBuf> {
        let base = dirs::config_dir().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "Config directory unavailable")
        })?;
        Ok(base.join("audia").join("preferences.ron"))
    }

    pub fn save(&self) {
        let Ok(path) = Self::preferences_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let snapshot = PreferencesSnapshot {
            search_string: self.search_string.get_untracked(),
            selected_language: self.selected_language.get_untracked(),
            selected_theme: self.selected_theme.get_untracked(),
            follow_system_theme: self.follow_system_theme.get_untracked(),
            autoplay_on_queue_add: self.autoplay_on_queue_add.get_untracked(),
            restore_queue_on_startup: self.restore_queue_on_startup.get_untracked(),
        };

        if let Ok(data) = ron::ser::to_string_pretty(&snapshot, ron::ser::PrettyConfig::default()) {
            let _ = std::fs::write(path, data);
        }
    }

    pub fn load(&mut self, cx: &mut Context) {
        let Ok(path) = Self::preferences_path() else {
            return;
        };

        if let Ok(f) = std::fs::File::open(path) {
            if let Ok(saved) = from_reader::<_, PreferencesSnapshot>(f) {
                self.search_string.set(saved.search_string);
                self.selected_language.set(saved.selected_language);
                self.selected_theme.set(saved.selected_theme);
                cx.emit(EnvironmentEvent::SetThemeMode(
                    if let Some(theme) = saved.selected_theme {
                        match theme {
                            0 => ThemeMode::LightMode,
                            1 => ThemeMode::DarkMode,
                            _ => unreachable!(),
                        }
                    } else {
                        ThemeMode::System
                    },
                ));

                self.follow_system_theme.set(saved.follow_system_theme);
                self.autoplay_on_queue_add.set(saved.autoplay_on_queue_add);
                self.restore_queue_on_startup
                    .set(saved.restore_queue_on_startup);
            }
        }
    }
}

/// The different pages in the Preferences Dialog
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum PreferencesPage {
    #[default]
    General,
    Appearance,
    Playback,
}

impl_res_simple!(PreferencesPage);

impl PreferencesPage {
    pub fn localized_name(&self) -> Localized {
        match self {
            PreferencesPage::General => Localized::new("general"),
            PreferencesPage::Appearance => Localized::new("appearance"),
            PreferencesPage::Playback => Localized::new("playback"),
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            PreferencesPage::General => ICON_SETTINGS,
            PreferencesPage::Appearance => ICON_PALETTE,
            PreferencesPage::Playback => ICON_PLAYER_PLAY,
        }
    }
}

/// The different preferences
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Preference {
    Language,
    Theme,
    AutoplayOnQueueAdd,
    RestoreQueueOnStartup,
}

impl Preference {
    pub fn localized_name(&self) -> Localized {
        match self {
            Preference::Language => Localized::new("language"),
            Preference::Theme => Localized::new("theme"),
            Preference::AutoplayOnQueueAdd => Localized::new("autoplay_on_queue_add"),
            Preference::RestoreQueueOnStartup => Localized::new("restore_queue_on_startup"),
        }
    }

    pub fn localized_tags(&self) -> Localized {
        match self {
            Preference::Language => Localized::new("language_tags"),
            Preference::Theme => Localized::new("theme_tags"),
            Preference::AutoplayOnQueueAdd => Localized::new("autoplay_on_queue_add_tags"),
            Preference::RestoreQueueOnStartup => Localized::new("restore_queue_on_startup_tags"),
        }
    }
}

/// Events for the Preferences Dialog
pub enum PreferencesEvent {
    Show,
    Hide,
    SetSelectedPage(PreferencesPage),
    SetSearch(String),
    SetSelectedLanguage(usize),
    SetSelectedTheme(usize),
    ToggleUseSystemTheme,
    ToggleAutoplayOnQueueAdd,
    ToggleRestoreQueueOnStartup,
}

impl Model for PreferencesData {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|preferences_event, _: &mut _| match preferences_event {
            PreferencesEvent::Show => {
                self.show.set(true);
            }
            PreferencesEvent::Hide => {
                self.show.set(false);
                self.save();
            }
            PreferencesEvent::SetSelectedPage(selected_page) => {
                self.selected_page.set(*selected_page);
            }
            PreferencesEvent::SetSearch(search) => {
                self.search_string.set(search.clone());
                self.search(cx);
            }
            PreferencesEvent::SetSelectedLanguage(selected_language) => {
                self.selected_language.set(Some(*selected_language));
                let idx = *selected_language;
                let lang = self
                    .language
                    .with(|langs| match langs.get(idx).map(|s| s.as_str()) {
                        Some("System Default") => langid!("en-GB"),
                        Some("English") => langid!("en-GB"),
                        Some("Deutsch") => langid!("de"),
                        Some("Español") => langid!("es"),
                        Some("Français") => langid!("fr"),
                        _ => langid!("en-GB"),
                    });
                cx.emit(EnvironmentEvent::SetLocale(lang));
                self.save();
            }
            PreferencesEvent::SetSelectedTheme(selected_theme) => {
                self.selected_theme.set(Some(*selected_theme));
                self.follow_system_theme.set(false);
                cx.emit(EnvironmentEvent::SetThemeMode(if *selected_theme == 0 {
                    ThemeMode::LightMode
                } else {
                    ThemeMode::DarkMode
                }));
                self.save();
            }
            PreferencesEvent::ToggleUseSystemTheme => {
                self.follow_system_theme.update(|v| *v = !*v);
                cx.emit(EnvironmentEvent::SetThemeMode(ThemeMode::System));
                self.save();
            }
            PreferencesEvent::ToggleAutoplayOnQueueAdd => {
                self.autoplay_on_queue_add.update(|v| *v = !*v);
                self.save();
            }
            PreferencesEvent::ToggleRestoreQueueOnStartup => {
                self.restore_queue_on_startup.update(|v| *v = !*v);
                self.save();
            }
        });

        event.map(|window_event, _: &mut _| {
            if let WindowEvent::WindowClose = window_event {
                self.save();
            }
        });
    }
}
