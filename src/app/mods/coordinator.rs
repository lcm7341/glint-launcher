use super::install::first_geode_version;
use super::{ModChange, ModContext, ModLoader, ModsState};
use crate::app::{App, Notice};
use crate::instances::Instance;
use crate::instances::InstanceName;
use crate::mods::{
    DllMod, add_dll_mod, add_geode_mod_file, echo_folder, find_mods_folder, has_geode,
    has_mega_hack, install_glint_loader, remove_dll_mod, remove_glint_loader, set_dll_mod_enabled,
    set_extensions_enabled, set_geode_mod_enabled, set_geode_mod_pinned, set_mega_hack_enabled,
};
use crate::mods::{EchoMod, MegaHackLoader};
use crate::platform::native_game_running;
use std::fs;
use std::path::Path;

impl App {
    pub(in crate::app) fn change_mods(
        &mut self,
        name: &InstanceName,
        change: ModChange,
    ) -> Option<Notice> {
        if let ModChange::InstallFromIndex { id, name: mod_name } = change {
            return self.start_index_install(name, id, mod_name);
        }
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
        if let ModChange::Add { source } = &change
            && has_geode(&game_dir)
        {
            return Some(self.add_geode_file(name, source));
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
            (ModChange::Add { source }, Some(folder)) => {
                Some(match add_dll_mod(&folder, &source) {
                    Ok(added) => Notice::done(format!("Added {} to \"{name}\"", added.name)),
                    Err(error) => Notice::failed(format!("Couldn't add the mod: {error}")),
                })
            }
            (
                ModChange::SetEchoEnabled { .. }
                | ModChange::SetGeodeEnabled { .. }
                | ModChange::SetMegaHackEnabled { .. }
                | ModChange::SetExtensionsEnabled { .. }
                | ModChange::SetGeodePinned { .. }
                | ModChange::InstallFromIndex { .. },
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

    pub(in crate::app) fn may_be_game_open(&self, instance: &Instance) -> bool {
        self.is_game_open(instance)
            || (crate::platform::HostOs::CURRENT == crate::platform::HostOs::Windows
                && native_game_running().unwrap_or(false))
    }

    fn add_geode_file(&self, name: &InstanceName, source: &Path) -> Notice {
        let Some(Ok(instance)) = self.find(name) else {
            return Notice::failed(format!("There's no instance \"{name}\""));
        };
        if self.may_be_game_open(instance) {
            return Notice::failed("Close the game first");
        }
        let config = instance.config();
        let gd = &config.version;
        match add_geode_mod_file(&instance.game_dir(), source, gd, config.platform) {
            Ok(added) if added.is_made_for_gd => {
                Notice::done(format!("Added {} to \"{name}\"", added.name))
            }
            Ok(added) => {
                let made_for = match added.made_for {
                    Some(made_for) => format!("is made for GD {made_for}"),
                    None => format!("doesn't say it supports {}", config.platform.display_name()),
                };
                Notice::done(format!(
                    "Added {} to \"{name}\", but it {made_for}, not {gd}",
                    added.name
                ))
            }
            Err(error) => Notice::failed(format!("Couldn't add the mod: {error}")),
        }
    }
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

impl App {
    pub(in crate::app) fn forget_mods(&mut self) {
        self.mods.invalidate();
    }
    pub(in crate::app) fn poll_mods(&mut self) {
        let play = &self.play;
        let inputs = self
            .library
            .entries()
            .iter()
            .filter_map(|entry| entry.as_ref().ok())
            .map(|instance| (instance, play.gd_save_dir(instance)));
        self.mods.refresh(inputs);
    }
    pub(crate) fn mods_state(&self, instance: &Instance) -> ModsState {
        self.mods.state(instance)
    }
    pub(crate) fn echo(&self, instance: &Instance) -> Option<EchoMod> {
        self.mods.echo(instance)
    }
    pub(crate) fn mega_hack(&self, instance: &Instance) -> Option<MegaHackLoader> {
        self.mods.mega_hack(instance)
    }
    pub(crate) fn extensions_enabled(&self, instance: &Instance) -> bool {
        self.mods.extensions_enabled(instance)
    }
    pub(crate) fn mod_loader(&self, instance: &Instance) -> ModLoader {
        self.mods.mod_loader(instance)
    }
    pub(crate) fn installing_geode(&self) -> bool {
        self.mods.installing_geode()
    }
    pub(crate) fn installing_mod(&self, name: &InstanceName) -> Option<&str> {
        self.mods.installing_mod(name)
    }
    pub(in crate::app) fn start_geode_install(&mut self, name: &InstanceName) -> Option<Notice> {
        if self.mods.installing_geode() {
            return Some(Notice::failed("Already installing something"));
        }
        let instance = self.library.find(name)?.as_ref().ok()?;
        self.mods
            .start_loader(instance, &self.notify, &mut self.activity)
    }
    pub(in crate::app) fn start_index_install(
        &mut self,
        name: &InstanceName,
        id: String,
        mod_name: String,
    ) -> Option<Notice> {
        if self.mods.installing_geode() {
            return Some(Notice::failed("Already installing something"));
        }
        let Some(Ok(instance)) = self.library.find(name) else {
            return Some(Notice::failed(format!("There's no instance \"{name}\"")));
        };
        let context = ModContext {
            game_open: self.may_be_game_open(instance),
            notify: &self.notify,
        };
        self.mods
            .start_index_install(instance, id, mod_name, context, &mut self.activity)
    }
    pub(in crate::app) fn remove_geode_from(&mut self, name: &InstanceName) -> Option<Notice> {
        self.mods
            .remove_loader(self.library.find(name)?.as_ref().ok()?)
    }
}
