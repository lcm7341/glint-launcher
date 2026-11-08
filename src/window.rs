use crate::gd::InstanceName;
use crate::instances::Instance;
use weft::core::Modifiers;
use weft::prelude::{Grow, NamedKey, Ui};
use weft::ui::Shortcut;

use crate::dialogs::Dialog;
use crate::launcher::{Action, Launcher};

mod details;
mod instance_table;
mod log_tab;
mod status_bar;
mod title_bar;
mod toolbar;

pub(crate) struct Window {
    pub(crate) launcher: Launcher,
    icon: weft::gfx::Image,
    pub(crate) chosen: Option<InstanceName>,
    show_log: bool,
    log_seen: usize,
    filter: String,
    pub(crate) dialog: Option<Dialog>,
    pub(crate) actions: Vec<Action>,
}

impl Window {
    pub(crate) fn new(launcher: Launcher) -> Self {
        Self {
            icon: weft::gfx::Image::from_rgba8(
                crate::ICON_SIDE,
                crate::ICON_SIDE,
                crate::ICON_RGBA,
            ),
            chosen: launcher.default_instance().cloned(),
            launcher,
            show_log: false,
            log_seen: 0,
            filter: String::new(),
            dialog: None,
            actions: Vec::new(),
        }
    }

    pub(crate) fn build(&mut self, ui: &mut Ui) {
        self.launcher.poll();
        if self.chosen_instance().is_none() {
            let first = self
                .instances()
                .next()
                .map(|instance| instance.name().clone());
            self.chosen = first;
        }
        self.actions.clear();
        self.handle_shortcuts(ui);

        ui.column()
            .gap(0)
            .width(Grow(1))
            .height(Grow(1))
            .show(|ui| {
                title_bar::show(ui, &self.icon);
                toolbar::show(ui, self);
                ui.separator();
                if self.show_log {
                    self.log_seen = log_tab::show(ui, &self.launcher, self.log_seen);
                } else {
                    let mut split = ui
                        .split_remembered()
                        .default_value(0.5)
                        .min_sizes(300.0, 360.0)
                        .name("Resize the table and details")
                        .begin();
                    instance_table::show(split.first(), self);
                    details::show(split.second(), self);
                    split.end();
                }
                ui.separator();
                status_bar::show(ui, self);
            });
        crate::dialogs::show(ui, self);
        crate::qr_sign_in::qr_dialog(ui, &self.launcher, &mut self.actions);
        self.apply(ui);
    }

    fn instances(&self) -> impl Iterator<Item = &Instance> {
        self.launcher
            .instances()
            .iter()
            .filter_map(|entry| entry.as_ref().ok())
    }

    fn chosen_instance(&self) -> Option<&Instance> {
        let chosen = self.chosen.as_ref()?;
        self.instances().find(|instance| instance.name() == chosen)
    }

    fn is_downloading(&self, instance: &Instance) -> bool {
        self.launcher
            .download()
            .is_some_and(|download| download.instance() == instance.name())
    }

    fn can_change(&self, instance: &Instance) -> bool {
        !self.is_downloading(instance) && self.launcher.playing() != Some(instance.name())
    }

    fn handle_shortcuts(&mut self, ui: &mut Ui) {
        if self.dialog.is_some() {
            return;
        }
        if ui.shortcut(Shortcut::command('f')) {
            ui.request_focus(filter_id());
        }
        if ui.shortcut(Shortcut::command('n')) {
            self.dialog = Some(Dialog::new_instance(&self.launcher));
        }
        let Some(chosen) = self.chosen_instance().cloned() else {
            return;
        };
        if !self.can_change(&chosen) {
            return;
        }
        if ui.shortcut(Shortcut::new(Modifiers::NONE, NamedKey::F2)) {
            self.dialog = Some(Dialog::rename(chosen.name()));
        } else if ui.shortcut(Shortcut::new(Modifiers::NONE, NamedKey::Delete)) {
            self.dialog = Some(Dialog::delete(&chosen));
        }
    }

    fn apply(&mut self, ui: &mut Ui) {
        // weft reruns the build when layout moves, only apply on the first pass
        let mut actions = std::mem::take(&mut self.actions);
        if !actions.is_empty() && !ui.once_per_frame(ui.child_id("apply actions")) {
            actions.clear();
        }
        for action in actions {
            if let Action::Rename { from, to } = &action
                && self.chosen.as_ref() == Some(from)
            {
                self.chosen = Some(to.clone());
            }
            self.launcher.apply(action);
        }
        let notices = self.launcher.take_notices();
        if !notices.is_empty() {
            ui.request_redraw();
        }
        for notice in notices {
            let toast = ui.toast(notice.text);
            if notice.is_failure {
                toast.urgent();
            }
        }
    }
}

fn filter_id() -> weft::core::Id {
    weft::core::Id::from_hash(&"glint search instances")
}
