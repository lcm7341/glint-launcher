use crate::app::{Action, ModChange};
use crate::instances::Instance;
use crate::instances::InstanceName;
use crate::platform::FilePicker;
use crate::task::Notify;
use weft::prelude::Ui;

#[derive(Default)]
pub(crate) struct ModPicker {
    picker: FilePicker,
    target: Option<InstanceName>,
}

impl ModPicker {
    pub(crate) fn is_available() -> bool {
        FilePicker::is_available()
    }
    pub(crate) fn is_open(&self) -> bool {
        self.picker.is_open()
    }

    pub(crate) fn open(&mut self, instance: &Instance, notify: Notify) {
        if self.is_open() || !Self::is_available() {
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

    pub(crate) fn poll(&mut self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let Some(result) = self.picker.take_result() else {
            return;
        };
        let Some(instance) = self.target.take() else {
            return;
        };
        match result {
            Ok(Some(source)) => actions.push(Action::Mod {
                instance,
                change: ModChange::Add { source },
            }),
            Ok(None) => {}
            Err(error) => {
                ui.toast(format!("Couldn't select a mod: {error}")).urgent();
            }
        }
    }
}
