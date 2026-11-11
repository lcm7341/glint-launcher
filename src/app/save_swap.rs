use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::gd::GdPlatform;
use crate::instances::{Instance, SaveSwap};
use crate::platform::native_game_running;

use super::{Action, App, Notice, SaveMode};

impl App {
    pub(crate) fn finish_save_swap(&mut self) {
        if self.native_platform().is_none() {
            return;
        }
        let Some(swap) = self.save_swap() else {
            return;
        };
        if !swap.has_unfinished() {
            return;
        }
        match native_game_running() {
            Ok(false) => {}
            Ok(true) => {
                self.activity.notice(Notice::failed(
                    "GD is open with an instance's save swapped in. Close it and restart Glint",
                ));
                return;
            }
            Err(error) => {
                self.activity.notice(Notice::failed(format!(
                    "Couldn't check if GD is open, restart Glint: {error}"
                )));
                return;
            }
        }
        match swap.restore() {
            Ok(Some(name)) => self.push_log(format!("Swapped the save of \"{name}\" back")),
            Ok(None) => {}
            Err(error) => self.activity.notice(Notice::failed(format!(
                "Couldn't put your usual save back: {error}"
            ))),
        }
    }

    pub(super) fn swap_save_in(&self, instance: &Instance, mode: SaveMode) -> Result<bool, String> {
        let swap = self.save_swap();
        let is_needed =
            mode != SaveMode::Linked || swap.as_ref().is_some_and(SaveSwap::has_unfinished);
        if !is_needed {
            return Ok(false);
        }
        let Some(swap) = swap else {
            return Err("Couldn't find your main save folder".to_owned());
        };
        match native_game_running() {
            Ok(false) => {}
            Ok(true) => {
                return Err("GD is open".to_owned());
            }
            Err(error) => {
                return Err(format!("Couldn't check if GD is open: {error}"));
            }
        }
        swap.restore()
            .map_err(|error| format!("Couldn't put your usual save back: {error}"))?;
        if mode == SaveMode::Linked {
            return Ok(false);
        }
        swap.swap_in(instance.name(), &instance.save_dir())
            .map_err(|error| format!("Couldn't swap the save in: {error}"))?;
        Ok(true)
    }

    pub(super) fn refuses_while_playing(&mut self, action: &Action) -> bool {
        let (Action::Rename { from: name, .. }
        | Action::Delete(name)
        | Action::SetLinked { instance: name, .. }) = action
        else {
            return false;
        };
        if self.playing() != Some(name) {
            return false;
        }
        self.activity
            .notice(Notice::failed(format!("\"{name}\" is open")));
        true
    }

    /// saves from the old CFFIXED_USER_HOME launch
    pub(crate) fn move_old_mac_saves(&mut self) {
        if self.native_platform() != Some(GdPlatform::MacOs) {
            return;
        }
        let instances: Vec<Instance> = self
            .instances()
            .iter()
            .filter_map(|entry| entry.as_ref().ok().cloned())
            .collect();
        for instance in instances {
            let Some(old) = crate::platform::native_save_dir(Some(&instance.mac_home_dir())) else {
                continue;
            };
            let name = instance.name();
            match move_old_save(&old, &instance.save_dir()) {
                Ok(OldSave::None) => {}
                Ok(OldSave::Moved) => {
                    self.push_log(format!("Moved \"{name}\"'s save into its save folder"))
                }
                Ok(OldSave::Both) => self.activity.notice(Notice::failed(format!(
                    "\"{name}\" has saves in {} and {}, Glint left both",
                    old.display(),
                    instance.save_dir().display()
                ))),
                Err(error) => self.activity.notice(Notice::failed(format!(
                    "Couldn't move \"{name}\"'s save from {}: {error}",
                    old.display()
                ))),
            }
        }
    }

    fn save_swap(&self) -> Option<SaveSwap> {
        let data_dir = self.library.data_dir()?;
        Some(SaveSwap::new(data_dir, usual_save_dir()?))
    }
}

pub(super) fn usual_save_dir() -> Option<PathBuf> {
    crate::platform::native_save_dir(None)
}

enum OldSave {
    None,
    Moved,
    Both,
}

fn move_old_save(old: &Path, save: &Path) -> io::Result<OldSave> {
    if !has_save_files(old) {
        return Ok(OldSave::None);
    }
    if has_save_files(save) {
        return Ok(OldSave::Both);
    }
    fs::create_dir_all(save)?;
    for entry in fs::read_dir(old)? {
        let entry = entry?;
        let target = save.join(entry.file_name());
        if !target.exists() {
            fs::rename(entry.path(), target)?;
        }
    }
    let _ = fs::remove_dir(old);
    Ok(OldSave::Moved)
}

fn has_save_files(folder: &Path) -> bool {
    fs::read_dir(folder).is_ok_and(|entries| {
        entries.filter_map(Result::ok).any(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.starts_with("CC") && name.ends_with(".dat")
        })
    })
}
