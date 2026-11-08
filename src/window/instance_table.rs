use crate::gd::GdVersion;
use crate::instances::{GameFiles, Instance, InstanceError};
use crate::mods::DllLoader;
use weft::core::{Corners, Rect, RoundedRect};
use weft::prelude::{Align, Color, Fixed, Grow, Ui, vec2};
use weft::ui::{Column, SortDirection, TableSort};

use super::Window;
use crate::dialogs::Dialog;
use crate::glint_theme::{ACCENT, MUTED, era_color};
use crate::icons::Icon;
use crate::launcher::{Action, ModsState};
use crate::play::play;

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
    can_change: bool,
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

    let dialog = &mut window.dialog;
    let actions = &mut window.actions;
    let table = ui
        .table(rows.len())
        .id(table_id)
        .name("Instance list")
        .column_with(Column::new("Name").width(Grow(3)).min_width(90).sortable())
        .column_with(Column::new("Version").width(Fixed(84.0)).sortable())
        .column_with(Column::new("Mods").width(Grow(2)).min_width(70))
        .column_with(Column::new("State").width(Grow(2)).min_width(80))
        .row_height(26)
        .selectable()
        .striped(true)
        .height(Grow(1))
        .show(|ui, row, column| show_cell(ui, &rows[row], column, dialog, actions));

    let instance_at = |row: Option<usize>| row.and_then(|row| rows[row].instance.as_ref());
    if table.selection_changed()
        && let Some(instance) = instance_at(table.selected_row())
    {
        window.chosen = Some(instance.name().clone());
    }
    if let Some(instance) = instance_at(table.activated_row())
        && instance.game_files() == GameFiles::Present
        && window.launcher.playing().is_none()
        && window.launcher.cant_play().is_none()
        && let Some(action) = play(&window.launcher, Some(instance)).action
    {
        window.actions.push(action);
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
            let is_favorite = |instance: &Instance| window.launcher.is_favorite(instance.name());
            is_favorite(right)
                .cmp(&is_favorite(left))
                .then_with(|| left.name().cmp(right.name()))
        }
    });
    let broken = window
        .launcher
        .instances()
        .iter()
        .filter_map(|entry| entry.as_ref().err())
        .map(|error| Row {
            instance: None,
            name: broken_name(error),
            version: None,
            mods: String::new(),
            state: "Can't be read".to_owned(),
            is_active: false,
            is_favorite: false,
            can_change: false,
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
                is_favorite: window.launcher.is_favorite(instance.name()),
                can_change: window.can_change(instance),
                instance: Some(instance.clone()),
            }
        })
        .chain(broken)
        .collect()
}

fn mods_text(window: &Window, instance: &Instance) -> String {
    let mut parts = Vec::new();
    match window.launcher.mods_state(instance) {
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
        ModsState::NeedsGeode | ModsState::NoLoader { .. } => {}
    }
    if window
        .launcher
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
    if window.launcher.playing() == Some(instance.name()) {
        return ("Playing".to_owned(), true);
    }
    if let Some(download) = window
        .launcher
        .download()
        .filter(|download| download.instance() == instance.name())
    {
        let percent = download
            .fraction()
            .map_or_else(String::new, |fraction| format!(" {:.0}%", fraction * 100.0));
        return (format!("Downloading{percent}"), true);
    }
    let text = match instance.game_files() {
        GameFiles::Present if instance.is_linked() => "Ready, linked",
        GameFiles::Present => "Ready",
        GameFiles::Incomplete => "Partly downloaded",
        GameFiles::Missing => "Not downloaded",
    };
    (text.to_owned(), false)
}

fn show_cell(
    ui: &mut Ui,
    row: &Row,
    column: usize,
    dialog: &mut Option<Dialog>,
    actions: &mut Vec<Action>,
) {
    match column {
        0 => show_name_cell(ui, row, dialog, actions),
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

fn show_name_cell(ui: &mut Ui, row: &Row, dialog: &mut Option<Dialog>, actions: &mut Vec<Action>) {
    if row.is_favorite {
        ui.canvas().size(STAR, STAR).show(|canvas, _, _| {
            Icon::StarFilled.draw(canvas, vec2(0.0, 0.0), STAR, ACCENT);
        });
    }
    let label = ui.label(row.name.clone()).wrap(false);
    let Some(instance) = &row.instance else {
        label.style(|style| style.color(MUTED));
        return;
    };
    let label = label.show();
    let name = instance.name();
    ui.context_menu(&label).show(|ui| {
        let favorite = if row.is_favorite {
            "Remove from favorites"
        } else {
            "Add to favorites"
        };
        if ui.menu_item(favorite).clicked() {
            actions.push(Action::ToggleFavorite(name.clone()));
        }
        if ui.menu_item("Set as default").clicked() {
            actions.push(Action::SetDefault(name.clone()));
        }
        if ui.menu_item("Open folder").clicked() {
            actions.push(Action::OpenFolder(name.clone()));
        }
        ui.separator();
        if ui.menu_item("Rename…").enabled(row.can_change).clicked() {
            *dialog = Some(Dialog::rename(name));
        }
        if ui.menu_item("Delete…").enabled(row.can_change).clicked() {
            *dialog = Some(Dialog::delete(instance));
        }
    });
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
