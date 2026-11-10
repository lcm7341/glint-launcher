use std::path::{Path, PathBuf};

use crate::instances::InstanceName;

#[derive(Clone, PartialEq, Eq)]
pub struct DataDir {
    root: PathBuf,
}

impl DataDir {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn from_platform_default() -> Option<Self> {
        crate::platform::data_dir().map(Self::new)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn settings_path(&self) -> PathBuf {
        self.root.join("settings.txt")
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

    pub fn main_save_backups_dir(&self) -> PathBuf {
        self.save_backups_dir().join("auto").join("main-save")
    }

    pub fn own_save_backups_dir(&self, name: &InstanceName) -> PathBuf {
        self.save_backups_dir()
            .join("auto")
            .join("instances")
            .join(name.as_str())
    }

    pub fn main_save_version_path(&self) -> PathBuf {
        self.root.join("main-save-version")
    }

    pub fn tools_dir(&self) -> PathBuf {
        self.root.join("tools")
    }

    pub fn wine_dir(&self) -> PathBuf {
        self.root.join("wine")
    }

    pub fn instances_dir(&self) -> PathBuf {
        self.root.join("instances")
    }

    pub fn instance_dir(&self, name: &InstanceName) -> PathBuf {
        self.instances_dir().join(name.as_str())
    }
}
