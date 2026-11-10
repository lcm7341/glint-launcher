use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::gd::{Catalog, GdVersion};
use crate::instances::InstanceName;
use crate::instances::{Instance, InstanceError};
use crate::paths::DataDir;
use crate::settings::{NewInstanceSave, Settings};

use crate::platform::FolderOpener;

pub(crate) use crate::task::Notify;

pub(crate) struct App {
    catalog: Catalog,
    library: InstanceLibrary,
    activity: Activity,
    downloads: Downloads,
    settings: Settings,
    notify: Notify,
    play: PlaySession,
    wine: WineSupport,
    mods: ModManager,
    folder_opener: FolderOpener,
    updater: Updater,
}

mod activity;
mod library;
mod messages;
use activity::Activity;
use library::InstanceLibrary;
pub(crate) use messages::{Action, Notice};

mod downloads;
mod existing;
mod folders;
mod mods;
mod play;
mod save_backups;
pub(crate) use save_backups::ago;
mod updates;
mod windows_save;
mod windows_steam;
mod wine;

use downloads::Downloads;
pub(crate) use mods::ModLoader;
use mods::ModManager;
pub(crate) use mods::{EchoState, ModChange, ModsState};
use play::PlaySession;
pub(crate) use play::{SaveMode, launch_blocker_message};
use updates::Updater;

use wine::WineSupport;

impl App {
    pub(crate) fn new(notify: impl Fn() + Send + Sync + 'static) -> Self {
        let notify: Notify = Arc::new(notify);
        let data_dir = DataDir::from_platform_default();
        let downloads = Downloads::new(data_dir.as_ref().map(DataDir::tools_dir));
        let wine = WineSupport::new(data_dir.clone(), Arc::clone(&notify));
        let library = InstanceLibrary::new(data_dir);
        let mut app = Self {
            catalog: Catalog::bundled(),
            library,
            activity: Activity::default(),
            downloads,
            settings: Settings::default(),
            notify: Arc::clone(&notify),
            play: PlaySession::new(&notify),
            wine,
            mods: ModManager::default(),
            folder_opener: FolderOpener::default(),
            updater: Updater::start(&notify),
        };
        if let Some(data_dir) = app.library.data_dir() {
            app.settings = Settings::load(data_dir);
        }
        if app.library.data_dir().is_none() {
            app.activity
                .notice(Notice::failed("Glint can't find its data folder"));
        }
        app.mods.fetch_echo(&notify);
        app.refresh();
        app.adopt_steam_install();
        app.finish_save_swap();
        app
    }

    pub(crate) fn notify(&self) -> Notify {
        Arc::clone(&self.notify)
    }

    pub(crate) fn depot_downloader(&self) -> Option<&Path> {
        self.downloads.program()
    }

    pub(crate) fn tools_dir(&self) -> Option<PathBuf> {
        Some(self.library.data_dir()?.tools_dir())
    }

    pub(crate) fn steam_account(&self) -> Option<&str> {
        self.settings.steam_username.as_deref()
    }

    pub(crate) fn is_favorite(&self, name: &InstanceName) -> bool {
        self.settings.favorites.contains(name)
    }

    pub(crate) fn new_instance_save(&self) -> NewInstanceSave {
        self.settings.new_instance_save
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
        let Some(data_dir) = self.library.data_dir() else {
            return;
        };
        if let Err(error) = self.settings.save(data_dir) {
            self.activity
                .notice(Notice::failed(format!("Couldn't save settings: {error}")));
        }
    }

    pub(crate) fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    pub(crate) fn instances(&self) -> &[Result<Instance, InstanceError>] {
        self.library.entries()
    }

    pub(crate) fn take_notices(&mut self) -> Vec<Notice> {
        let notices = self.activity.take_notices();
        if !notices.is_empty() {
            self.forget_mods();
        }
        notices
    }
    pub(crate) fn log(&self) -> &[String] {
        self.activity.log()
    }
    pub(crate) fn log_added(&self) -> usize {
        self.activity.log_added()
    }
    fn push_log(&mut self, line: String) {
        self.activity.push_log(line);
    }

    pub(crate) fn poll(&mut self) {
        self.mods.poll_echo(&mut self.activity);
        self.poll_depot_fetch();
        self.poll_download();
        self.poll_game();
        self.wine.poll(&mut self.activity);
        self.mods.poll_install(&mut self.activity);
        self.poll_mods();
        self.updater.poll(&mut self.activity);
    }

    pub(crate) fn apply(&mut self, action: Action) {
        self.forget_mods();
        if self.library.data_dir().is_none() || self.refuses_while_playing(&action) {
            return;
        }
        if let Action::Delete(name) = &action {
            self.note_deleted(name);
        }
        self.sync_settings(&action);
        let notice = match action {
            Action::Create {
                name,
                entry,
                linked,
            } => {
                let linked = linked && self.link_blocker(entry.platform, &entry.version).is_none();
                self.library.create(name, entry, linked, &mut self.activity)
            }
            Action::SetLinked { instance, linked } => {
                let blocker = match self.find(&instance) {
                    Some(Ok(found)) if linked => {
                        self.link_blocker(found.config().platform, &found.config().version)
                    }
                    _ => None,
                };
                if let Some(blocker) = blocker {
                    Some(Notice::failed(blocker))
                } else {
                    self.library
                        .set_linked(&instance, linked, &mut self.activity)
                }
            }
            Action::RestoreBackup { instance, backup } => self.restore_backup(&instance, &backup),
            Action::CopyMainSave(instance) => self.copy_main_save(&instance),
            Action::SetNewInstanceSave(save) => {
                self.settings.new_instance_save = save;
                self.save_settings();
                None
            }
            Action::Rename { from, to } => self.library.rename(&from, to, &mut self.activity),
            Action::Delete(name) => self.library.delete(&name, &mut self.activity),
            Action::Download(name) => self.start_download(&name),
            Action::InstallGeode(name) => self.start_geode_install(&name),
            Action::RemoveGeode(name) => self.remove_geode_from(&name),
            Action::Mod {
                instance,
                change: ModChange::InstallEcho,
            } => self.start_echo_install(&instance),
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
                self.downloads.cancel();
                None
            }
            Action::OpenFolder(name) => self.open_instance_folder(&name),
            Action::OpenLink(url) => self.open_link(&url),
            Action::OpenSaveFolder(name) => self.open_save_folder(&name),
            Action::InstallUpdate => self.start_update(),
        };
        self.activity.extend(notice);
    }

    fn find(&self, name: &InstanceName) -> Option<&Result<Instance, InstanceError>> {
        self.library.find(name)
    }

    pub(crate) fn short_folder(&self, instance: &Instance) -> String {
        if instance.is_imported() {
            return self.short_path(&instance.game_dir());
        }
        self.short_path(instance.folder())
    }

    pub(crate) fn default_name(&self, version: &GdVersion) -> Option<InstanceName> {
        self.library.default_name(version)
    }
    fn refresh(&mut self) {
        self.library.refresh(&mut self.activity);
    }
}
