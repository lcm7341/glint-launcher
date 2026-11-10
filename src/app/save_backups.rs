use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::{App, Notice, SaveMode};
use crate::gd::{GdPlatform, GdVersion};
use crate::instances::{
    Instance, InstanceName, SaveBackup, back_up_save, list_backups, read_backup, restore_files,
};

impl App {
    fn backups_dir(&self, instance: &Instance) -> Option<PathBuf> {
        let data_dir = self.library.data_dir()?;
        Some(if instance.is_linked() {
            data_dir.main_save_backups_dir()
        } else {
            data_dir.own_save_backups_dir(instance.name())
        })
    }

    pub(crate) fn save_backups(&self, name: &InstanceName) -> Vec<SaveBackup> {
        let Some(Ok(instance)) = self.find(name) else {
            return Vec::new();
        };
        self.backups_dir(instance)
            .and_then(|backups| list_backups(&backups).ok())
            .unwrap_or_default()
    }

    pub(super) fn restore_backup(&mut self, name: &InstanceName, backup: &Path) -> Option<Notice> {
        let Some(Ok(instance)) = self.find(name) else {
            return Some(Notice::failed(format!(
                "There is no instance called \"{name}\""
            )));
        };
        let instance = instance.clone();
        if self.playing().is_some() || self.is_game_open(&instance) {
            return Some(Notice::failed("Close Geometry Dash first"));
        }
        let backups = self.backups_dir(&instance)?;
        if backup.parent() != Some(backups.as_path()) {
            return Some(Notice::failed("That backup isn't this instance's"));
        }
        // read before backing up, pruning can delete it
        let files = match read_backup(backup) {
            Ok(files) => files,
            Err(error) => {
                return Some(Notice::failed(format!("Couldn't read the backup: {error}")));
            }
        };
        let save = self.gd_save_dir(&instance);
        if let Err(error) = back_up_save(&save, &backups, &instance.config().version) {
            return Some(Notice::failed(format!(
                "Didn't restore, couldn't back up the current save first: {error}"
            )));
        }
        Some(match restore_files(&files, &save) {
            Ok(()) => {
                self.push_log(format!(
                    "Restored {} into {}",
                    backup.display(),
                    save.display()
                ));
                Notice::done("Restored the save. Glint backed up the one it replaced")
            }
            Err(error) => Notice::failed(format!("Couldn't restore the save: {error}")),
        })
    }

    pub(super) fn back_up_save_for(&mut self, instance: &Instance) -> Result<(), Notice> {
        let Some(backups) = self.backups_dir(instance) else {
            return Ok(());
        };
        let save = self.gd_save_dir(instance);
        match back_up_save(&save, &backups, &instance.config().version) {
            Ok(Some(folder)) => {
                self.push_log(format!("Backed up the save to {}", folder.display()));
                Ok(())
            }
            Ok(None) => Ok(()),
            Err(error) => Err(Notice::failed(format!(
                "Didn't start, couldn't back up the save in {}: {error}",
                save.display()
            ))),
        }
    }

    pub(crate) fn save_mode(&self, instance: &Instance) -> SaveMode {
        let config = instance.config();
        if instance.is_linked()
            && self
                .link_blocker(config.platform, &config.version)
                .is_none()
        {
            SaveMode::Linked
        } else {
            SaveMode::Own
        }
    }

    pub(crate) fn link_blocker(&self, platform: GdPlatform, version: &GdVersion) -> Option<String> {
        if !self.shares_platform_save(platform) {
            return Some("Windows builds on a Mac keep their own save".to_owned());
        }
        let newest = self.main_save_version()?;
        (newest > *version).then(|| format!("Your main save is from {newest}"))
    }

    // wine save differs from mac
    fn shares_platform_save(&self, platform: GdPlatform) -> bool {
        self.native_platform() != Some(GdPlatform::MacOs) || platform != GdPlatform::Windows
    }

    pub(crate) fn can_copy_main_save(&self, instance: &Instance) -> bool {
        self.save_mode(instance) == SaveMode::Own
            && self.shares_platform_save(instance.config().platform)
    }

    pub(super) fn copy_main_save(&mut self, name: &InstanceName) -> Option<Notice> {
        let Some(Ok(instance)) = self.find(name) else {
            return Some(Notice::failed(format!(
                "There is no instance called \"{name}\""
            )));
        };
        let instance = instance.clone();
        if !self.can_copy_main_save(&instance) {
            return Some(Notice::failed("This instance can't copy your main save"));
        }
        if self.playing().is_some() || self.is_game_open(&instance) {
            return Some(Notice::failed("Close Geometry Dash first"));
        }
        let Some(main) = self.play.main_save_dir(&instance) else {
            return Some(Notice::failed("Couldn't find your main save"));
        };
        let files = match read_backup(&main) {
            Ok(files) => files,
            Err(error) => {
                return Some(Notice::failed(format!(
                    "Couldn't read your main save: {error}"
                )));
            }
        };
        let own = self.gd_save_dir(&instance);
        let backups = self.backups_dir(&instance)?;
        if let Err(error) = back_up_save(&own, &backups, &instance.config().version) {
            return Some(Notice::failed(format!(
                "Didn't copy, couldn't back up the per-instance save first: {error}"
            )));
        }
        Some(match restore_files(&files, &own) {
            Ok(()) => Notice::done(format!(
                "Copied your main save into \"{name}\". Glint backed up the per-instance save"
            )),
            Err(error) => Notice::failed(format!("Couldn't copy your main save: {error}")),
        })
    }

    pub(super) fn note_main_save_version(&self, instance: &Instance) {
        let Some(data_dir) = self.library.data_dir() else {
            return;
        };
        let version = &instance.config().version;
        if !instance.is_linked() || self.recorded_main_save_version() >= Some(version.clone()) {
            return;
        }
        let _ = fs::create_dir_all(data_dir.root());
        let _ = fs::write(data_dir.main_save_version_path(), format!("{version}\n"));
    }

    // imported linked installs count
    fn main_save_version(&self) -> Option<GdVersion> {
        let imported = self
            .instances()
            .iter()
            .filter_map(|entry| entry.as_ref().ok())
            .filter(|instance| instance.is_imported() && instance.is_linked())
            .filter(|instance| self.shares_platform_save(instance.config().platform))
            .map(|instance| instance.config().version.clone());
        imported.chain(self.recorded_main_save_version()).max()
    }

    fn recorded_main_save_version(&self) -> Option<GdVersion> {
        let path = self.library.data_dir()?.main_save_version_path();
        fs::read_to_string(path).ok()?.trim().parse().ok()
    }
}

pub(crate) fn ago(taken: SystemTime) -> String {
    let seconds = SystemTime::now()
        .duration_since(taken)
        .map_or(0, |since| since.as_secs());
    let (count, unit) = match seconds {
        0..60 => return "Just now".to_owned(),
        60..3_600 => (seconds / 60, "minute"),
        3_600..86_400 => (seconds / 3_600, "hour"),
        _ => (seconds / 86_400, "day"),
    };
    let plural = if count == 1 { "" } else { "s" };
    format!("{count} {unit}{plural} ago")
}
