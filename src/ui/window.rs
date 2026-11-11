use crate::instances::Instance;
use crate::instances::InstanceName;
use weft::core::Modifiers;
use weft::prelude::{Grow, NamedKey, Ui};
use weft::ui::Shortcut;

use crate::app::{Action, App};
use crate::ui::dialogs::Dialog;
use crate::ui::play_button::Click;

mod details;
mod instance_table;
mod log_tab;
mod status_bar;
mod title_bar;
mod toolbar;

use crate::ui::mod_picker::ModPicker;

pub(crate) struct Window {
    pub(crate) app: App,
    icon: weft::gfx::Image,
    mod_picker: ModPicker,
    pub(crate) chosen: Option<InstanceName>,
    show_log: bool,
    log_seen: usize,
    filter: String,
    show_steam_account: bool,
    update_offered: bool,
    steam_answer: String,
    pub(crate) dialog: Option<Dialog>,
    pub(crate) actions: Vec<Action>,
}

impl Window {
    pub(crate) fn new(app: App) -> Self {
        let dialog = app
            .offers_move_to_applications()
            .then_some(Dialog::MoveToApplications);
        Self {
            icon: weft::gfx::Image::from_rgba8(
                crate::ICON_SIDE,
                crate::ICON_SIDE,
                crate::ICON_RGBA,
            ),
            chosen: app.default_instance().cloned(),
            app,
            mod_picker: ModPicker::default(),
            show_log: false,
            log_seen: 0,
            filter: String::new(),
            show_steam_account: false,
            update_offered: false,
            steam_answer: String::new(),
            dialog,
            actions: Vec::new(),
        }
    }

    pub(crate) fn build(&mut self, ui: &mut Ui) {
        self.app.poll();
        if self.chosen_instance().is_none() {
            let first = self
                .instances()
                .next()
                .map(|instance| instance.name().clone());
            self.chosen = first;
        }
        self.actions.clear();
        self.mod_picker.poll(&mut self.actions);
        self.handle_shortcuts(ui);
        if !self.update_offered
            && self.dialog.is_none()
            && let Some(version) = self.app.update_available()
        {
            self.update_offered = true;
            self.dialog = Some(Dialog::Update(version));
        }

        ui.column()
            .gap(0)
            .width(Grow(1))
            .height(Grow(1))
            .show(|ui| {
                title_bar::show(ui, self, crate::SYSTEM_BUTTONS);
                toolbar::show(ui, self);
                ui.separator();
                if self.show_log {
                    self.log_seen = log_tab::show(ui, &self.app, self.log_seen);
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
        crate::ui::dialogs::show(ui, self);
        crate::ui::dialogs::steam_sign_in::qr_dialog(ui, &self.app, &mut self.actions);
        crate::ui::dialogs::steam_sign_in::prompt_dialog(
            ui,
            &self.app,
            &mut self.steam_answer,
            &mut self.actions,
        );
        self.apply(ui);
        if self.app.should_quit() {
            ui.quit();
        }
    }

    fn instances(&self) -> impl Iterator<Item = &Instance> {
        self.app
            .instances()
            .iter()
            .filter_map(|entry| entry.as_ref().ok())
    }

    fn chosen_instance(&self) -> Option<&Instance> {
        let chosen = self.chosen.as_ref()?;
        self.instances().find(|instance| instance.name() == chosen)
    }

    fn is_downloading(&self, instance: &Instance) -> bool {
        self.app
            .download()
            .is_some_and(|download| download.instance() == instance.name())
    }

    fn can_change(&self, instance: &Instance) -> bool {
        !self.is_downloading(instance) && self.app.playing() != Some(instance.name())
    }

    fn click(&mut self, click: Click) {
        match click {
            Click::Run(action) => self.actions.push(action),
            Click::SetUpWine(name) => self.dialog = Some(Dialog::WineSetup(name)),
            Click::SignIn(name) => {
                self.dialog = Some(Dialog::SteamSignIn(
                    crate::ui::dialogs::steam_sign_in::SignInForm::new(name),
                ));
            }
        }
    }

    fn handle_shortcuts(&mut self, ui: &mut Ui) {
        if self.dialog.is_some() {
            return;
        }
        if ui.shortcut(Shortcut::command('f')) {
            ui.request_focus(filter_id());
        }
        if ui.shortcut(Shortcut::command('n')) {
            self.dialog = Some(Dialog::new_instance(&self.app));
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
        // first pass only
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
            self.app.apply(action);
        }
        let notices = self.app.take_notices();
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
