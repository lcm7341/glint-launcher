use std::fs;
use std::path::{Path, PathBuf};

use crate::gd::InstanceName;
use crate::instances::{FoundInstall, find_installs, recognize_install, same_folder};

use super::{Launcher, Notice};

impl Launcher {
    pub(crate) fn steam_install(&self) -> Option<PathBuf> {
        let install = self
            .steam_folder
            .as_ref()?
            .join("steamapps/common/Geometry Dash");
        install.is_dir().then_some(install)
    }

    pub(crate) fn found_installs(&self) -> &[FoundInstall] {
        &self.found
    }

    // not at startup, reading desktop/documents/downloads pops a permission prompt on macos
    pub(crate) fn refresh_found(&mut self) {
        let folders: Vec<PathBuf> = self
            .steam_install()
            .into_iter()
            .chain(search_folders())
            .collect();
        let linked: Vec<PathBuf> = self
            .instances
            .iter()
            .filter_map(|entry| entry.as_ref().ok()?.config().game_folder.clone())
            .collect();
        self.found = find_installs(&folders)
            .into_iter()
            .filter(|install| {
                !linked
                    .iter()
                    .any(|folder| same_folder(folder, &install.folder))
            })
            .collect();
    }

    pub(crate) fn adopt_steam_install(&mut self) {
        let (Some(store), Some(install)) = (&self.store, self.steam_install()) else {
            return;
        };
        if recognize_install(&install).is_err()
            || store.data_dir().steam_removed_path().exists()
            || self.is_linked_anywhere(&install)
        {
            return;
        }
        let name: InstanceName = "Steam".parse().expect("valid name");
        let Some(notice) = self.add_existing(&install, Some(name.clone())) else {
            return;
        };
        let notice = match self.find(&name) {
            Some(Ok(instance)) if !notice.is_failure => {
                Notice::done(format!("Added Steam ({})", instance.config().version))
            }
            _ => notice,
        };
        self.notices.push(notice);
    }

    pub(super) fn add_existing(
        &mut self,
        folder: &Path,
        name: Option<InstanceName>,
    ) -> Option<Notice> {
        let store = self.store.as_ref()?;
        let config = match recognize_install(folder) {
            Ok(config) => config,
            Err(error) => return Some(Notice::failed(format!("Couldn't add it: {error}"))),
        };
        let base = name.unwrap_or_else(|| name_for_folder(folder, &config.version.to_string()));
        let Some(name) = store.free_name(&base) else {
            return Some(Notice::failed(format!("Every name for {base} is taken")));
        };
        let version = config.version.clone();
        let notice = match store.create(name, config) {
            Ok(instance) => Notice::done(format!(
                "Added \"{}\" ({version}) at {}",
                instance.name(),
                folder.display()
            )),
            Err(error) => Notice::failed(format!("Couldn't add it: {error}")),
        };
        self.refresh();
        Some(notice)
    }

    pub(super) fn note_deleted(&mut self, name: &InstanceName) {
        let Some(Ok(instance)) = self.find(name) else {
            return;
        };
        let is_steam = match (&instance.config().game_folder, self.steam_install()) {
            (Some(folder), Some(steam)) => same_folder(folder, &steam),
            _ => false,
        };
        if is_steam && let Some(store) = &self.store {
            let _ = fs::write(store.data_dir().steam_removed_path(), b"");
        }
    }

    fn is_linked_anywhere(&self, install: &Path) -> bool {
        self.instances.iter().any(|entry| {
            entry
                .as_ref()
                .ok()
                .and_then(|instance| instance.config().game_folder.as_deref())
                .is_some_and(|folder| same_folder(folder, install))
        })
    }
}

fn name_for_folder(folder: &Path, version: &str) -> InstanceName {
    folder
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.parse().ok())
        .unwrap_or_else(|| version.parse().expect("version is a valid name"))
}

fn search_folders() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };
    ["Games", "Desktop", "Downloads", "Documents"]
        .into_iter()
        .map(|folder| home.join(folder))
        .collect()
}
