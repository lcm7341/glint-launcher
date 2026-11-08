use std::time::SystemTime;

use crate::instances::Instance;
use crate::mods::GeodeMod;
use weft::prelude::{Align, Fixed, FontWeight, Grow, Insets, Sense, Transition, Ui};
use weft::text::FontFamily;
use weft::ui::Border;

use crate::glint_theme::{HAIRLINE, HOVERED, MUTED, RAISED, TEXT, WELL};
use crate::icons::{Icon, icon_button};
use crate::launcher::{Action, Launcher, ModChange};
use crate::markdown_view::show_markdown;

const COLUMNS: usize = 2;

const CARD_HEIGHT: f32 = 112.0;

const GAP: f32 = 8.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Sort {
    #[default]
    Name,
    OnFirst,
    RecentlyUpdated,
    Developer,
}

impl std::fmt::Display for Sort {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Sort::Name => "Name",
            Sort::OnFirst => "On first",
            Sort::RecentlyUpdated => "Recently updated",
            Sort::Developer => "Developer",
        })
    }
}

#[derive(Default)]
struct GridState {
    query: String,
    sort: Sort,
    details: Option<String>,
}

pub(crate) fn show_geode_mods(
    ui: &mut Ui,
    launcher: &Launcher,
    instance: &Instance,
    mods: std::rc::Rc<[GeodeMod]>,
    actions: &mut Vec<Action>,
) {
    let change = |change| Action::Mod {
        instance: instance.name().clone(),
        change,
    };
    let state_id = ui.child_id(("geode grid", instance.name().as_str()));
    let (mut query, mut sort, details) = ui.memory(state_id, |state: &mut GridState| {
        (state.query.clone(), state.sort, state.details.clone())
    });
    let is_open = launcher.is_game_open(instance);
    let on = mods.iter().filter(|found| found.is_enabled).count();

    ui.row()
        .width(Grow(1))
        .gap(8)
        .align(Align::Center)
        .show(|ui| {
            let mut caption = format!("{on} of {} on", mods.len());
            if is_open {
                caption.push_str(". Close the game to change them");
            }
            ui.label(caption)
                .muted()
                .font_size(12)
                .wrap(false)
                .style(|style| style.width(Grow(1)).clip(true));
            ui.text_input(&mut query)
                .name("Search mods")
                .placeholder("Search mods")
                .width(Fixed(150.0));
            let sorts = [
                Sort::Name,
                Sort::OnFirst,
                Sort::RecentlyUpdated,
                Sort::Developer,
            ];
            ui.dropdown(&mut sort, sorts)
                .name("Sort mods")
                .labels(sorts.map(|sort| sort.to_string()))
                .style(|style| style.width(Fixed(150.0)))
                .show();
        });

    let shown = ordered(&mods, &query, sort);
    if shown.is_empty() {
        ui.label("No mods match.").muted();
    }
    let mut open_details = None;
    let rows = shown.len().div_ceil(COLUMNS);
    ui.virtual_list(rows)
        .row_height(CARD_HEIGHT + GAP)
        .height(Grow(1))
        .width(Grow(1))
        .name("Geode mods")
        .show(|ui, row| {
            ui.row()
                .gap(GAP)
                .width(Grow(1))
                .height(Fixed(CARD_HEIGHT))
                .show(|ui| {
                    for column in 0..COLUMNS {
                        let Some(found) = shown.get(row * COLUMNS + column) else {
                            ui.column().width(Grow(1)).show(|_| {});
                            continue;
                        };
                        ui.push_id(&found.id, |ui| {
                            if show_card(ui, found, is_open, &change, actions) {
                                open_details = Some(found.id.clone());
                            }
                        });
                    }
                });
        });

    let mut details = open_details.or(details);
    if let Some(id) = details.clone()
        && let Some(found) = mods.iter().find(|found| found.id == id)
    {
        let mut is_shown = true;
        ui.modal(&mut is_shown)
            .title(found.name.clone())
            .style(|style| style.width(Fixed(620.0)))
            .show(|ui| {
                ui.scroll_area()
                    .name("Mod details")
                    .height(Fixed(480.0))
                    .show(|ui| show_details(ui, instance, found, is_open, &change, actions));
            });
        if !is_shown {
            details = None;
        }
    }
    ui.memory(state_id, |state: &mut GridState| {
        state.query = query;
        state.sort = sort;
        state.details = details;
    });
}

fn ordered<'a>(mods: &'a [GeodeMod], query: &str, sort: Sort) -> Vec<&'a GeodeMod> {
    let query = query.trim().to_lowercase();
    let mut shown: Vec<&GeodeMod> = mods
        .iter()
        .filter(|found| {
            query.is_empty()
                || found.name.to_lowercase().contains(&query)
                || found.id.contains(&query)
                || found
                    .info
                    .developers
                    .iter()
                    .any(|developer| developer.to_lowercase().contains(&query))
        })
        .collect();
    let by_name = |a: &GeodeMod, b: &GeodeMod| a.name.to_lowercase().cmp(&b.name.to_lowercase());
    shown.sort_by(|a, b| {
        let then = match sort {
            Sort::Name => by_name(a, b),
            Sort::OnFirst => b.is_enabled.cmp(&a.is_enabled).then_with(|| by_name(a, b)),
            Sort::RecentlyUpdated => b.info.updated.cmp(&a.info.updated),
            Sort::Developer => developer(a).cmp(&developer(b)).then_with(|| by_name(a, b)),
        };
        b.is_pinned.cmp(&a.is_pinned).then(then)
    });
    shown
}

fn developer(found: &GeodeMod) -> String {
    found
        .info
        .developers
        .first()
        .map(|name| name.to_lowercase())
        .unwrap_or_default()
}

fn show_card(
    ui: &mut Ui,
    found: &GeodeMod,
    is_open: bool,
    change: &impl Fn(ModChange) -> Action,
    actions: &mut Vec<Action>,
) -> bool {
    let card = ui
        .column()
        .gap(4)
        .width(Grow(1))
        .height(Fixed(CARD_HEIGHT))
        .padding(Insets::new(8.0, 10.0, 10.0, 10.0))
        .sense(Sense::CLICK)
        .style(|style| {
            let rest = if found.is_pinned { RAISED } else { WELL };
            style
                .bg(rest)
                .border(Border::new(1.0, HAIRLINE))
                .radius(6)
                .transition(Transition::millis(80))
                .hover(|state| state.bg(HOVERED))
        })
        .show(|ui| {
            ui.row()
                .gap(8)
                .align(Align::Center)
                .width(Grow(1))
                .show(|ui| {
                    let mut on = found.is_enabled;
                    let toggled = ui
                        .toggle(&mut on, "")
                        .name(format!("Load {}", found.name))
                        .enabled(!is_open)
                        .changed();
                    if toggled {
                        actions.push(change(ModChange::SetGeodeEnabled {
                            id: found.id.clone(),
                            enabled: on,
                        }));
                    }
                    ui.label(found.name.clone()).truncate().style(|style| {
                        style
                            .weight(FontWeight::SEMI_BOLD)
                            .color(TEXT)
                            .width(Grow(1))
                    });
                    show_pin(ui, found, is_open, change, actions);
                });
            ui.label(byline(found)).muted().font_size(12).truncate();
            let description = found.info.description.as_deref().unwrap_or_default();
            ui.label(description.split_whitespace().collect::<Vec<_>>().join(" "))
                .font_size(12)
                .max_lines(2)
                .style(|style| style.color(MUTED));
        })
        .response;
    ui.tooltip_for(&card, "Details");
    card.clicked()
}

fn show_pin(
    ui: &mut Ui,
    found: &GeodeMod,
    is_open: bool,
    change: &impl Fn(ModChange) -> Action,
    actions: &mut Vec<Action>,
) {
    let (pin, tip) = if found.is_pinned {
        (Icon::PinFilled, "Unpin")
    } else {
        (Icon::Pin, "Pin to the top")
    };
    if icon_button(ui, pin, tip, false).clicked() && !is_open {
        actions.push(change(ModChange::SetGeodePinned {
            id: found.id.clone(),
            pinned: !found.is_pinned,
        }));
    }
}

fn byline(found: &GeodeMod) -> String {
    match found.info.developers.as_slice() {
        [] => found.version.clone(),
        [only] => format!("{} · {only}", found.version),
        [first, rest @ ..] => format!("{} · {first} +{}", found.version, rest.len()),
    }
}

fn show_details(
    ui: &mut Ui,
    instance: &Instance,
    found: &GeodeMod,
    is_open: bool,
    change: &impl Fn(ModChange) -> Action,
    actions: &mut Vec<Action>,
) {
    let info = &found.info;
    ui.column().gap(14).width(Grow(1)).show(|ui| {
        ui.label(byline(found)).muted();
        ui.row().gap(6).show(|ui| {
            let state = if found.is_enabled { "On" } else { "Off" };
            let pinned = found.is_pinned.then_some("Pinned");
            let marks = [Some(state), pinned]
                .into_iter()
                .flatten()
                .map(str::to_owned)
                .chain(info.tags.iter().cloned());
            for (index, mark) in marks.enumerate() {
                ui.push_id(index, |ui| chip(ui, mark));
            }
        });
        if let Some(description) = &info.description {
            ui.label(description.clone())
                .style(|style| style.color(TEXT));
        }
        let mut facts = vec![("ID", found.id.clone())];
        if !info.developers.is_empty() {
            facts.push(("By", info.developers.join(", ")));
        }
        if let Some(gd) = &info.gd {
            facts.push(("Made for GD", gd.clone()));
        }
        if let Some(geode) = &info.geode {
            facts.push(("Needs Geode", geode.clone()));
        }
        if let Some(updated) = info.updated {
            facts.push(("Updated", ago(updated)));
        }
        facts.push(("Size", size(info.size)));
        section(ui, "Facts", |ui| {
            for (index, (label, value)) in facts.into_iter().enumerate() {
                ui.push_id(index, |ui| fact(ui, label, &value));
            }
        });
        if !info.dependencies.is_empty() {
            section(ui, "Needs", |ui| {
                for (index, (id, version)) in info.dependencies.iter().enumerate() {
                    ui.push_id(index, |ui| dependency(ui, id, version));
                }
            });
        }
        if !info.links.is_empty() {
            ui.row().gap(6).show(|ui| {
                for (kind, url) in &info.links {
                    let label = capitalized(kind);
                    if ui.button(label).tooltip(url.clone()).clicked() {
                        actions.push(Action::OpenLink(url.clone()));
                    }
                }
            });
        }
        if let Some(about) = crate::mods::read_about(&instance.game_dir(), &found.id) {
            section(ui, "About", |ui| show_markdown(ui, &about));
        }
        ui.separator();
        show_sheet_controls(ui, found, is_open, change, actions);
    });
}

fn show_sheet_controls(
    ui: &mut Ui,
    found: &GeodeMod,
    is_open: bool,
    change: &impl Fn(ModChange) -> Action,
    actions: &mut Vec<Action>,
) {
    ui.row().gap(12).align(Align::Center).show(|ui| {
        let mut on = found.is_enabled;
        let toggled = ui
            .toggle(&mut on, "Load this mod")
            .enabled(!is_open)
            .changed();
        if toggled {
            actions.push(change(ModChange::SetGeodeEnabled {
                id: found.id.clone(),
                enabled: on,
            }));
        }
        ui.spacer();
        show_pin(ui, found, is_open, change, actions);
    });
}

fn section(ui: &mut Ui, title: &str, body: impl FnOnce(&mut Ui)) {
    ui.push_id(title, |ui| {
        ui.column().gap(6).width(Grow(1)).show(|ui| {
            ui.label(title)
                .font_size(12)
                .style(|style| style.weight(FontWeight::SEMI_BOLD).color(MUTED));
            body(ui);
        });
    });
}

fn fact(ui: &mut Ui, label: &str, value: &str) {
    ui.row().gap(12).width(Grow(1)).show(|ui| {
        ui.label(label.to_owned())
            .wrap(false)
            .style(|style| style.color(MUTED).width(Fixed(110.0)).clip(true));
        ui.label(value.to_owned())
            .font_size(12)
            .style(|style| style.font_family(FontFamily::Monospace).color(TEXT));
    });
}

fn dependency(ui: &mut Ui, id: &str, version: &str) {
    ui.row().gap(12).width(Grow(1)).show(|ui| {
        ui.label(id.to_owned())
            .font_size(12)
            .wrap(false)
            .style(|style| {
                style
                    .font_family(FontFamily::Monospace)
                    .color(TEXT)
                    .width(Grow(1))
                    .clip(true)
            });
        ui.label(version.to_owned())
            .font_size(12)
            .wrap(false)
            .style(|style| style.color(MUTED));
    });
}

fn chip(ui: &mut Ui, text: String) {
    ui.label(text).font_size(12).wrap(false).style(|style| {
        style
            .color(MUTED)
            .bg(RAISED)
            .radius(4)
            .padding(Insets::symmetric(2.0, 8.0))
    });
}

fn capitalized(word: &str) -> String {
    let mut letters = word.chars();
    letters
        .next()
        .map(|first| first.to_uppercase().chain(letters).collect())
        .unwrap_or_default()
}

fn ago(time: SystemTime) -> String {
    let days = SystemTime::now()
        .duration_since(time)
        .map_or(0, |since| since.as_secs() / 86_400);
    match days {
        0 => "Today".to_owned(),
        1 => "Yesterday".to_owned(),
        2..=59 => format!("{days} days ago"),
        _ => format!("{} months ago", days / 30),
    }
}

fn size(bytes: u64) -> String {
    const MB: u64 = 1024 * 1024;
    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else {
        format!("{} KB", bytes.div_ceil(1024))
    }
}
