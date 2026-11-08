use crate::instances::{GameFiles, Instance};
use weft::prelude::{Align, Fixed, FontWeight, Grow, Insets, Justify, Ui};

use super::Window;
use crate::dialogs::Dialog;
use crate::glint_theme::{MUTED, TEXT};
use crate::icons::{Icon, icon_button};
use crate::launcher::{Action, ModLoader};

const LABEL_WIDTH: f32 = 96.0;

const MARGIN: f32 = 14.0;

pub(super) fn show(ui: &mut Ui, window: &mut Window) {
    let Some(instance) = window.chosen_instance().cloned() else {
        show_empty(ui);
        return;
    };
    let content = |window: &mut Window, ui: &mut Ui| {
        ui.column()
            .width(Grow(1))
            .height(Grow(1))
            .padding(Insets::new(10.0, MARGIN, MARGIN, MARGIN))
            .gap(4)
            .show(|ui| {
                show_title(ui, window, &instance);
                ui.space(4);
                show_fields(ui, window, &instance);
            });
    };
    if window.launcher.mods_state(&instance).is_geode_grid() {
        content(window, ui);
    } else {
        ui.scroll_area()
            .name("Instance")
            .height(Grow(1))
            .show(|ui| content(window, ui));
    }
}

fn show_title(ui: &mut Ui, window: &mut Window, instance: &Instance) {
    let name = instance.name().clone();
    ui.row()
        .width(Grow(1))
        .gap(10)
        .align(Align::Center)
        .show(|ui| {
            ui.label(name.to_string())
                .font_size(15)
                .truncate()
                .style(|style| style.weight(FontWeight::SEMI_BOLD).color(TEXT));
            ui.row()
                .width(Grow(1))
                .gap(4)
                .align(Align::Center)
                .show(|ui| {
                    ui.label(format!("Geometry Dash {}", instance.config().version))
                        .wrap(false)
                        .style(|style| style.color(MUTED));
                    ui.label(facts(window, instance))
                        .truncate()
                        .style(|style| style.color(MUTED).width(Grow(1)));
                });
            let is_favorite = window.launcher.is_favorite(&name);
            let (star, tip) = if is_favorite {
                (Icon::StarFilled, "Remove from favorites")
            } else {
                (Icon::Star, "Add to favorites")
            };
            if icon_button(ui, star, tip, false).clicked() {
                window.actions.push(Action::ToggleFavorite(name.clone()));
            }
            let more = icon_button(ui, Icon::More, "More", false);
            let can_change = window.can_change(instance);
            let is_default = window.launcher.default_instance() == Some(&name);
            ui.menu(&more).show(|ui| {
                if ui
                    .menu_item("Set as default")
                    .enabled(!is_default)
                    .clicked()
                {
                    window.actions.push(Action::SetDefault(name.clone()));
                }
                if ui.menu_item("Open folder").clicked() {
                    window.actions.push(Action::OpenFolder(name.clone()));
                }
                ui.separator();
                if ui.menu_item("Rename…").enabled(can_change).clicked() {
                    window.dialog = Some(Dialog::rename(&name));
                }
                if ui.menu_item("Delete…").enabled(can_change).clicked() {
                    window.dialog = Some(Dialog::delete(instance));
                }
            });
        });
}

fn facts(window: &Window, instance: &Instance) -> String {
    let mut facts = format!("- {}", instance.config().platform.display_name());
    if window.launcher.default_instance() == Some(instance.name()) {
        facts.push_str(" - Default");
    }
    facts
}

fn show_fields(ui: &mut Ui, window: &mut Window, instance: &Instance) {
    let name = instance.name().clone();
    let files = match (window.is_downloading(instance), instance.game_files()) {
        (true, _) => "Downloading",
        (false, GameFiles::Present) if instance.is_linked() => "Linked install",
        (false, GameFiles::Present) => "Installed",
        (false, GameFiles::Incomplete) => "Partly downloaded",
        (false, GameFiles::Missing) => "Not downloaded",
    };
    field(ui, "Files", files, None, &mut window.actions);
    field(
        ui,
        "Folder",
        &window.launcher.short_folder(instance),
        Some(("Open", Action::OpenFolder(name.clone()))),
        &mut window.actions,
    );
    let save_path = window.launcher.gd_save_dir(instance);
    let save_dir = short_save_dir(&window.launcher.short_path(&save_path));
    let source = window.launcher.describe_save(instance);
    field_with_tooltip(
        ui,
        "Save",
        &save_dir,
        &format!("{}\n{source}", save_path.display()),
        Some(("Open", Action::OpenSaveFolder(name.clone()))),
        &mut window.actions,
    );
    let loader = if instance.config().version >= "2.2".parse().expect("a fixed version parses") {
        match window.launcher.mod_loader(instance) {
            ModLoader::Geode if instance.is_linked() => Some(("Geode", None)),
            ModLoader::Geode => Some(("Geode", Some(("Remove Geode", Action::RemoveGeode(name))))),
            ModLoader::InstallingGeode => Some(("Installing Geode…", None)),
            ModLoader::GeodeAvailable => Some((
                "No mod loader",
                Some(("Install Geode", Action::InstallGeode(name))),
            )),
            ModLoader::None => Some(("No mod loader", None)),
        }
    } else {
        None
    };
    ui.space(4);
    ui.row()
        .width(Grow(1))
        .gap(8)
        .align(Align::Center)
        .style(|style| style.min_height(28.0))
        .show(|ui| {
            ui.label("Mods")
                .wrap(false)
                .style(|style| style.weight(FontWeight::SEMI_BOLD).color(TEXT));
            if let Some((text, action)) = loader {
                ui.label(format!("· {text}"))
                    .wrap(false)
                    .style(|style| style.color(MUTED).width(Grow(1)));
                if let Some((label, action)) = action
                    && ui.button(label).height(24).clicked()
                {
                    window.actions.push(action);
                }
            }
        });
    ui.column().width(Grow(1)).height(Grow(1)).show(|ui| {
        crate::mods_view::show_mods(ui, &window.launcher, instance, &mut window.actions);
    });
}

fn field(
    ui: &mut Ui,
    label: &str,
    value: &str,
    action: Option<(&str, Action)>,
    actions: &mut Vec<Action>,
) {
    field_with_tooltip(ui, label, value, value, action, actions);
}

fn field_with_tooltip(
    ui: &mut Ui,
    label: &str,
    value: &str,
    tooltip: &str,
    action: Option<(&str, Action)>,
    actions: &mut Vec<Action>,
) {
    ui.push_id(label, |ui| {
        ui.row()
            .width(Grow(1))
            .gap(8)
            .align(Align::Center)
            .style(|style| style.min_height(28.0))
            .show(|ui| {
                field_label(ui, label);
                ui.label(value)
                    .truncate()
                    .tooltip(tooltip)
                    .style(|style| style.color(TEXT).width(Grow(1)));
                if let Some((label, action)) = action
                    && ui.button(label).height(24).clicked()
                {
                    actions.push(action);
                }
            });
    });
}

fn short_save_dir(path: &str) -> String {
    let path = path.replace("/pfx/drive_c/users/steamuser/AppData/Local/", "/…/");
    for steam in [
        "~/.local/share/Steam/steamapps/",
        "~/.steam/steam/steamapps/",
    ] {
        if let Some(rest) = path.strip_prefix(steam) {
            return format!("Steam/{rest}");
        }
    }
    path
}

fn field_label(ui: &mut Ui, label: &str) {
    ui.label(label)
        .wrap(false)
        .style(|style| style.color(MUTED).width(Fixed(LABEL_WIDTH)));
}

fn show_empty(ui: &mut Ui) {
    ui.column()
        .width(Grow(1))
        .height(Grow(1))
        .align(Align::Center)
        .justify(Justify::Center)
        .gap(6)
        .show(|ui| {
            ui.label("No instances yet")
                .font_size(15)
                .style(|style| style.weight(FontWeight::SEMI_BOLD).color(TEXT));
            ui.label("Make one with New… or add an existing GD folder.")
                .muted();
        });
}
