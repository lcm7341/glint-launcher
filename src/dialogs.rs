use std::path::PathBuf;

use crate::gd::{GdPlatform, GdVersion, InstanceName};
use crate::instances::Instance;
use weft::prelude::{Fixed, Grow, Justify, Ui};

use crate::add_existing::AddExisting;
use crate::form::{DIALOG_WIDTH, empty_label, field_label, show_field_hint, show_form};
use crate::geode_index::GeodeIndex;
use crate::launcher::{Action, Launcher};
use crate::sentence::home_relative;
use crate::window::Window;

pub(crate) enum Dialog {
    NewInstance {
        name: String,
        platform: GdPlatform,
        release: Option<GdVersion>,
    },
    Rename {
        from: InstanceName,
        name: String,
    },
    Delete {
        name: InstanceName,
        linked: Option<PathBuf>,
    },
    AddExisting(AddExisting),
    GeodeIndex(GeodeIndex),
}

impl Dialog {
    pub(crate) fn new_instance(launcher: &Launcher) -> Self {
        let platform = default_platform(launcher);
        Dialog::NewInstance {
            name: String::new(),
            platform,
            release: newest(launcher, platform),
        }
    }

    pub(crate) fn rename(name: &InstanceName) -> Self {
        Dialog::Rename {
            from: name.clone(),
            name: name.to_string(),
        }
    }

    pub(crate) fn delete(instance: &Instance) -> Self {
        Dialog::Delete {
            name: instance.name().clone(),
            linked: instance.config().game_folder.clone(),
        }
    }
}

pub(crate) fn default_platform(launcher: &Launcher) -> GdPlatform {
    launcher
        .native_platform()
        .into_iter()
        .chain(GdPlatform::ALL)
        .find(|platform| launcher.catalog().newest(*platform).is_some())
        .unwrap_or(GdPlatform::Windows)
}

pub(crate) fn show(ui: &mut Ui, window: &mut Window) {
    let Some(mut dialog) = window.dialog.take() else {
        return;
    };
    let mut open = true;
    match &mut dialog {
        Dialog::NewInstance {
            name,
            platform,
            release,
        } => show_new_instance(ui, window, &mut open, name, platform, release),
        Dialog::Rename { from, name } => show_rename(ui, window, &mut open, from, name),
        Dialog::Delete { name, linked } => show_delete(ui, window, &mut open, name, linked),
        Dialog::AddExisting(add) => crate::add_existing::show(ui, window, &mut open, add),
        Dialog::GeodeIndex(index) => {
            index.show(ui, &mut open, &window.launcher, &mut window.actions);
        }
    }
    if open && window.dialog.is_none() {
        window.dialog = Some(dialog);
    }
}

fn show_new_instance(
    ui: &mut Ui,
    window: &mut Window,
    open: &mut bool,
    name: &mut String,
    platform: &mut GdPlatform,
    release: &mut Option<GdVersion>,
) {
    let launcher = &window.launcher;
    let mut create = None;
    ui.modal(open)
        .title("New instance")
        .style(|style| style.width(Fixed(DIALOG_WIDTH)))
        .show(|ui| {
            let placeholder = release
                .as_ref()
                .and_then(|version| launcher.default_name(version))
                .map_or_else(|| "Name".to_owned(), |name| name.to_string());
            let typed = name.trim();
            let typed_name = if typed.is_empty() {
                Ok(None)
            } else {
                typed.parse::<InstanceName>().map(Some)
            };
            let mut submitted = false;
            show_form(ui, |ui| {
                field_label(ui, "Name");
                submitted = ui
                    .text_input(name)
                    .name("Name")
                    .placeholder(placeholder)
                    .width(Grow(1))
                    .submitted();
                empty_label(ui);
                show_field_hint(ui, typed_name.as_ref().err());

                field_label(ui, "Platform");
                let before = *platform;
                ui.segmented(platform, GdPlatform::ALL)
                    .labels(GdPlatform::ALL.map(GdPlatform::display_name))
                    .name("Platform")
                    .show();
                if *platform != before {
                    *release = newest(launcher, *platform);
                }

                field_label(ui, "Release");
                let versions: Vec<GdVersion> = launcher
                    .catalog()
                    .for_platform(*platform)
                    .map(|entry| entry.version.clone())
                    .collect();
                match release {
                    Some(version) => {
                        ui.dropdown(version, versions.iter().cloned())
                            .labels(versions.iter().map(GdVersion::to_string))
                            .name("Release")
                            .searchable()
                            .search_placeholder("Find a version")
                            .show();
                    }
                    None => {
                        ui.label("None in the catalog yet").muted();
                    }
                }
            });

            let entry = release
                .as_ref()
                .and_then(|version| launcher.catalog().find(version, *platform));
            ui.row().justify(Justify::End).gap(8).show(|ui| {
                if ui.button("Cancel").clicked() {
                    ui.close_overlay();
                }
                let can_create = entry.is_some() && typed_name.is_ok();
                let clicked = ui.button("Create").primary().enabled(can_create).clicked();
                if let (Some(entry), Ok(typed)) = (entry, typed_name)
                    && (clicked || submitted)
                {
                    create = Some((typed, entry.clone()));
                    ui.close_overlay();
                }
            });
        });
    if let Some((name, entry)) = create {
        window.chosen = name
            .clone()
            .or_else(|| window.launcher.default_name(&entry.version));
        window.actions.push(Action::Create { name, entry });
    }
}

fn show_rename(
    ui: &mut Ui,
    window: &mut Window,
    open: &mut bool,
    from: &InstanceName,
    name: &mut String,
) {
    ui.modal(open)
        .title(format!("Rename \"{from}\""))
        .style(|style| style.width(Fixed(DIALOG_WIDTH)))
        .show(|ui| {
            let mut submitted = false;
            let mut parsed = None;
            show_form(ui, |ui| {
                field_label(ui, "New name");
                submitted = ui
                    .text_input(name)
                    .name("New name")
                    .select_all_on_focus(true)
                    .width(Grow(1))
                    .submitted();
                let typed = name.trim();
                parsed = (!typed.is_empty()).then(|| typed.parse::<InstanceName>());
                empty_label(ui);
                show_field_hint(ui, parsed.as_ref().and_then(|parsed| parsed.as_ref().err()));
            });
            let to = parsed.and_then(Result::ok).filter(|to| to != from);
            ui.row().justify(Justify::End).gap(8).show(|ui| {
                if ui.button("Cancel").clicked() {
                    ui.close_overlay();
                }
                let clicked = ui
                    .button("Rename instance")
                    .primary()
                    .enabled(to.is_some())
                    .clicked();
                if let Some(to) = to
                    && (clicked || submitted)
                {
                    window.actions.push(Action::Rename {
                        from: from.clone(),
                        to,
                    });
                    ui.close_overlay();
                }
            });
        });
}

fn show_delete(
    ui: &mut Ui,
    window: &mut Window,
    open: &mut bool,
    name: &InstanceName,
    linked: &Option<PathBuf>,
) {
    ui.modal(open)
        .title(format!("Delete \"{name}\"?"))
        .show(|ui| {
            match linked {
                Some(install) => ui.label(format!("Files in {} are kept.", home_relative(install))),
                None => ui.label("Deletes the game files, mods and save."),
            };
            ui.row().justify(Justify::End).gap(8).show(|ui| {
                if ui.button("Cancel").clicked() {
                    ui.close_overlay();
                }
                if ui.button("Delete instance").danger().clicked() {
                    window.actions.push(Action::Delete(name.clone()));
                    ui.close_overlay();
                }
            });
        });
}

fn newest(launcher: &Launcher, platform: GdPlatform) -> Option<GdVersion> {
    launcher
        .catalog()
        .newest(platform)
        .map(|entry| entry.version.clone())
}
