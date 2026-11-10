use crate::gd::{OLD_ON_NEW_SAVE, is_before_2_2};
use crate::instances::{GameFiles, Instance};
use weft::prelude::{Align, Fixed, FontWeight, Grow, Insets, Justify, Ui};

use super::Window;
use crate::app::{Action, ModLoader, SaveMode};
use crate::ui::dialogs::Dialog;
use crate::ui::icons::{Icon, icon_button};
use crate::ui::mod_picker::ModPicker;
use crate::ui::mods::browse::GeodeIndex;
use crate::ui::theme::{MUTED, TEXT};

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
    if window.app.mods_state(&instance).is_geode_grid() {
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
            let is_favorite = window.app.is_favorite(&name);
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
            let is_default = window.app.default_instance() == Some(&name);
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
    if window.app.default_instance() == Some(instance.name()) {
        facts.push_str(" - Default");
    }
    facts
}

fn show_fields(ui: &mut Ui, window: &mut Window, instance: &Instance) {
    let name = instance.name().clone();
    let files = match (window.is_downloading(instance), instance.game_files()) {
        (true, _) => "Downloading",
        (false, GameFiles::Present) if instance.is_imported() => "Imported install",
        (false, GameFiles::Present) => "Installed",
        (false, GameFiles::Incomplete) => "Unfinished download",
        (false, GameFiles::Missing) => "Not downloaded",
    };
    field(ui, "Files", files, None, &mut window.actions);
    field(
        ui,
        "Folder",
        &window.app.short_folder(instance),
        Some(("Open", Action::OpenFolder(name.clone()))),
        &mut window.actions,
    );
    show_save(ui, window, instance);
    show_link(ui, window, instance);
    let is_geode_era = instance.config().version >= "2.2".parse().expect("a fixed version parses");
    let (title, status, loader_action) = match window.app.mod_loader(instance) {
        _ if !is_geode_era => ("Mods", None, None),
        ModLoader::Geode if instance.is_imported() => ("Geode Mods", None, None),
        ModLoader::Geode => (
            "Geode Mods",
            None,
            Some(("Remove Geode", Action::RemoveGeode(name))),
        ),
        ModLoader::InstallingGeode => ("Mods", Some("Installing Geode…"), None),
        ModLoader::GeodeAvailable => (
            "Mods",
            Some("No mod loader"),
            Some(("Install Geode", Action::InstallGeode(name))),
        ),
        ModLoader::None => ("Mods", Some("No mod loader"), None),
    };
    let can_browse = window.app.mod_loader(instance) == ModLoader::Geode;
    let mut browse = false;
    ui.space(4);
    ui.row()
        .width(Grow(1))
        .gap(8)
        .align(Align::Center)
        .style(|style| style.min_height(28.0))
        .show(|ui| {
            ui.label(title)
                .wrap(false)
                .style(|style| style.weight(FontWeight::SEMI_BOLD).color(TEXT));
            if let Some(status) = status {
                ui.label(status)
                    .wrap(false)
                    .style(|style| style.color(MUTED));
            }
            ui.spacer();
            if let Some((label, action)) = loader_action
                && ui.button(label).height(24).clicked()
            {
                window.actions.push(action);
            }
            if can_browse {
                browse = ui
                    .button("Browse mods…")
                    .height(24)
                    .tooltip("Browse Geode mods")
                    .clicked();
                if ModPicker::is_available()
                    && ui
                        .button("Add mod…")
                        .height(24)
                        .enabled(!window.mod_picker.is_open())
                        .tooltip("Add a .geode file")
                        .clicked()
                {
                    window.mod_picker.open(instance, window.app.notify());
                }
            }
        });
    if browse {
        let index = GeodeIndex::new(instance, window.app.notify());
        window.dialog = Some(Dialog::GeodeIndex(index));
    }
    ui.column().width(Grow(1)).height(Grow(1)).show(|ui| {
        crate::ui::mods::show_mods(
            ui,
            &window.app,
            instance,
            &mut window.actions,
            &mut window.mod_picker,
        );
    });
}

fn show_save(ui: &mut Ui, window: &mut Window, instance: &Instance) {
    let name = instance.name().clone();
    let save_path = window.app.gd_save_dir(instance);
    let save_dir = short_save_dir(&window.app.short_path(&save_path));
    let tooltip = format!(
        "{}\n{}",
        save_path.display(),
        window.app.describe_save(instance)
    );
    let has_backups = !window.app.save_backups(&name).is_empty();
    ui.push_id("Save", |ui| {
        ui.row()
            .width(Grow(1))
            .gap(8)
            .align(Align::Center)
            .style(|style| style.min_height(28.0))
            .show(|ui| {
                field_label(ui, "Save");
                ui.label(save_dir)
                    .truncate()
                    .tooltip(tooltip)
                    .style(|style| style.color(TEXT).width(Grow(1)));
                if ui
                    .button("Backups…")
                    .height(24)
                    .enabled(has_backups)
                    .tooltip("Glint backs up the save before every launch")
                    .clicked()
                {
                    window.dialog = Some(Dialog::Backups(name.clone()));
                }
                if ui.button("Open").height(24).clicked() {
                    window.actions.push(Action::OpenSaveFolder(name.clone()));
                }
            });
    });
}

fn show_link(ui: &mut Ui, window: &mut Window, instance: &Instance) {
    let config = instance.config();
    let linked = window.app.save_mode(instance) == SaveMode::Linked;
    let blocker = window.app.link_blocker(config.platform, &config.version);
    let value = match (linked, &blocker) {
        (true, _) => "Yes".to_owned(),
        (false, Some(reason)) => format!("No - {reason}"),
        (false, None) => "No".to_owned(),
    };
    let about = if linked {
        "Uses your main save"
    } else {
        "Has its own save"
    };
    let mut copy_tip =
        "Replaces its own save with a copy of your main save. Glint backs up its own first"
            .to_owned();
    if is_before_2_2(&config.version) {
        copy_tip.push_str(&format!(". {OLD_ON_NEW_SAVE}"));
    }
    let can_change = window.can_change(instance);
    let can_copy = window.app.can_copy_main_save(instance);
    let name = instance.name().clone();
    ui.push_id("Linked", |ui| {
        ui.row()
            .width(Grow(1))
            .gap(8)
            .align(Align::Center)
            .style(|style| style.min_height(28.0))
            .show(|ui| {
                field_label(ui, "Linked");
                ui.label(value)
                    .truncate()
                    .tooltip(about)
                    .style(|style| style.color(TEXT).width(Grow(1)));
                if can_copy
                    && ui
                        .button("Copy main save")
                        .height(24)
                        .enabled(can_change)
                        .tooltip(copy_tip)
                        .clicked()
                {
                    window.actions.push(Action::CopyMainSave(name.clone()));
                }
                // a link that a newer main save blocks can still be cleared
                let (button, tip, link) = if instance.is_linked() {
                    ("Unlink", "Use its own save", false)
                } else {
                    (
                        "Link",
                        "Use your main save. Its own stays in its folder",
                        true,
                    )
                };
                let shown = instance.is_linked() || blocker.is_none();
                if shown
                    && ui
                        .button(button)
                        .height(24)
                        .enabled(can_change)
                        .tooltip(tip)
                        .clicked()
                {
                    window.actions.push(Action::SetLinked {
                        instance: name.clone(),
                        linked: link,
                    });
                }
            });
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
            ui.label("No instances")
                .font_size(15)
                .style(|style| style.weight(FontWeight::SEMI_BOLD).color(TEXT));
            ui.label("Make one with New… or add an existing GD folder.")
                .muted();
        });
}
