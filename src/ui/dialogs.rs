pub(crate) mod add_existing;
pub(crate) mod steam_sign_in;

use std::path::PathBuf;

use crate::gd::{CatalogEntry, GdPlatform, GdVersion};
use crate::instances::Instance;
use crate::instances::InstanceName;
use crate::launch::WINE_VERSION;
use weft::prelude::{Fixed, Grow, Justify, Ui};

use crate::app::{Action, App};
use crate::format::home_relative;
use crate::gd::is_encrypted_mac_build;
use crate::ui::dialogs::add_existing::AddExisting;
use crate::ui::form::{DIALOG_WIDTH, empty_label, field_label, show_field_hint, show_form};
use crate::ui::mods::browse::GeodeIndex;
use crate::ui::window::Window;

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
    WineSetup(InstanceName),
    AddExisting(AddExisting),
    GeodeIndex(GeodeIndex),
}

impl Dialog {
    pub(crate) fn new_instance(launcher: &App) -> Self {
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

pub(crate) fn default_platform(launcher: &App) -> GdPlatform {
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
        Dialog::WineSetup(name) => show_wine_setup(ui, window, &mut open, name),
        Dialog::AddExisting(add) => {
            crate::ui::dialogs::add_existing::show(ui, window, &mut open, add)
        }
        Dialog::GeodeIndex(index) => {
            index.show(ui, &mut open, &window.app, &mut window.actions);
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
    let launcher = &window.app;
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
                let pre_2_2_on_mac = launcher.native_platform() == Some(GdPlatform::MacOs)
                    && release.as_ref().is_some_and(is_encrypted_mac_build);
                if pre_2_2_on_mac {
                    empty_label(ui);
                    ui.label(match platform {
                        GdPlatform::MacOs => {
                            "The macOS build before 2.2 is encrypted and won't open on a Mac"
                        }
                        GdPlatform::Windows => "Pre-2.2 plays the Windows build through Wine",
                    })
                    .muted();
                }
            });

            let find = |platform| {
                release
                    .as_ref()
                    .and_then(|version| launcher.catalog().find(version, platform))
            };
            let entry = find(*platform);
            // ask rather than quietly swapping in the windows build
            let wine_entry = (*platform == GdPlatform::MacOs
                && launcher.native_platform() == Some(GdPlatform::MacOs)
                && release.as_ref().is_some_and(is_encrypted_mac_build))
            .then(|| find(GdPlatform::Windows))
            .flatten();
            create = create_buttons(ui, entry, wine_entry, typed_name.ok(), submitted);
        });
    if let Some((name, entry)) = create {
        window.chosen = name
            .clone()
            .or_else(|| window.app.default_name(&entry.version));
        window.actions.push(Action::Create { name, entry });
    }
}

// a pre-2.2 macOS build won't open, so a Mac asks instead of quietly picking Windows
fn create_buttons(
    ui: &mut Ui,
    entry: Option<&CatalogEntry>,
    wine_entry: Option<&CatalogEntry>,
    name: Option<Option<InstanceName>>,
    submitted: bool,
) -> Option<(Option<InstanceName>, CatalogEntry)> {
    let mut chosen = None;
    ui.row().justify(Justify::End).gap(8).show(|ui| {
        if ui.button("Cancel").clicked() {
            ui.close_overlay();
        }
        if let Some(windows) = wine_entry {
            if ui
                .button("Use the macOS build")
                .enabled(entry.is_some() && name.is_some())
                .tooltip("It won't open")
                .clicked()
            {
                chosen = entry;
            }
            let wine = ui
                .button("Run through Wine")
                .primary()
                .enabled(name.is_some())
                .clicked();
            if wine || submitted {
                chosen = Some(windows);
            }
        } else {
            let can_create = entry.is_some() && name.is_some();
            let clicked = ui.button("Create").primary().enabled(can_create).clicked();
            if clicked || submitted {
                chosen = entry;
            }
        }
    });
    let picked = (name?, chosen?.clone());
    ui.close_overlay();
    Some(picked)
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

fn show_wine_setup(ui: &mut Ui, window: &mut Window, open: &mut bool, name: &InstanceName) {
    ui.modal(open)
        .title("Set up old versions")
        .style(|style| style.width(Fixed(DIALOG_WIDTH)))
        .show(|ui| {
            ui.label(
                "Pre-2.2 versions run their Windows build on a Mac. Setting up does this once:",
            );
            ui.label(format!("1. Downloads Wine {WINE_VERSION} (~215 MB)"));
            ui.label("2. Installs Steam for Windows inside Wine");
            ui.label("3. Opens it so you can sign in");
            ui.label("Takes ~3 GB in Glint's folder. Your normal Steam and saves aren't touched.")
                .muted();
            ui.row().justify(Justify::End).gap(8).show(|ui| {
                if ui.button("Cancel").clicked() {
                    ui.close_overlay();
                }
                if ui.button("Set up").primary().clicked() {
                    window.actions.push(Action::Play(name.clone()));
                    ui.close_overlay();
                }
            });
        });
}

fn newest(launcher: &App, platform: GdPlatform) -> Option<GdVersion> {
    launcher
        .catalog()
        .newest(platform)
        .map(|entry| entry.version.clone())
}
