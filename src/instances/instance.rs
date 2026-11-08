use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::gd::{InstanceConfig, InstanceName};

use crate::instances::GameFiles;
use crate::instances::game_files::INCOMPLETE_MARKER;

#[derive(Clone, PartialEq, Eq)]
pub struct Instance {
    name: InstanceName,
    config: InstanceConfig,
    folder: PathBuf,
    game_files: GameFiles,
}

impl Instance {
    pub(crate) fn new(name: InstanceName, config: InstanceConfig, folder: PathBuf) -> Self {
        let game_dir = game_dir(&config, &folder);
        let game_files = GameFiles::of(&game_dir);
        Self {
            name,
            config,
            folder,
            game_files,
        }
    }

    pub fn game_files(&self) -> GameFiles {
        self.game_files
    }

    pub fn mark_download_started(&self) -> io::Result<()> {
        fs::create_dir_all(self.game_dir())?;
        fs::write(self.game_dir().join(INCOMPLETE_MARKER), b"")
    }

    pub fn mark_download_finished(&self) -> io::Result<()> {
        match fs::remove_file(self.game_dir().join(INCOMPLETE_MARKER)) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        }
    }

    pub fn name(&self) -> &InstanceName {
        &self.name
    }

    pub fn config(&self) -> &InstanceConfig {
        &self.config
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    pub fn config_path(&self) -> PathBuf {
        config_path(&self.folder)
    }

    pub fn game_dir(&self) -> PathBuf {
        game_dir(&self.config, &self.folder)
    }

    pub fn is_linked(&self) -> bool {
        self.config.game_folder.is_some()
    }

    pub fn save_dir(&self) -> PathBuf {
        self.folder.join("save")
    }

    pub fn proton_dir(&self) -> PathBuf {
        self.folder.join("proton")
    }
}

const GAME_DIR: &str = "game";

fn game_dir(config: &InstanceConfig, folder: &Path) -> PathBuf {
    config
        .game_folder
        .clone()
        .unwrap_or_else(|| folder.join(GAME_DIR))
}

pub(crate) fn config_path(instance_folder: &Path) -> PathBuf {
    instance_folder.join("instance.txt")
}
