use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::depot::{SteamShortcut, find_depot, steam_shortcuts};
use crate::gd::{Catalog, CatalogEntry, GdVersion, InstanceConfig, InstanceName, Settings};
use crate::instances::{DataDir, FoundInstall, Instance, InstanceError, InstanceStore};
use crate::launch::{find_proton, find_runtime};

use crate::download::Download;
use crate::open_folder::FolderOpener;

#[derive(Clone)]
pub(crate) enum Action {
    Create {
        name: Option<InstanceName>,
        entry: CatalogEntry,
    },
    Rename {
        from: InstanceName,
        to: InstanceName,
    },
    Delete(InstanceName),
    OpenFolder(InstanceName),
    OpenLink(String),
    OpenSaveFolder(InstanceName),
    Download(InstanceName),
    CancelDownload,
    SignOut,
    SetDefault(InstanceName),
    ToggleFavorite(InstanceName),
    Play(InstanceName),
    AddExisting(PathBuf),
    InstallGeode(InstanceName),
    RemoveGeode(InstanceName),
    Mod {
        instance: InstanceName,
        change: ModChange,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Notice {
    pub(crate) text: String,
    pub(crate) is_failure: bool,
}

impl Notice {
    fn done(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_failure: false,
        }
    }

    fn failed(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_failure: true,
        }
    }
}

pub(crate) type Notify = Arc<dyn Fn() + Send + Sync>;

pub(crate) struct Launcher {
    catalog: Catalog,
    store: Option<InstanceStore>,
    instances: Vec<Result<Instance, InstanceError>>,
    notices: Vec<Notice>,
    depot_downloader: Option<PathBuf>,
    settings: Settings,
    download: Option<Download>,
    notify: Notify,
    log: Vec<String>,
    steam_folder: Option<PathBuf>,
    proton: Option<PathBuf>,
    runtime: Option<PathBuf>,
    game: Option<Game>,
    found: Vec<FoundInstall>,
    shortcuts: Vec<SteamShortcut>,
    geode_task: Option<GeodeTask>,
    mod_picker: ModPicker,
    mods_cache: mods_cache::ModsCache,
    running: play::RunningGames,
    folder_opener: FolderOpener,
}

mod downloads;
mod existing;
mod folders;
mod geode;
mod mods;
mod mods_cache;
mod play;

use geode::GeodeTask;
pub(crate) use geode::ModLoader;
use mods::ModPicker;
pub(crate) use mods::{ModChange, ModsState};
use play::Game;

impl Launcher {
    pub(crate) fn new(notify: impl Fn() + Send + Sync + 'static) -> Self {
        let notify: Notify = Arc::new(notify);
        let data_dir = DataDir::from_platform_default();
        let steam_folder = crate::depot::steam_folder();
        let depot_downloader = data_dir
            .as_ref()
            .and_then(|data_dir| find_depot(&data_dir.tools_dir()));
        let store = data_dir.map(InstanceStore::new);
        let mut launcher = Self {
            catalog: Catalog::bundled(),
            store,
            instances: Vec::new(),
            notices: Vec::new(),
            depot_downloader,
            settings: Settings::default(),
            download: None,
            notify: Arc::clone(&notify),
            log: Vec::new(),
            proton: steam_folder.as_deref().and_then(find_proton),
            runtime: steam_folder.as_deref().and_then(find_runtime),
            shortcuts: steam_folder
                .as_deref()
                .map(steam_shortcuts)
                .unwrap_or_default(),
            steam_folder,
            game: None,
            found: Vec::new(),
            geode_task: None,
            mod_picker: ModPicker::default(),
            mods_cache: mods_cache::ModsCache::default(),
            running: play::RunningGames::watch(&notify),
            folder_opener: FolderOpener::default(),
        };
        if let Some(store) = &launcher.store {
            launcher.settings = store.data_dir().load_settings();
        }
        if launcher.store.is_none() {
            launcher.notices.push(Notice::failed("HOME isn't set"));
        }
        launcher.refresh();
        launcher.adopt_steam_install();
        launcher
    }

    pub(crate) fn notify(&self) -> Notify {
        Arc::clone(&self.notify)
    }

    pub(crate) fn depot_downloader(&self) -> Option<&Path> {
        self.depot_downloader.as_deref()
    }

    pub(crate) fn tools_dir(&self) -> Option<PathBuf> {
        Some(self.store.as_ref()?.data_dir().tools_dir())
    }

    pub(crate) fn steam_account(&self) -> Option<&str> {
        self.settings.steam_username.as_deref()
    }

    pub(crate) fn is_favorite(&self, name: &InstanceName) -> bool {
        self.settings.favorites.contains(name)
    }

    pub(crate) fn default_instance(&self) -> Option<&InstanceName> {
        self.settings.default_instance.as_ref()
    }

    fn sync_settings(&mut self, action: &Action) {
        let settings = &mut self.settings;
        let changed = match action {
            Action::Delete(name) => {
                let was = settings.favorites.len();
                settings.favorites.retain(|favorite| favorite != name);
                let was_default = settings.default_instance.as_ref() == Some(name);
                if was_default {
                    settings.default_instance = None;
                }
                was_default || was != settings.favorites.len()
            }
            Action::Rename { from, to } => {
                let mut changed = false;
                for favorite in &mut settings.favorites {
                    if favorite == from {
                        *favorite = to.clone();
                        changed = true;
                    }
                }
                if settings.default_instance.as_ref() == Some(from) {
                    settings.default_instance = Some(to.clone());
                    changed = true;
                }
                changed
            }
            _ => false,
        };
        if changed {
            self.save_settings();
        }
    }

    fn save_settings(&mut self) {
        let Some(store) = &self.store else {
            return;
        };
        if let Err(error) = store.data_dir().save_settings(&self.settings) {
            self.notices
                .push(Notice::failed(format!("Couldn't save settings: {error}")));
        }
    }

    pub(crate) fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    pub(crate) fn instances(&self) -> &[Result<Instance, InstanceError>] {
        &self.instances
    }

    pub(crate) fn take_notices(&mut self) -> Vec<Notice> {
        let notices = std::mem::take(&mut self.notices);
        if !notices.is_empty() {
            self.forget_mods();
        }
        for notice in &notices {
            self.push_log(notice.text.clone());
        }
        notices
    }

    pub(crate) fn log(&self) -> &[String] {
        &self.log
    }

    fn push_log(&mut self, line: String) {
        if self.log.len() == 2000 {
            self.log.remove(0);
        }
        self.log.push(line);
    }

    pub(crate) fn poll(&mut self) {
        self.poll_download();
        self.poll_game();
        self.poll_geode();
        self.poll_mod_picker();
        self.poll_mods();
    }

    pub(crate) fn apply(&mut self, action: Action) {
        self.forget_mods();
        if self.store.is_none() {
            return;
        }
        if let Action::Delete(name) = &action {
            self.note_deleted(name);
        }
        self.sync_settings(&action);
        let changes_instances = matches!(
            action,
            Action::Create { .. } | Action::Rename { .. } | Action::Delete(_)
        );
        let notice = match action {
            Action::Create { name, entry } => self.create(name, entry),
            Action::Rename { from, to } => self.rename(&from, to),
            Action::Delete(name) => self.delete(&name),
            Action::Download(name) => self.start_download(&name),
            Action::InstallGeode(name) => self.start_geode_install(&name),
            Action::RemoveGeode(name) => self.remove_geode_from(&name),
            Action::Mod { instance, change } => self.change_mods(&instance, change),
            Action::AddExisting(folder) => {
                let notice = self.add_existing(&folder, None);
                self.refresh_found();
                notice
            }
            Action::Play(name) => self.start_game(&name),
            Action::SignOut => {
                self.settings.steam_username = None;
                self.save_settings();
                Some(Notice::done("Signed out"))
            }
            Action::ToggleFavorite(name) => {
                let favorites = &mut self.settings.favorites;
                match favorites.iter().position(|favorite| *favorite == name) {
                    Some(position) => {
                        favorites.remove(position);
                    }
                    None => favorites.push(name),
                }
                self.save_settings();
                None
            }
            Action::SetDefault(name) => {
                self.settings.default_instance = Some(name);
                self.save_settings();
                None
            }
            Action::CancelDownload => {
                if let Some(download) = &self.download {
                    download.cancel();
                }
                None
            }
            Action::OpenFolder(name) => self.open_instance_folder(&name),
            Action::OpenLink(url) => self.open_link(&url),
            Action::OpenSaveFolder(name) => self.open_save_folder(&name),
        };
        self.notices.extend(notice);
        if changes_instances {
            self.refresh();
        }
    }

    fn create(&mut self, name: Option<InstanceName>, entry: CatalogEntry) -> Option<Notice> {
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
        let config = InstanceConfig::new(entry.version, entry.platform);
        Some(match store.create(name, config) {
            Ok(instance) => Notice::done(format!("Created \"{}\"", instance.name())),
            Err(error) => Notice::failed(format!("Couldn't create the instance: {error}")),
        })
    }

    fn rename(&mut self, from: &InstanceName, to: InstanceName) -> Option<Notice> {
        Some(match self.store.as_ref()?.rename(from, to) {
            Ok(instance) => Notice::done(format!("Renamed \"{from}\" to \"{}\"", instance.name())),
            Err(error) => Notice::failed(format!("Couldn't rename: {error}")),
        })
    }

    fn delete(&mut self, name: &InstanceName) -> Option<Notice> {
        Some(match self.store.as_ref()?.delete(name) {
            Ok(()) => Notice::done(format!("Deleted \"{name}\"")),
            Err(error) => Notice::failed(format!("Couldn't delete: {error}")),
        })
    }

    fn find(&self, name: &InstanceName) -> Option<&Result<Instance, InstanceError>> {
        self.instances.iter().find(|entry| match entry {
            Ok(instance) => instance.name() == name,
            Err(_) => false,
        })
    }

    pub(crate) fn short_folder(&self, instance: &Instance) -> String {
        if instance.is_linked() {
            return self.short_path(&instance.game_dir());
        }
        self.short_path(instance.folder())
    }

    pub(crate) fn default_name(&self, version: &GdVersion) -> Option<InstanceName> {
        self.store.as_ref()?.free_name(&version_name(version))
    }

    fn refresh(&mut self) {
        let Some(store) = &self.store else {
            return;
        };
        match store.list() {
            Ok(instances) => self.instances = instances,
            Err(error) => self
                .notices
                .push(Notice::failed(format!("Couldn't list instances: {error}"))),
        }
    }
}

fn version_name(version: &GdVersion) -> InstanceName {
    version
        .to_string()
        .parse()
        .expect("version is a valid name")
}
