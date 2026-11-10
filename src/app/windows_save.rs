use std::path::PathBuf;

use crate::gd::GdPlatform;
use crate::instances::{Instance, SaveSwap};
use crate::platform::native_game_running;

use super::{Action, App, Notice};

impl App {
    pub(crate) fn finish_save_swap(&mut self) {
        if self.native_platform() != Some(GdPlatform::Windows) {
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

    pub(super) fn swap_save_in(&self, instance: &Instance) -> Result<bool, String> {
        let swap = self.save_swap();
        let is_needed =
            !instance.is_linked() || swap.as_ref().is_some_and(SaveSwap::has_unfinished);
        if !is_needed {
            return Ok(false);
        }
        let Some(swap) = swap else {
            return Err("LOCALAPPDATA isn't set".to_owned());
        };
        match native_game_running() {
            Ok(false) => {}
            Ok(true) => {
                return Err("GD is already open".to_owned());
            }
            Err(error) => {
                return Err(format!("Couldn't check if GD is open: {error}"));
            }
        }
        if instance.is_linked() {
            swap.restore()
                .map_err(|error| format!("Couldn't put your usual save back: {error}"))?;
            return Ok(false);
        }
        swap.swap_in(instance.name(), &instance.save_dir())
            .map_err(|error| format!("Couldn't swap the save in: {error}"))?;
        Ok(true)
    }

    pub(super) fn refuses_while_playing(&mut self, action: &Action) -> bool {
        let (Action::Rename { from: name, .. } | Action::Delete(name)) = action else {
            return false;
        };
        if self.playing() != Some(name) {
            return false;
        }
        self.activity
            .notice(Notice::failed(format!("\"{name}\" is open")));
        true
    }

    fn save_swap(&self) -> Option<SaveSwap> {
        let data_dir = self.library.data_dir()?;
        Some(SaveSwap::new(data_dir, usual_save_dir()?))
    }
}

pub(super) fn usual_save_dir() -> Option<PathBuf> {
    crate::platform::native_save_dir(None)
}
