use std::io;
use std::path::PathBuf;

use ron::de::from_reader;
use serde::{Deserialize, Serialize};
use vizia::prelude::*;

pub const DEFAULT_LEFT_PANEL_WIDTH: f32 = 320.0;

pub const DEFAULT_RIGHT_PANEL_WIDTH: f32 = 300.0;

pub const DEFAULT_WINDOW_WIDTH: u32 = 1200;

pub const DEFAULT_WINDOW_HEIGHT: u32 = 800;

pub const DEFAULT_WINDOW_X: i32 = 0;

pub const DEFAULT_WINDOW_Y: i32 = 0;

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
    #[serde(default = "default_window_width")]
    window_width: u32,
    #[serde(default = "default_window_height")]
    window_height: u32,
    #[serde(default = "default_window_x")]
    window_x: i32,
    #[serde(default = "default_window_y")]
    window_y: i32,
}

fn default_left_panel_width() -> f32 {
    DEFAULT_LEFT_PANEL_WIDTH
}

fn default_right_panel_width() -> f32 {
    DEFAULT_RIGHT_PANEL_WIDTH
}

fn default_window_width() -> u32 {
    DEFAULT_WINDOW_WIDTH
}

fn default_window_height() -> u32 {
    DEFAULT_WINDOW_HEIGHT
}

fn default_window_x() -> i32 {
    DEFAULT_WINDOW_X
}

fn default_window_y() -> i32 {
    DEFAULT_WINDOW_Y
}

#[derive(Clone, Copy)]
pub struct PanelState {
    pub left_width: Signal<f32>,
    pub right_width: Signal<f32>,
    pub window_width: Signal<u32>,
    pub window_height: Signal<u32>,
    pub window_x: Signal<i32>,
    pub window_y: Signal<i32>,
}

impl PanelState {
    pub fn new() -> Self {
        let data = Self {
            left_width: Signal::new(DEFAULT_LEFT_PANEL_WIDTH),
            right_width: Signal::new(DEFAULT_RIGHT_PANEL_WIDTH),
            window_width: Signal::new(DEFAULT_WINDOW_WIDTH as u32),
            window_height: Signal::new(DEFAULT_WINDOW_HEIGHT as u32),
            window_x: Signal::new(DEFAULT_WINDOW_X as i32),
            window_y: Signal::new(DEFAULT_WINDOW_Y as i32),
        };

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
                self.window_width.set(saved.window_width);
                self.window_height.set(saved.window_height);
                self.window_x.set(saved.window_x);
                self.window_y.set(saved.window_y);
            }
        }
    }

    pub fn save(&self) {
        let Ok(path) = Self::state_path() else {
            return;
        };

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let snapshot = QueuePanelSnapshot {
            left_width: self.left_width.get_untracked(),
            right_width: self.right_width.get_untracked(),
            window_width: self.window_width.get_untracked(),
            window_height: self.window_height.get_untracked(),
            window_x: self.window_x.get_untracked(),
            window_y: self.window_y.get_untracked(),
        };

        if let Ok(data) = ron::ser::to_string_pretty(&snapshot, ron::ser::PrettyConfig::default()) {
            let _ = std::fs::write(path, data);
        }
    }
}

impl Model for PanelState {
    fn event(&mut self, ex: &mut EventContext, event: &mut Event) {
        event.map(|window_event, _: &mut _| match window_event {
            WindowEvent::WindowClose => {
                if let Some(window) = ex.window() {
                    let pos = window.outer_position().unwrap_or_default();
                    let size = window.outer_size();
                    let pos = pos.to_logical(ex.scale_factor() as f64);
                    let size = size.to_logical(ex.scale_factor() as f64);

                    self.window_width.set(size.width);
                    self.window_height.set(size.height);
                    self.window_x.set(pos.x);
                    self.window_y.set(pos.y);
                }
                self.save();
            }

            _ => {}
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
