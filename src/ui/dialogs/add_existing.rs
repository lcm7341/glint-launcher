use std::path::PathBuf;

use crate::instances::recognize_install;
use weft::prelude::{Align, Fixed, Grow, Ui};

use crate::app::{Action, App};
use crate::format::{capitalized, home_relative};
use crate::platform::FilePicker;
use crate::ui::form::{field_label, show_form};
use crate::ui::window::Window;

const WIDTH: f32 = 560.0;

#[derive(Default)]
pub(crate) struct AddExisting {
    folder: String,
    picker: FilePicker,
    picker_error: Option<String>,
}

pub(crate) fn show(ui: &mut Ui, window: &mut Window, open: &mut bool, add: &mut AddExisting) {
    if let Some(result) = add.picker.take_result() {
        match result {
            Ok(Some(picked)) => {
                add.folder = home_relative(&picked);
                add.picker_error = None;
            }
            Ok(None) => add.picker_error = None,
            Err(error) => add.picker_error = Some(format!("Couldn't select a folder: {error}")),
        }
    }
    let launcher = &window.app;
    let actions = &mut window.actions;
    ui.modal(open)
        .title("Add existing install")
        .style(|style| style.width(Fixed(WIDTH)))
        .show(|ui| {
            if let Some(error) = &add.picker_error {
                ui.label(error).muted();
            }
            show_found(ui, launcher, actions);
            ui.separator();
            show_other_folder(ui, &mut add.folder, &mut add.picker, launcher, actions);
        });
}

fn show_found(ui: &mut Ui, launcher: &App, actions: &mut Vec<Action>) {
    let found = launcher.found_installs();
    if found.is_empty() {
        ui.label("No other installs found.").muted();
        return;
    }
    ui.label("Found").muted().font_size(12);
    for (index, install) in found.iter().enumerate() {
        ui.push_id(index, |ui| {
            ui.row().gap(12).align(Align::Center).show(|ui| {
                ui.column().gap(2).width(Grow(1)).show(|ui| {
                    ui.label(format!("Geometry Dash {}", install.config.version));
                    ui.label(home_relative(&install.folder))
                        .muted()
                        .font_size(12)
                        .wrap(false)
                        .style(|style| style.clip(true));
                });
                if ui.button("Add").clicked() {
                    actions.push(Action::AddExisting(install.folder.clone()));
                }
            });
        });
    }
}

fn show_other_folder(
    ui: &mut Ui,
    folder: &mut String,
    picker: &mut FilePicker,
    launcher: &App,
    actions: &mut Vec<Action>,
) {
    let mut submitted = false;
    let typed = expand_home(folder.trim());
    show_form(ui, |ui| {
        field_label(ui, "Other folder");
        ui.row().gap(8).align(Align::Center).show(|ui| {
            submitted = ui
                .text_input(folder)
                .name("Other folder")
                .placeholder("~/Games/GD-2.113")
                .width(Grow(1))
                .submitted();
            if FilePicker::is_available() {
                let browse = ui.button("Browse…").enabled(!picker.is_open()).show();
                if browse.clicked() {
                    let start = typed
                        .clone()
                        .filter(|path| path.is_dir())
                        .or_else(crate::platform::home_dir)
                        .unwrap_or_default();
                    picker.open(&start, launcher.notify());
                }
            }
        });
    });
    let recognized = typed.as_ref().map(|path| recognize_install(path));
    let hint = match &recognized {
        None => "GD folder".to_owned(),
        Some(Ok(config)) => format!("Geometry Dash {}", config.version),
        Some(Err(error)) => capitalized(&error.to_string()),
    };
    ui.row().gap(8).align(Align::Center).show(|ui| {
        ui.label(hint)
            .muted()
            .font_size(12)
            .style(|style| style.width(Grow(1)));
        let can_add = matches!(recognized, Some(Ok(_)));
        let clicked = ui.button("Add folder").enabled(can_add).clicked();
        if let (true, Some(path)) = (can_add && (clicked || submitted), typed) {
            actions.push(Action::AddExisting(path));
            folder.clear();
        }
        if ui.button("Done").clicked() {
            ui.close_overlay();
        }
    });
}

fn expand_home(text: &str) -> Option<PathBuf> {
    if text.is_empty() {
        return None;
    }
    match (text.strip_prefix('~'), crate::platform::home_dir()) {
        (Some(rest), Some(home)) => Some(home.join(rest.trim_start_matches('/'))),
        _ => Some(PathBuf::from(text)),
    }
}
