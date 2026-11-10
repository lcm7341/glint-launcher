use super::{Activity, Notice};
use crate::gd::{CatalogEntry, GdVersion};
use crate::instances::{
    FoundInstall, Instance, InstanceError, InstanceStore, find_installs, recognize_install,
    same_folder,
};
use crate::instances::{InstanceConfig, InstanceName};
use crate::paths::DataDir;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) struct InstanceLibrary {
    store: Option<InstanceStore>,
    instances: Vec<Result<Instance, InstanceError>>,
    found: Vec<FoundInstall>,
}

impl InstanceLibrary {
    pub(super) fn create(
        &mut self,
        name: Option<InstanceName>,
        entry: CatalogEntry,
        linked: bool,
        activity: &mut Activity,
    ) -> Option<Notice> {
        let notice = self.create_files(name, entry, linked);
        self.refresh(activity);
        notice
    }
    pub(super) fn rename(
        &mut self,
        from: &InstanceName,
        to: InstanceName,
        activity: &mut Activity,
    ) -> Option<Notice> {
        let notice = self.rename_files(from, to);
        self.refresh(activity);
        notice
    }
    pub(super) fn set_linked(
        &mut self,
        name: &InstanceName,
        linked: bool,
        activity: &mut Activity,
    ) -> Option<Notice> {
        let store = self.store.as_ref()?;
        let notice = match store.set_linked(name, linked) {
            Ok(_) if linked => Notice::done(format!("\"{name}\" uses your main save now")),
            Ok(_) => Notice::done(format!("\"{name}\" uses its own save now")),
            Err(error) => Notice::failed(format!("Couldn't change the save: {error}")),
        };
        self.refresh(activity);
        Some(notice)
    }
    pub(super) fn delete(
        &mut self,
        name: &InstanceName,
        activity: &mut Activity,
    ) -> Option<Notice> {
        let notice = self.delete_files(name);
        self.refresh(activity);
        notice
    }

    pub(super) fn new(data_dir: Option<DataDir>) -> Self {
        Self {
            store: data_dir.map(InstanceStore::new),
            instances: Vec::new(),
            found: Vec::new(),
        }
    }
    pub(super) fn data_dir(&self) -> Option<&DataDir> {
        self.store.as_ref().map(InstanceStore::data_dir)
    }
    pub(super) fn entries(&self) -> &[Result<Instance, InstanceError>] {
        &self.instances
    }
    pub(super) fn found(&self) -> &[FoundInstall] {
        &self.found
    }
    fn create_files(
        &mut self,
        name: Option<InstanceName>,
        entry: CatalogEntry,
        linked: bool,
    ) -> Option<Notice> {
        let store = self.store.as_ref()?;
        let name = match name {
            Some(name) => name,
            None => {
                let base = version_name(&entry.version);
                let Some(name) = store.free_name(&base) else {
                    return Some(Notice::failed(format!("Every name for {base} is taken")));
                };
                name
            }
        };
        let config = InstanceConfig::new(entry.version, entry.platform).with_linked(linked);
        Some(match store.create(name, config) {
            Ok(instance) => Notice::done(format!("Created \"{}\"", instance.name())),
            Err(error) => Notice::failed(format!("Couldn't create the instance: {error}")),
        })
    }

    fn rename_files(&mut self, from: &InstanceName, to: InstanceName) -> Option<Notice> {
        Some(match self.store.as_ref()?.rename(from, to) {
            Ok(instance) => Notice::done(format!("Renamed \"{from}\" to \"{}\"", instance.name())),
            Err(error) => Notice::failed(format!("Couldn't rename: {error}")),
        })
    }

    fn delete_files(&mut self, name: &InstanceName) -> Option<Notice> {
        Some(match self.store.as_ref()?.delete(name) {
            Ok(()) => Notice::done(format!("Deleted \"{name}\"")),
            Err(error) => Notice::failed(format!("Couldn't delete: {error}")),
        })
    }

    pub(super) fn find(&self, name: &InstanceName) -> Option<&Result<Instance, InstanceError>> {
        self.instances.iter().find(|entry| match entry {
            Ok(instance) => instance.name() == name,
            Err(_) => false,
        })
    }

    pub(super) fn default_name(&self, version: &GdVersion) -> Option<InstanceName> {
        self.store.as_ref()?.free_name(&version_name(version))
    }

    pub(super) fn refresh(&mut self, activity: &mut Activity) {
        let Some(store) = &self.store else {
            return;
        };
        match store.list() {
            Ok(instances) => self.instances = instances,
            Err(error) => {
                activity.notice(Notice::failed(format!("Couldn't list instances: {error}")))
            }
        }
    }
    // not at startup, reading desktop/documents/downloads pops a permission prompt on macos
    pub(crate) fn refresh_found(&mut self, steam_install: Option<PathBuf>) {
        let folders: Vec<PathBuf> = steam_install.into_iter().chain(search_folders()).collect();
        let imported: Vec<PathBuf> = self
            .instances
            .iter()
            .filter_map(|entry| entry.as_ref().ok()?.config().game_folder.clone())
            .collect();
        self.found = find_installs(&folders)
            .into_iter()
            .filter(|install| {
                !imported
                    .iter()
                    .any(|folder| same_folder(folder, &install.folder))
            })
            .collect();
    }

    pub(crate) fn adopt_steam_install(
        &mut self,
        steam_install: Option<PathBuf>,
        activity: &mut Activity,
    ) {
        let (Some(store), Some(install)) = (&self.store, steam_install) else {
            return;
        };
        if recognize_install(&install).is_err()
            || store.data_dir().steam_removed_path().exists()
            || self.is_imported_anywhere(&install)
        {
            return;
        }
        let name: InstanceName = "Steam".parse().expect("valid name");
        let Some(notice) = self.add_existing(&install, Some(name.clone()), activity) else {
            return;
        };
        let notice = match self.find(&name) {
            Some(Ok(instance)) if !notice.is_failure => {
                Notice::done(format!("Added Steam ({})", instance.config().version))
            }
            _ => notice,
        };
        activity.notice(notice);
    }

    pub(super) fn add_existing(
        &mut self,
        folder: &Path,
        name: Option<InstanceName>,
        activity: &mut Activity,
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
        self.refresh(activity);
        Some(notice)
    }

    pub(super) fn note_deleted(&mut self, name: &InstanceName, steam_install: Option<PathBuf>) {
        let Some(Ok(instance)) = self.find(name) else {
            return;
        };
        let is_steam = match (&instance.config().game_folder, steam_install) {
            (Some(folder), Some(steam)) => same_folder(folder, &steam),
            _ => false,
        };
        if is_steam && let Some(store) = &self.store {
            let _ = fs::write(store.data_dir().steam_removed_path(), b"");
        }
    }

    fn is_imported_anywhere(&self, install: &Path) -> bool {
        self.instances.iter().any(|entry| {
            entry
                .as_ref()
                .ok()
                .and_then(|instance| instance.config().game_folder.as_deref())
                .is_some_and(|folder| same_folder(folder, install))
        })
    }
}
fn version_name(version: &GdVersion) -> InstanceName {
    version
        .to_string()
        .parse()
        .expect("version is a valid name")
}

fn name_for_folder(folder: &Path, version: &str) -> InstanceName {
    folder
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.parse().ok())
        .unwrap_or_else(|| version.parse().expect("version is a valid name"))
}

fn search_folders() -> Vec<PathBuf> {
    let Some(home) = crate::platform::home_dir() else {
        return Vec::new();
    };
    ["Games", "Desktop", "Downloads", "Documents"]
        .into_iter()
        .map(|folder| home.join(folder))
        .collect()
}
