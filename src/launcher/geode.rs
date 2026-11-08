use crate::gd::{GdPlatform, InstanceName};
use crate::instances::{GameFiles, Instance};
use crate::mods::{fetch_latest_geode, has_geode, install_geode, install_geode_mod, remove_geode};

use super::{Launcher, Notice};
use crate::job::Job;

pub(super) const FIRST_GEODE_VERSION: &str = "2.2";

pub(super) struct GeodeTask {
    instance: InstanceName,
    index_mod: Option<IndexInstall>,
    job: Job<Result<String, String>>,
}

struct IndexInstall {
    id: String,
    name: String,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum ModLoader {
    Geode,
    InstallingGeode,
    GeodeAvailable,
    None,
}

impl Launcher {
    pub(crate) fn mod_loader(&self, instance: &Instance) -> ModLoader {
        let game_dir = instance.game_dir();
        if self
            .geode_task
            .as_ref()
            .is_some_and(|task| &task.instance == instance.name() && task.index_mod.is_none())
        {
            return ModLoader::InstallingGeode;
        }
        if has_geode(&game_dir) {
            return ModLoader::Geode;
        }
        let first = FIRST_GEODE_VERSION.parse().expect("a fixed version parses");
        let can_install = !instance.is_linked()
            && instance.config().platform == GdPlatform::Windows
            && instance.game_files() == GameFiles::Present
            && instance.config().version >= first;
        if can_install {
            ModLoader::GeodeAvailable
        } else {
            ModLoader::None
        }
    }

    pub(super) fn start_geode_install(&mut self, name: &InstanceName) -> Option<Notice> {
        if self.geode_task.is_some() {
            return Some(Notice::failed("Already installing something"));
        }
        let Some(Ok(instance)) = self.find(name) else {
            return None;
        };
        let version = instance.config().version.clone();
        let game_dir = instance.game_dir();
        let job = Job::spawn(&self.notify, move || match fetch_latest_geode(&version) {
            Ok(Some(release)) => install_geode(&game_dir, &release)
                .map(|()| match release.prerelease {
                    true => format!("{} (prerelease)", release.version),
                    false => release.version,
                })
                .map_err(|error| error.to_string()),
            Ok(None) => Err(format!("there's no Geode for {version} yet")),
            Err(error) => Err(error.to_string()),
        });
        self.push_log(format!("Installing Geode into \"{name}\""));
        self.geode_task = Some(GeodeTask {
            instance: name.clone(),
            index_mod: None,
            job,
        });
        None
    }

    pub(crate) fn installing_geode(&self) -> bool {
        self.geode_task.is_some()
    }

    pub(crate) fn installing_mod(&self, name: &InstanceName) -> Option<&str> {
        let task = self.geode_task.as_ref()?;
        let index_mod = task.index_mod.as_ref()?;
        (&task.instance == name).then_some(index_mod.id.as_str())
    }

    pub(super) fn start_index_install(
        &mut self,
        name: &InstanceName,
        id: String,
        mod_name: String,
    ) -> Option<Notice> {
        if self.geode_task.is_some() {
            return Some(Notice::failed("Already installing something"));
        }
        let Some(Ok(instance)) = self.find(name) else {
            return Some(Notice::failed(format!("There's no instance \"{name}\"")));
        };
        let game_dir = instance.game_dir();
        if !has_geode(&game_dir) {
            return Some(Notice::failed("Install Geode first"));
        }
        if self.may_be_game_open(instance) {
            return Some(Notice::failed("Close the game first"));
        }
        let version = instance.config().version.clone();
        let platform = instance.config().platform;
        let wanted = id.clone();
        let job = Job::spawn(&self.notify, move || {
            install_geode_mod(&game_dir, &wanted, &version, platform)
                .map(|names| match names.split_first() {
                    Some((first, [])) => first.clone(),
                    Some((first, needed)) => format!("{first}, with {}", needed.join(", ")),
                    None => wanted,
                })
                .map_err(|error| error.to_string())
        });
        self.push_log(format!("Installing {mod_name} ({id}) into \"{name}\""));
        self.geode_task = Some(GeodeTask {
            instance: name.clone(),
            index_mod: Some(IndexInstall { id, name: mod_name }),
            job,
        });
        None
    }

    pub(super) fn remove_geode_from(&mut self, name: &InstanceName) -> Option<Notice> {
        let Some(Ok(instance)) = self.find(name) else {
            return None;
        };
        if instance.is_linked() {
            return Some(Notice::failed("Linked installs are left alone"));
        }
        Some(match remove_geode(&instance.game_dir()) {
            Ok(()) => Notice::done(format!("Removed Geode from \"{name}\"")),
            Err(error) => Notice::failed(format!("Couldn't remove Geode: {error}")),
        })
    }

    pub(super) fn poll_geode(&mut self) {
        let Some(task) = &self.geode_task else {
            return;
        };
        let Some(outcome) = task.job.take() else {
            return;
        };
        let name = task.instance.clone();
        let index_mod = task
            .index_mod
            .as_ref()
            .map(|index_mod| index_mod.name.clone());
        self.geode_task = None;
        self.notices.push(match (outcome, index_mod) {
            (Ok(version), None) => {
                Notice::done(format!("Installed Geode {version} into \"{name}\""))
            }
            (Err(reason), None) => Notice::failed(format!("Couldn't install Geode: {reason}")),
            (Ok(installed), Some(_)) => {
                Notice::done(format!("Installed {installed} into \"{name}\""))
            }
            (Err(reason), Some(mod_name)) => {
                Notice::failed(format!("Couldn't install {mod_name}: {reason}"))
            }
        });
    }
}
