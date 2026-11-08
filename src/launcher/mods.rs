use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::gd::{GdPlatform, GdVersion, InstanceName};
use crate::instances::{GameFiles, Instance};
use crate::mods::{
    DllLoader, DllMod, GeodeMod, ModsFolder, add_dll_mod, echo_folder, find_mods_folder, has_geode,
    has_mega_hack, install_glint_loader, list_dll_mods, remove_dll_mod, remove_glint_loader,
    set_dll_mod_enabled, set_extensions_enabled, set_geode_mod_enabled, set_geode_mod_pinned,
    set_mega_hack_enabled,
};

use super::geode::FIRST_GEODE_VERSION;
use super::{Launcher, ModLoader, Notice};
use crate::folder_picker::FolderPicker;

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum ModsState {
    Geode {
        mods: Result<Rc<[GeodeMod]>, String>,
    },
    NeedsGeode,
    NoLoader {
        can_install: bool,
    },
    NoMacLoader,
    Dll {
        loader: DllLoader,
        mods: Result<Vec<DllMod>, String>,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum ModChange {
    InstallGlintLoader,
    RemoveGlintLoader,
    SetEnabled { file_name: String, enabled: bool },
    Add,
    Remove { file_name: String },
    OpenFolder,
    SetMegaHackEnabled { enabled: bool },
    SetExtensionsEnabled { enabled: bool },
    SetGeodeEnabled { id: String, enabled: bool },
    SetGeodePinned { id: String, pinned: bool },
    SetEchoEnabled { file_name: String, enabled: bool },
}

#[derive(Default)]
pub(super) struct ModPicker {
    picker: FolderPicker,
    instance: Option<InstanceName>,
}

impl ModsState {
    pub(crate) fn is_geode_grid(&self) -> bool {
        matches!(self, ModsState::Geode { mods: Ok(mods) } if !mods.is_empty())
    }
}

impl Launcher {
    pub(super) fn read_mods_state(
        &self,
        instance: &Instance,
        geode: &mut crate::mods::GeodeModsReader,
    ) -> ModsState {
        let game_dir = instance.game_dir();
        if matches!(
            self.mod_loader(instance),
            ModLoader::Geode | ModLoader::InstallingGeode
        ) {
            let mods = geode
                .list(&game_dir, &self.gd_save_dir(instance))
                .map(Rc::from)
                .map_err(|error| error.to_string());
            return ModsState::Geode { mods };
        }
        if let Some(ModsFolder { loader, path }) = find_mods_folder(&game_dir) {
            let mods = list_dll_mods(&path).map_err(|error| error.to_string());
            return ModsState::Dll { loader, mods };
        }
        if instance.config().version >= first_geode_version() {
            return ModsState::NeedsGeode;
        }
        if instance.config().platform == GdPlatform::MacOs {
            return ModsState::NoMacLoader;
        }
        ModsState::NoLoader {
            can_install: instance.game_files() == GameFiles::Present,
        }
    }

    pub(crate) fn is_picking_mod(&self) -> bool {
        self.mod_picker.picker.is_open()
    }

    pub(crate) fn can_pick_mod() -> bool {
        FolderPicker::is_available()
    }

    pub(super) fn change_mods(&mut self, name: &InstanceName, change: ModChange) -> Option<Notice> {
        let Some(Ok(instance)) = self.find(name) else {
            return Some(Notice::failed(format!("There's no instance \"{name}\"")));
        };
        let game_dir = instance.game_dir();
        let is_before_geode = instance.config().version < first_geode_version();
        // geode rewrites its settings when the game closes
        let is_geode_setting = matches!(
            change,
            ModChange::SetGeodeEnabled { .. } | ModChange::SetGeodePinned { .. }
        );
        if is_geode_setting && self.is_game_open(instance) {
            return Some(Notice::failed("Close the game first"));
        }
        if let ModChange::SetGeodePinned { id, pinned } = &change {
            let verb = if *pinned { "pin" } else { "unpin" };
            return set_geode_mod_pinned(&self.gd_save_dir(instance), id, *pinned)
                .err()
                .map(|error| Notice::failed(format!("Couldn't {verb} the mod: {error}")));
        }
        if let ModChange::SetGeodeEnabled { id, enabled } = &change {
            let verb = if *enabled { "turn on" } else { "turn off" };
            return set_geode_mod_enabled(&self.gd_save_dir(instance), id, *enabled)
                .err()
                .map(|error| Notice::failed(format!("Couldn't {verb} the mod: {error}")));
        }
        if let ModChange::SetMegaHackEnabled { enabled } = change {
            let verb = if enabled { "turn on" } else { "turn off" };
            return set_mega_hack_enabled(&game_dir, enabled)
                .err()
                .map(|error| Notice::failed(format!("Couldn't {verb} Mega Hack: {error}")));
        }
        if let ModChange::SetExtensionsEnabled { enabled } = change {
            let verb = if enabled { "turn on" } else { "turn off" };
            return set_extensions_enabled(&game_dir.join("extensions"), enabled)
                .err()
                .map(|error| Notice::failed(format!("Couldn't {verb} the extensions: {error}")));
        }
        if let ModChange::SetEchoEnabled { file_name, enabled } = &change {
            let verb = if *enabled { "turn on" } else { "turn off" };
            return set_dll_mod_enabled(&echo_folder(&game_dir), file_name, *enabled)
                .err()
                .map(|error| Notice::failed(format!("Couldn't {verb} Echo: {error}")));
        }
        let folder = find_mods_folder(&game_dir).map(|found| found.path);
        match (change, folder) {
            (ModChange::InstallGlintLoader, _) => {
                Some(install_loader(name, &game_dir, is_before_geode))
            }
            (ModChange::RemoveGlintLoader, _) => Some(match remove_glint_loader(&game_dir) {
                Ok(()) => Notice::done(format!("Removed Glint Loader from \"{name}\"")),
                Err(error) => Notice::failed(format!("Couldn't remove Glint Loader: {error}")),
            }),
            (_, None) => Some(Notice::failed(format!("Nothing loads mods in \"{name}\""))),
            (ModChange::SetEnabled { file_name, enabled }, Some(folder)) => {
                let verb = if enabled { "turn on" } else { "turn off" };
                set_dll_mod_enabled(&folder, &file_name, enabled)
                    .err()
                    .map(|error| Notice::failed(format!("Couldn't {verb} the mod: {error}")))
            }
            (ModChange::Remove { file_name }, Some(folder)) => {
                Some(match remove_dll_mod(&folder, &file_name) {
                    Ok(()) => {
                        let shown = DllMod::from_file_name(&file_name)
                            .map_or(file_name, |removed| removed.name);
                        Notice::done(format!("Removed {shown} from \"{name}\""))
                    }
                    Err(error) => Notice::failed(format!("Couldn't remove the mod: {error}")),
                })
            }
            (ModChange::Add, Some(_)) => self.pick_mod(name),
            (
                ModChange::SetEchoEnabled { .. }
                | ModChange::SetGeodeEnabled { .. }
                | ModChange::SetMegaHackEnabled { .. }
                | ModChange::SetExtensionsEnabled { .. }
                | ModChange::SetGeodePinned { .. },
                Some(_),
            ) => None,
            (ModChange::OpenFolder, Some(folder)) => match fs::create_dir_all(&folder) {
                Ok(()) => self.open_folder(&folder),
                Err(error) => Some(Notice::failed(format!(
                    "Couldn't open {}: {error}",
                    folder.display()
                ))),
            },
        }
    }

    fn pick_mod(&mut self, name: &InstanceName) -> Option<Notice> {
        if !Self::can_pick_mod() {
            return Some(Notice::failed(
                "No file picker found (install kdialog or zenity)",
            ));
        }
        let home = env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
        let downloads = home.join("Downloads");
        let start = if downloads.is_dir() { downloads } else { home };
        let notify = self.notify();
        self.mod_picker.picker.open_dll_file(&start, notify);
        self.mod_picker.instance = Some(name.clone());
        None
    }

    pub(super) fn poll_mod_picker(&mut self) {
        let Some(source) = self.mod_picker.picker.take_picked() else {
            return;
        };
        let Some(name) = self.mod_picker.instance.take() else {
            return;
        };
        let folder = match self.find(&name) {
            Some(Ok(instance)) => find_mods_folder(&instance.game_dir()),
            _ => None,
        };
        let notice = match folder.map(|folder| add_dll_mod(&folder.path, &source)) {
            Some(Ok(added)) => Notice::done(format!("Added {} to \"{name}\"", added.name)),
            Some(Err(error)) => Notice::failed(format!("Couldn't add the mod: {error}")),
            None => Notice::failed(format!("Nothing loads mods in \"{name}\"")),
        };
        self.notices.push(notice);
    }
}

fn first_geode_version() -> GdVersion {
    FIRST_GEODE_VERSION.parse().expect("a fixed version parses")
}

fn install_loader(name: &InstanceName, game_dir: &Path, is_before_geode: bool) -> Notice {
    // 2.2+ is 64-bit, glint loader is 32-bit
    if !is_before_geode {
        return Notice::failed("Glint Loader is only for pre-2.2");
    }
    if has_mega_hack(game_dir) || has_geode(game_dir) {
        return Notice::failed(format!("\"{name}\" already has a mod loader"));
    }
    match install_glint_loader(game_dir) {
        Ok(()) => Notice::done(format!("Installed Glint Loader into \"{name}\"")),
        Err(error) => Notice::failed(format!("Couldn't install Glint Loader: {error}")),
    }
}
