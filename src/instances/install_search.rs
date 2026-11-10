use std::fs;
use std::path::{Path, PathBuf};

use crate::instances::InstanceConfig;

use crate::instances::recognize_install;

#[derive(Clone, PartialEq, Eq)]
pub struct FoundInstall {
    pub folder: PathBuf,
    pub config: InstanceConfig,
}

pub fn find_installs(folders: &[PathBuf]) -> Vec<FoundInstall> {
    let mut found: Vec<FoundInstall> = Vec::new();
    for folder in folders {
        for candidate in std::iter::once(folder.clone()).chain(subfolders(folder)) {
            let Ok(config) = recognize_install(&candidate) else {
                continue;
            };
            let folder = config
                .game_folder
                .clone()
                .unwrap_or_else(|| candidate.clone());
            if !found
                .iter()
                .any(|install| same_folder(&install.folder, &folder))
            {
                found.push(FoundInstall { folder, config });
            }
        }
    }
    found
}

pub fn same_folder(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn subfolders(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut folders: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    folders.sort();
    folders
}
