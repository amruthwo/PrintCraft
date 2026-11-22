//! Things PrintCraft remembers between runs: light/dark, content-pack
//! folders, recent files and the address book.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::theme::Look;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Contact {
    pub name: String,
    pub street: String,
    pub city: String,
    pub region: String,
    pub postcode: String,
    pub phone: String,
    pub email: String,
    /// "MM-DD" for the event reminder.
    pub birthday: String,
}

impl Contact {
    pub fn mailing_block(&self) -> String {
        let mut s = self.name.clone();
        if !self.street.is_empty() {
            s.push('\n');
            s.push_str(&self.street);
        }
        let line3 = format!("{} {} {}", if self.city.is_empty() { String::new() } else { format!("{},", self.city) }, self.region, self.postcode);
        if !line3.trim().is_empty() {
            s.push('\n');
            s.push_str(line3.trim());
        }
        s
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub look: Look,
    #[serde(default)]
    pub content_folders: Vec<PathBuf>,
    #[serde(default)]
    pub recent: Vec<PathBuf>,
    #[serde(default)]
    pub contacts: Vec<Contact>,
    #[serde(default = "yes")]
    pub show_rulers: bool,
    /// How strongly items pull toward guides, in screen points (0 = off).
    #[serde(default = "snap_default")]
    pub snap_strength: f32,
    /// Also snap to 1/16-inch ruler marks.
    #[serde(default)]
    pub snap_grid: bool,
    #[serde(default = "yes")]
    pub show_printable: bool,
    /// Edge most home printers can't reach, in inches.
    #[serde(default = "margin_default")]
    pub printable_margin: f32,
    #[serde(default = "autosave_default")]
    pub autosave_minutes: u32,
    #[serde(default)]
    pub modern: bool,
    #[serde(default)]
    pub recent_colors: Vec<crate::model::Rgba>,
    #[serde(default)]
    pub ai_provider: String,
    #[serde(default)]
    pub ai_key: String,
    /// Undo steps kept (never fewer than 10).
    #[serde(default = "undo_default")]
    pub undo_steps: usize,
    /// Match the computer's light or dark setting.
    #[serde(default = "yes")]
    pub follow_system: bool,
    /// Legacy Pages: one page at a time with page buttons along the bottom.
    /// Off (the default) shows every page in the workspace.
    #[serde(default)]
    pub legacy_pages: bool,
    /// With every page shown: lay them side by side (tiled) instead of one
    /// above the other.
    #[serde(default)]
    pub tile_pages: bool,
    /// Show the guidelines placed from the rulers.
    #[serde(default = "yes")]
    pub show_guides: bool,
    /// Look for a new version on GitHub once a day.
    #[serde(default = "yes")]
    pub check_updates: bool,
    /// When that last happened (milliseconds since 1970).
    #[serde(default)]
    pub last_update_check: u64,
    /// A version the person chose to skip.
    #[serde(default)]
    pub skip_version: String,
}

fn undo_default() -> usize {
    100
}

fn snap_default() -> f32 {
    8.0
}
fn margin_default() -> f32 {
    0.25
}
fn autosave_default() -> u32 {
    2
}

fn yes() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            look: Look::Light,
            content_folders: Vec::new(),
            recent: Vec::new(),
            contacts: Vec::new(),
            show_rulers: true,
            snap_strength: snap_default(),
            snap_grid: false,
            show_printable: true,
            printable_margin: margin_default(),
            autosave_minutes: autosave_default(),
            modern: false,
            recent_colors: Vec::new(),
            ai_provider: String::new(),
            ai_key: String::new(),
            undo_steps: undo_default(),
            follow_system: true,
            legacy_pages: false,
            tile_pages: false,
            show_guides: true,
            check_updates: true,
            last_update_check: 0,
            skip_version: String::new(),
        }
    }
}

impl Settings {
    /// Settings are a small JSON file on the desktop (the browser version
    /// keeps them in the browser's storage for the site).
    pub fn load() -> Settings {
        crate::platform::load_settings().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }
    pub fn save(&self) {
        if let Ok(b) = serde_json::to_vec_pretty(self) {
            crate::platform::store_settings(&b);
        }
    }
    pub fn remember_color(&mut self, c: crate::model::Rgba) {
        if c.is_clear() {
            return;
        }
        self.recent_colors.retain(|r| *r != c);
        self.recent_colors.insert(0, c);
        self.recent_colors.truncate(10);
        self.save();
    }

    pub fn remember(&mut self, p: PathBuf) {
        self.recent.retain(|r| r != &p);
        self.recent.insert(0, p);
        self.recent.truncate(8);
    }
}
