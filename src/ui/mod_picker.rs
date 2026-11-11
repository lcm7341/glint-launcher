use crate::app::{Action, ModChange};
use crate::instances::Instance;
use crate::instances::InstanceName;
use crate::platform::FilePicker;
use crate::task::Notify;

#[derive(Default)]
pub(crate) struct ModPicker {
    picker: FilePicker,
    target: Option<InstanceName>,
}

impl ModPicker {
    pub(crate) fn is_open(&self) -> bool {
        self.picker.is_open()
    }

    pub(crate) fn open(&mut self, instance: &Instance, notify: Notify) {
        if self.is_open() {
            return;
        }
        let home = crate::platform::home_dir().unwrap_or_default();
        let downloads = home.join("Downloads");
        let start = if downloads.is_dir() { downloads } else { home };
        if crate::mods::has_geode(&instance.game_dir()) {
            self.picker.open_geode_file(&start, notify);
        } else {
            self.picker.open_dll_file(&start, notify);
        }
        self.target = Some(instance.name().clone());
    }

    pub(crate) fn poll(&mut self, actions: &mut Vec<Action>) {
        let Some(result) = self.picker.take_result() else {
            return;
        };
        let Some(instance) = self.target.take() else {
            return;
        };
        if let Some(source) = result {
            actions.push(Action::Mod {
                instance,
                change: ModChange::Add { source },
            });
        }
    }
}
