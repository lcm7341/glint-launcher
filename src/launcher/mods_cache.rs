use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::instances::Instance;
use crate::mods::{EchoMod, GeodeModsReader, MegaHackLoader};

use super::{Launcher, ModsState};

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

impl Launcher {
    pub(super) fn forget_mods(&mut self) {
        self.mods_cache.read_at = None;
    }

    pub(super) fn poll_mods(&mut self) {
        if self
            .mods_cache
            .read_at
            .is_some_and(|read_at| read_at.elapsed() < Duration::from_secs(2))
        {
            return;
        }
        let mut geode = std::mem::take(&mut self.mods_cache.geode);
        let read = self
            .instances
            .iter()
            .filter_map(|entry| entry.as_ref().ok())
            .map(|instance| {
                let game_dir = instance.game_dir();
                let snapshot = ModsSnapshot {
                    state: self.read_mods_state(instance, &mut geode),
                    echo: crate::mods::find_echo(&game_dir),
                    mega_hack: crate::mods::find_mega_hack_loader(&game_dir),
                    extensions_on: crate::mods::extensions_enabled(&game_dir.join("extensions")),
                };
                (game_dir, snapshot)
            })
            .collect();
        self.mods_cache = ModsCache {
            geode,
            read,
            read_at: Some(Instant::now()),
        };
    }

    pub(crate) fn mods_state(&self, instance: &Instance) -> ModsState {
        self.mods_snapshot(instance)
            .map_or(ModsState::NoLoader { can_install: false }, |snapshot| {
                snapshot.state.clone()
            })
    }

    pub(crate) fn echo(&self, instance: &Instance) -> Option<EchoMod> {
        self.mods_snapshot(instance)?.echo.clone()
    }

    pub(crate) fn mega_hack(&self, instance: &Instance) -> Option<MegaHackLoader> {
        self.mods_snapshot(instance)?.mega_hack.clone()
    }

    pub(crate) fn extensions_enabled(&self, instance: &Instance) -> bool {
        self.mods_snapshot(instance)
            .is_some_and(|snapshot| snapshot.extensions_on)
    }

    fn mods_snapshot(&self, instance: &Instance) -> Option<&ModsSnapshot> {
        self.mods_cache.read.get(&instance.game_dir())
    }
}
