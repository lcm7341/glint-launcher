use crate::instances::Instance;
use crate::mods::{DllLoader, DllMod, EchoMod, GeodeMod, MegaHackLoader};
use crate::task::Notify;
use std::path::PathBuf;
use std::rc::Rc;

mod cache;
mod coordinator;
mod install;
use cache::ModsCache;
use install::GeodeTask;
pub(crate) use install::ModLoader;

#[derive(Default)]
pub(super) struct ModManager {
    cache: ModsCache,
    installing: Option<GeodeTask>,
}

pub(super) struct ModContext<'a> {
    game_open: bool,
    notify: &'a Notify,
}

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
    Add { source: PathBuf },
    Remove { file_name: String },
    OpenFolder,
    SetMegaHackEnabled { enabled: bool },
    SetExtensionsEnabled { enabled: bool },
    SetGeodeEnabled { id: String, enabled: bool },
    SetGeodePinned { id: String, pinned: bool },
    SetEchoEnabled { file_name: String, enabled: bool },
    InstallFromIndex { id: String, name: String },
}

impl ModsState {
    pub(crate) fn is_geode_grid(&self) -> bool {
        matches!(self, ModsState::Geode { mods: Ok(mods) } if !mods.is_empty())
    }
}

impl ModManager {
    pub(super) fn invalidate(&mut self) {
        self.cache.invalidate();
    }
    pub(super) fn refresh<'a>(
        &mut self,
        instances: impl IntoIterator<Item = (&'a Instance, PathBuf)>,
    ) {
        if self.cache.is_fresh() {
            return;
        }
        let inputs: Vec<_> = instances
            .into_iter()
            .map(|(instance, save)| (instance, save, self.mod_loader(instance)))
            .collect();
        self.cache.refresh(inputs);
    }
    pub(super) fn state(&self, instance: &Instance) -> ModsState {
        self.cache.mods_state(instance)
    }
    pub(super) fn echo(&self, instance: &Instance) -> Option<EchoMod> {
        self.cache.echo(instance)
    }
    pub(super) fn mega_hack(&self, instance: &Instance) -> Option<MegaHackLoader> {
        self.cache.mega_hack(instance)
    }
    pub(super) fn extensions_enabled(&self, instance: &Instance) -> bool {
        self.cache.extensions_enabled(instance)
    }
}

