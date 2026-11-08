use crate::gd::{GdPlatform, InstanceName};
use crate::instances::{GameFiles, Instance};
use crate::mods::{fetch_latest_geode, has_geode, install_geode, remove_geode};

use super::{Launcher, Notice};
use crate::job::Job;

pub(super) const FIRST_GEODE_VERSION: &str = "2.2";

pub(super) struct GeodeTask {
    instance: InstanceName,
    job: Job<Result<String, String>>,
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
            .is_some_and(|task| &task.instance == instance.name())
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
        self.geode_task = None;
        self.notices.push(match outcome {
            Ok(version) => Notice::done(format!("Installed Geode {version} into \"{name}\"")),
            Err(reason) => Notice::failed(format!("Couldn't install Geode: {reason}")),
        });
    }
}
