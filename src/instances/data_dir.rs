use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::gd::{InstanceName, Settings};

#[derive(Clone, PartialEq, Eq)]
pub struct DataDir {
    root: PathBuf,
}

impl DataDir {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn from_platform_default() -> Option<Self> {
        default_root().map(Self::new)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn settings_path(&self) -> PathBuf {
        self.root.join("settings.txt")
    }

    pub fn load_settings(&self) -> Settings {
        fs::read_to_string(self.settings_path())
            .ok()
            .and_then(|text| Settings::from_text(&text).ok())
            .unwrap_or_default()
    }

    pub fn save_settings(&self, settings: &Settings) -> io::Result<()> {
        fs::create_dir_all(&self.root)?;
        fs::write(self.settings_path(), settings.to_text())
    }

    pub fn steam_removed_path(&self) -> PathBuf {
        self.root.join("steam-instance-removed")
    }

    pub fn save_swap_lock_path(&self) -> PathBuf {
        self.root.join("save-swap.lock")
    }

    pub fn save_backups_dir(&self) -> PathBuf {
        self.root.join("save-backups")
    }

    pub fn tools_dir(&self) -> PathBuf {
        self.root.join("tools")
    }

    pub fn instances_dir(&self) -> PathBuf {
        self.root.join("instances")
    }

    pub fn instance_dir(&self, name: &InstanceName) -> PathBuf {
        self.instances_dir().join(name.as_str())
    }
}

const APP_FOLDER: &str = "glint";

fn non_empty_env(key: &str) -> Option<PathBuf> {
    env::var_os(key)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[cfg(target_os = "windows")]
fn default_root() -> Option<PathBuf> {
    non_empty_env("APPDATA").map(|app_data| app_data.join(APP_FOLDER))
}

#[cfg(target_os = "macos")]
fn default_root() -> Option<PathBuf> {
    non_empty_env("HOME").map(|home| {
        home.join("Library")
            .join("Application Support")
            .join(APP_FOLDER)
    })
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn default_root() -> Option<PathBuf> {
    non_empty_env("XDG_DATA_HOME")
        .or_else(|| non_empty_env("HOME").map(|home| home.join(".local").join("share")))
        .map(|data_home| data_home.join(APP_FOLDER))
}
