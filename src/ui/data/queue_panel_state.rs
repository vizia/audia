use std::io;
use std::path::PathBuf;

use ron::de::from_reader;
use serde::{Deserialize, Serialize};
use vizia::prelude::*;

pub const DEFAULT_LEFT_PANEL_WIDTH: f32 = 320.0;

pub const DEFAULT_RIGHT_PANEL_WIDTH: f32 = 300.0;

pub enum PanelEvent {
    SetLeftPanelWidth(f32),
    SetRightPanelWidth(f32),
}

#[derive(Default, Serialize, Deserialize)]
struct QueuePanelSnapshot {
    #[serde(default = "default_left_panel_width")]
    left_width: f32,
    #[serde(default = "default_right_panel_width")]
    right_width: f32,
}

fn default_left_panel_width() -> f32 {
    DEFAULT_LEFT_PANEL_WIDTH
}

fn default_right_panel_width() -> f32 {
    DEFAULT_RIGHT_PANEL_WIDTH
}

#[derive(Clone, Copy)]
pub struct PanelState {
    pub left_width: Signal<f32>,
    pub right_width: Signal<f32>,
}

impl PanelState {
    pub fn new(cx: &mut Context) -> Self {
        let data = Self {
            left_width: Signal::new(DEFAULT_LEFT_PANEL_WIDTH),
            right_width: Signal::new(DEFAULT_RIGHT_PANEL_WIDTH),
        };
        data.build(cx);
        data
    }

    fn state_path() -> io::Result<PathBuf> {
        let base = dirs::config_dir().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "Config directory unavailable")
        })?;
        Ok(base.join("audia").join("panel.ron"))
    }

    pub fn load(&mut self) {
        let Ok(path) = Self::state_path() else {
            return;
        };

        if let Ok(file) = std::fs::File::open(path) {
            if let Ok(saved) = from_reader::<_, QueuePanelSnapshot>(file) {
                self.left_width.set(saved.left_width);
                self.right_width.set(saved.right_width);
            }
        }
    }

    pub fn save(&self) {
        Self::save_width(
            self.left_width.get_untracked(),
            self.right_width.get_untracked(),
        );
    }

    pub fn save_width(left_width: f32, right_width: f32) {
        let Ok(path) = Self::state_path() else {
            return;
        };

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let snapshot = QueuePanelSnapshot {
            left_width: left_width,
            right_width: right_width,
        };

        if let Ok(data) = ron::ser::to_string_pretty(&snapshot, ron::ser::PrettyConfig::default()) {
            let _ = std::fs::write(path, data);
        }
    }
}

impl Model for PanelState {
    fn event(&mut self, _: &mut EventContext, event: &mut Event) {
        event.map(|window_event, _: &mut _| {
            if let WindowEvent::WindowClose = window_event {
                self.save();
            }
        });

        event.map(|panel_event, _| match panel_event {
            PanelEvent::SetLeftPanelWidth(width) => {
                self.left_width.set(*width);
            }
            PanelEvent::SetRightPanelWidth(width) => {
                self.right_width.set(*width);
            }
        });
    }
}
