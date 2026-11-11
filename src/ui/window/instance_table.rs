use crate::gd::GdVersion;
use crate::instances::{GameFiles, Instance, InstanceError};
use crate::mods::DllLoader;
use weft::core::{Corners, Rect, RoundedRect};
use weft::prelude::{Align, Color, Fit, Grow, Ui, vec2};
use weft::ui::{Column, SortDirection, TableSort};

use super::Window;
use crate::app::{Action, ModsState};
use crate::ui::dialogs::Dialog;
use crate::ui::icons::Icon;
use crate::ui::play_button::play;
use crate::ui::theme::{ACCENT, MUTED, era_color};

const VERSION_COLUMN: usize = 1;

const STAR: f32 = 12.0;

struct Row {
    instance: Option<Instance>,
    name: String,
    version: Option<GdVersion>,
    mods: String,
    state: String,
    is_active: bool,
    is_favorite: bool,
}

pub(super) fn show(ui: &mut Ui, window: &mut Window) {
    let table_id = ui.child_id("instances");
    let sort = ui.table_state(table_id, |state| state.sort());
    let rows = rows(window, sort);
    let selected = rows.iter().position(|row| {
        row.instance
            .as_ref()
            .is_some_and(|instance| window.chosen.as_ref() == Some(instance.name()))
    });
    ui.table_state(table_id, |state| match selected {
        Some(row) => state.selection_mut().select_only(row),
        None => state.selection_mut().clear(),
    });

    let table = ui
        .table(rows.len())
        .id(table_id)
        .name("Instance list")
        .column_with(Column::new("Name").width(Fit).min_width(90).sortable())
        .column_with(Column::new("Version").width(Fit).sortable())
        .column_with(Column::new("Mods").width(Grow(1)).min_width(90))
        .column_with(Column::new("State").width(Grow(1)).min_width(90))
        .row_height(26)
        .selectable()
        .striped(true)
        .height(Grow(1))
        .show(|ui, row, column| show_cell(ui, &rows[row], column));

    let instance_at = |row: Option<usize>| row.and_then(|row| rows[row].instance.as_ref());
    if table.selection_changed()
        && let Some(instance) = instance_at(table.selected_row())
    {
        window.chosen = Some(instance.name().clone());
    }
    let anchor = match table.context_row() {
        Some((row, response)) => {
            if let Some(instance) = instance_at(Some(row)) {
                window.chosen = Some(instance.name().clone());
            }
            response
        }
        None => table.response(),
    };
    ui.context_menu(&anchor)
        .id(table_id.with(&"row menu"))
        .show(|ui| row_menu(ui, window));
    if let Some(instance) = instance_at(table.activated_row())
        && instance.game_files() == GameFiles::Present
        && window.app.playing().is_none()
        && window.app.cant_play(instance).is_none()
        && let Some(click) = play(&window.app, Some(instance)).click
    {
        window.click(click);
    }
}

fn rows(window: &Window, sort: Option<TableSort>) -> Vec<Row> {
    let filter = window.filter.trim().to_lowercase();
    let mut found: Vec<&Instance> = window
        .instances()
        .filter(|instance| {
            filter.is_empty()
                || instance.name().as_str().to_lowercase().contains(&filter)
                || instance.config().version.to_string().contains(&filter)
        })
        .collect();
    found.sort_by(|left, right| match sort {
        Some(sort) => {
            let ordering = if sort.column == VERSION_COLUMN {
                left.config()
                    .version
                    .cmp(&right.config().version)
                    .then_with(|| left.name().cmp(right.name()))
            } else {
                left.name().cmp(right.name())
            };
            match sort.direction {
                SortDirection::Ascending => ordering,
                SortDirection::Descending => ordering.reverse(),
            }
        }
        None => {
            let is_favorite = |instance: &Instance| window.app.is_favorite(instance.name());
            is_favorite(right)
                .cmp(&is_favorite(left))
                .then_with(|| left.name().cmp(right.name()))
        }
    });
    let broken = window
        .app
        .instances()
        .iter()
        .filter_map(|entry| entry.as_ref().err())
        .map(|error| Row {
            instance: None,
            name: broken_name(error),
            version: None,
            mods: String::new(),
            state: "Unreadable".to_owned(),
            is_active: false,
            is_favorite: false,
        });
    found
        .into_iter()
        .map(|instance| {
            let (state, is_active) = state_text(window, instance);
            Row {
                name: instance.name().to_string(),
                version: Some(instance.config().version.clone()),
                mods: mods_text(window, instance),
                state,
                is_active,
                is_favorite: window.app.is_favorite(instance.name()),
                instance: Some(instance.clone()),
            }
        })
        .chain(broken)
        .collect()
}

fn mods_text(window: &Window, instance: &Instance) -> String {
    let mut parts = Vec::new();
    match window.app.mods_state(instance) {
        ModsState::Geode { mods: Ok(mods) } => {
            let on = mods.iter().filter(|found| found.is_enabled).count();
            return format!("Geode, {on} on");
        }
        ModsState::Geode { mods: Err(_) } => return "Geode".to_owned(),
        ModsState::Dll { loader, mods } => {
            parts.push(
                match loader {
                    DllLoader::MegaHack => "Mega Hack",
                    DllLoader::GlintLoader => "Glint Loader",
                }
                .to_owned(),
            );
            let on = mods.map_or(0, |mods| {
                mods.iter().filter(|found| found.is_enabled).count()
            });
            if on > 0 {
                parts.push(format!("{on} DLL"));
            }
        }
        ModsState::NeedsGeode | ModsState::NoLoader { .. } | ModsState::NoMacLoader => {}
    }
    if window
        .app
        .echo(instance)
        .is_some_and(|echo| echo.dll.is_enabled)
    {
        parts.push("Echo".to_owned());
    }
    if parts.is_empty() {
        return "—".to_owned();
    }
    parts.join(" + ")
}

fn state_text(window: &Window, instance: &Instance) -> (String, bool) {
    if window.app.playing() == Some(instance.name()) {
        return ("Playing".to_owned(), true);
    }
    if let Some(download) = window
        .app
        .download()
        .filter(|download| download.instance() == instance.name())
    {
        let percent = download
            .fraction()
            .map_or_else(String::new, |fraction| format!(" {:.0}%", fraction * 100.0));
        return (format!("Downloading{percent}"), true);
    }
    let text = match instance.game_files() {
        GameFiles::Present if instance.is_imported() => "Ready, imported",
        GameFiles::Present => "Ready",
        GameFiles::Incomplete => "Unfinished download",
        GameFiles::Missing => "Not downloaded",
    };
    (text.to_owned(), false)
}

fn show_cell(ui: &mut Ui, row: &Row, column: usize) {
    match column {
        0 => show_name_cell(ui, row),
        VERSION_COLUMN => {
            if let Some(version) = &row.version {
                swatch(ui, era_color(version));
                ui.label(version.to_string()).wrap(false);
            }
        }
        2 => {
            ui.label(row.mods.clone())
                .wrap(false)
                .style(|style| style.color(MUTED));
        }
        _ => {
            let color = if row.is_active { ACCENT } else { MUTED };
            ui.label(row.state.clone())
                .wrap(false)
                .style(|style| style.color(color));
        }
    }
}

fn show_name_cell(ui: &mut Ui, row: &Row) {
    if row.is_favorite {
        ui.canvas().size(STAR, STAR).show(|canvas, _, _| {
            Icon::StarFilled.draw(canvas, vec2(0.0, 0.0), STAR, ACCENT);
        });
    }
    let label = ui.label(row.name.clone()).wrap(false);
    if row.instance.is_none() {
        label.style(|style| style.color(MUTED));
    }
}

fn row_menu(ui: &mut Ui, window: &mut Window) {
    let Some(instance) = window.chosen_instance().cloned() else {
        return;
    };
    let name = instance.name().clone();
    let favorite = if window.app.is_favorite(&name) {
        "Remove from favorites"
    } else {
        "Add to favorites"
    };
    if ui.menu_item(favorite).clicked() {
        window.actions.push(Action::ToggleFavorite(name.clone()));
    }
    let is_default = window.app.default_instance() == Some(&name);
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
    let can_change = window.can_change(&instance);
    if ui.menu_item("Rename…").enabled(can_change).clicked() {
        window.dialog = Some(Dialog::rename(&name));
    }
    if ui.menu_item("Delete…").enabled(can_change).clicked() {
        window.dialog = Some(Dialog::delete(&instance));
    }
}

fn swatch(ui: &mut Ui, color: Color) {
    let side = 8.0;
    ui.canvas()
        .size(side, side)
        .style(|style| style.align_self(Align::Center))
        .show(|canvas, rect, _| {
            let square = Rect::from_min_size(vec2(0.0, 0.0), rect.size());
            canvas.fill_rounded_rect(RoundedRect::new(square, Corners::all(2.0)), color);
        });
}

fn broken_name(error: &InstanceError) -> String {
    match error {
        InstanceError::BadConfig { name, .. } => name.to_string(),
        InstanceError::BadFolderName { folder, .. } => folder
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
        _ => String::new(),
    }
}
