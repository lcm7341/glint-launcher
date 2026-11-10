use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::instances::Instance;
use crate::mods::{EchoMod, GeodeModsReader, MegaHackLoader};

use super::{ModLoader, ModsState};
use crate::gd::GdPlatform;
use crate::instances::GameFiles;
use crate::mods::{ModsFolder, find_mods_folder, list_dll_mods};

#[derive(Clone)]
struct ModsSnapshot {
    state: ModsState,
    echo: Option<EchoMod>,
    mega_hack: Option<MegaHackLoader>,
    extensions_on: bool,
}

#[derive(Default)]
pub(super) struct ModsCache {
    geode: GeodeModsReader,
    read: HashMap<PathBuf, ModsSnapshot>,
    read_at: Option<Instant>,
}

impl ModsCache {
    pub(super) fn invalidate(&mut self) {
        self.read_at = None;
    }
    pub(super) fn is_fresh(&self) -> bool {
        self.read_at
            .is_some_and(|at| at.elapsed() < Duration::from_secs(2))
    }
    pub(super) fn refresh<'a>(
        &mut self,
        instances: impl IntoIterator<Item = (&'a Instance, PathBuf, ModLoader)>,
    ) {
        let mut geode = std::mem::take(&mut self.geode);
        self.read = instances
            .into_iter()
            .map(|(instance, save_dir, loader)| {
                let game_dir = instance.game_dir();
                let snapshot = ModsSnapshot {
                    state: read_mods_state(instance, &mut geode, loader, &save_dir),
                    echo: crate::mods::find_echo(&game_dir),
                    mega_hack: crate::mods::find_mega_hack_loader(&game_dir),
                    extensions_on: crate::mods::extensions_enabled(&game_dir.join("extensions")),
                };
                (game_dir, snapshot)
            })
            .collect();
        self.geode = geode;
        self.read_at = Some(Instant::now());
    }
    pub(super) fn mods_state(&self, instance: &Instance) -> ModsState {
        self.snapshot(instance)
            .map_or(ModsState::NoLoader { can_install: false }, |snapshot| {
                snapshot.state.clone()
            })
    }

    pub(super) fn echo(&self, instance: &Instance) -> Option<EchoMod> {
        self.snapshot(instance)?.echo.clone()
    }

    pub(super) fn mega_hack(&self, instance: &Instance) -> Option<MegaHackLoader> {
        self.snapshot(instance)?.mega_hack.clone()
    }

    pub(super) fn extensions_enabled(&self, instance: &Instance) -> bool {
        self.snapshot(instance)
            .is_some_and(|snapshot| snapshot.extensions_on)
    }

    fn snapshot(&self, instance: &Instance) -> Option<&ModsSnapshot> {
        self.read.get(&instance.game_dir())
    }
}
fn read_mods_state(
    instance: &Instance,
    geode: &mut crate::mods::GeodeModsReader,
    loader: ModLoader,
    save_dir: &std::path::Path,
) -> ModsState {
    let game_dir = instance.game_dir();
    if matches!(loader, ModLoader::Geode | ModLoader::InstallingGeode) {
        let mods = geode
            .list(&game_dir, save_dir)
            .map(Rc::from)
            .map_err(|error| error.to_string());
        return ModsState::Geode { mods };
    }
    if let Some(ModsFolder { loader, path }) = find_mods_folder(&game_dir) {
        let mods = list_dll_mods(&path).map_err(|error| error.to_string());
        return ModsState::Dll { loader, mods };
    }
    if instance.config().version >= super::install::first_geode_version() {
        return ModsState::NeedsGeode;
    }
    if instance.config().platform == GdPlatform::MacOs {
        return ModsState::NoMacLoader;
    }
    ModsState::NoLoader {
        can_install: instance.game_files() == GameFiles::Present,
    }
}
