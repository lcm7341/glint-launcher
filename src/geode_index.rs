use crate::gd::{GdPlatform, GdVersion, InstanceName};
use crate::instances::Instance;
use crate::mods::{INDEX_PAGE_SIZE, IndexMod, IndexPage, search_geode_index};
use weft::prelude::{Align, Fixed, FontWeight, Grow, Insets, Ui};
use weft::ui::Border;

use crate::glint_theme::{HAIRLINE, MUTED, TEXT, WELL};
use crate::job::Job;
use crate::launcher::{Action, Launcher, ModChange, ModsState, Notify};
use crate::sentence::capitalized;

const WIDTH: f32 = 640.0;

#[derive(Clone)]
struct Target {
    instance: InstanceName,
    gd: GdVersion,
    platform: GdPlatform,
}

#[derive(Default)]
pub(crate) struct GeodeIndex {
    target: Option<Target>,
    query: String,
    searched: String,
    mods: Vec<IndexMod>,
    total: u64,
    page: u32,
    pending: Option<Job<Result<IndexPage, String>>>,
    error: Option<String>,
}

impl GeodeIndex {
    pub(crate) fn new(instance: &Instance, notify: Notify) -> Self {
        let config = instance.config();
        let mut index = Self {
            target: Some(Target {
                instance: instance.name().clone(),
                gd: config.version.clone(),
                platform: config.platform,
            }),
            ..Self::default()
        };
        index.search(notify);
        index
    }

    fn search(&mut self, notify: Notify) {
        self.searched = self.query.trim().to_owned();
        self.mods.clear();
        self.total = 0;
        self.page = 0;
        self.load_page(1, notify);
    }

    fn load_page(&mut self, page: u32, notify: Notify) {
        let Some(target) = self.target.clone() else {
            return;
        };
        self.error = None;
        let query = self.searched.clone();
        self.pending = Some(Job::spawn(&notify, move || {
            search_geode_index(&target.gd, target.platform, &query, page)
                .map_err(|error| capitalized(&error.to_string()))
        }));
        self.page = page;
    }

    fn poll(&mut self) {
        let Some(pending) = &self.pending else {
            return;
        };
        let Some(result) = pending.take() else {
            return;
        };
        self.pending = None;
        match result {
            Ok(page) => {
                self.total = page.total;
                for found in page.mods {
                    if !self.mods.iter().any(|known| known.id == found.id) {
                        self.mods.push(found);
                    }
                }
            }
            Err(error) => {
                self.error = Some(error);
                self.page = self.page.saturating_sub(1);
            }
        }
    }

    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        open: &mut bool,
        launcher: &Launcher,
        actions: &mut Vec<Action>,
    ) {
        self.poll();
        let Some(target) = self.target.clone() else {
            *open = false;
            return;
        };
        let instance = launcher
            .instances()
            .iter()
            .filter_map(|entry| entry.as_ref().ok())
            .find(|instance| instance.name() == &target.instance)
            .cloned();
        let Some(instance) = instance else {
            *open = false;
            return;
        };
        let notify = launcher.notify();
        let mut search = false;
        let mut load_more = false;
        ui.modal(open)
            .title(format!("Geode mods for GD {}", target.gd))
            .style(|style| style.width(Fixed(WIDTH)))
            .show(|ui| {
                ui.row()
                    .width(Grow(1))
                    .gap(8)
                    .align(Align::Center)
                    .show(|ui| {
                        search |= ui
                            .text_input(&mut self.query)
                            .name("Search Geode's mods")
                            .placeholder("Search Geode's mods")
                            .width(Grow(1))
                            .submitted();
                        search |= ui.button("Search").clicked();
                    });
                self.show_results(ui, launcher, &instance, actions);
                load_more = self.show_footer(ui);
            });
        if search {
            self.search(notify);
        } else if load_more {
            self.load_page(self.page + 1, notify);
        }
    }

    fn show_results(
        &self,
        ui: &mut Ui,
        launcher: &Launcher,
        instance: &Instance,
        actions: &mut Vec<Action>,
    ) {
        let installed: Vec<String> = match launcher.mods_state(instance) {
            ModsState::Geode { mods: Ok(mods) } => {
                mods.iter().map(|found| found.id.clone()).collect()
            }
            _ => Vec::new(),
        };
        let installing = launcher.installing_mod(instance.name());
        let is_open = launcher.is_game_open(instance);
        ui.scroll_area()
            .name("Geode's mods")
            .height(Fixed(420.0))
            .show(|ui| {
                ui.column().gap(6).width(Grow(1)).show(|ui| {
                    if self.mods.is_empty() {
                        let text = match (&self.error, &self.pending) {
                            (Some(_), _) => String::new(),
                            (None, Some(_)) => "Loading…".to_owned(),
                            (None, None) if self.searched.is_empty() => {
                                format!("No mods for GD {} yet.", instance.config().version)
                            }
                            (None, None) => format!("No mods match \"{}\".", self.searched),
                        };
                        if !text.is_empty() {
                            ui.label(text).muted();
                        }
                    }
                    for found in &self.mods {
                        let state = if installing == Some(found.id.as_str()) {
                            InstallState::Installing
                        } else if installed.contains(&found.id) {
                            InstallState::Installed
                        } else if is_open {
                            InstallState::GameOpen
                        } else if launcher.installing_geode() {
                            InstallState::Busy
                        } else {
                            InstallState::Available
                        };
                        ui.push_id(&found.id, |ui| {
                            if show_mod(ui, found, state) {
                                actions.push(Action::Mod {
                                    instance: instance.name().clone(),
                                    change: ModChange::InstallFromIndex {
                                        id: found.id.clone(),
                                        name: found.name.clone(),
                                    },
                                });
                            }
                        });
                    }
                });
            });
    }

    fn show_footer(&self, ui: &mut Ui) -> bool {
        let mut load_more = false;
        ui.row()
            .width(Grow(1))
            .gap(8)
            .align(Align::Center)
            .show(|ui| {
                let text = match &self.error {
                    Some(error) => error.clone(),
                    None if self.mods.is_empty() => String::new(),
                    None => format!("{} of {} mods", self.mods.len(), self.total),
                };
                ui.label(text)
                    .muted()
                    .font_size(12)
                    .max_lines(2)
                    .style(|style| style.width(Grow(1)));
                let has_more = (self.mods.len() as u64) < self.total
                    && u64::from(self.page) * u64::from(INDEX_PAGE_SIZE) < self.total;
                let label = match (&self.pending, &self.error) {
                    (Some(_), _) if !self.mods.is_empty() => Some("Loading…"),
                    (None, Some(_)) => Some("Try again"),
                    (None, None) if has_more => Some("Load more"),
                    _ => None,
                };
                if let Some(label) = label {
                    load_more = ui.button(label).enabled(self.pending.is_none()).clicked();
                }
            });
        load_more
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InstallState {
    Available,
    Installed,
    Installing,
    Busy,
    GameOpen,
}

fn show_mod(ui: &mut Ui, found: &IndexMod, state: InstallState) -> bool {
    let mut install = false;
    ui.row()
        .width(Grow(1))
        .gap(10)
        .align(Align::Center)
        .padding(Insets::new(8.0, 10.0, 8.0, 10.0))
        .style(|style| style.bg(WELL).border(Border::new(1.0, HAIRLINE)).radius(6))
        .show(|ui| {
            ui.column().gap(3).width(Grow(1)).show(|ui| {
                ui.label(found.name.clone())
                    .truncate()
                    .style(|style| style.weight(FontWeight::SEMI_BOLD).color(TEXT));
                ui.label(byline(found)).muted().font_size(12).truncate();
                let description = found.description.as_deref().unwrap_or_default();
                ui.label(description.split_whitespace().collect::<Vec<_>>().join(" "))
                    .font_size(12)
                    .max_lines(2)
                    .style(|style| style.color(MUTED));
            });
            let (label, tooltip) = match state {
                InstallState::Available => ("Install", "Install with dependencies"),
                InstallState::Installed => ("Installed", "Already installed"),
                InstallState::Installing => ("Installing…", "Downloading"),
                InstallState::Busy => ("Install", "Another install is running"),
                InstallState::GameOpen => ("Install", "Close the game first"),
            };
            install = ui
                .button(label)
                .enabled(state == InstallState::Available)
                .tooltip(tooltip)
                .clicked();
        });
    install
}

fn byline(found: &IndexMod) -> String {
    let version = match found.version.strip_prefix('v') {
        Some(_) => found.version.clone(),
        None => format!("v{}", found.version),
    };
    let mut parts = vec![version];
    match found.developers.as_slice() {
        [] => {}
        [only] => parts.push(only.clone()),
        [first, rest @ ..] => parts.push(format!("{first} +{}", rest.len())),
    }
    parts.push(format!("{} downloads", short_count(found.downloads)));
    parts.join(" · ")
}

fn short_count(count: u64) -> String {
    match count {
        0..1_000 => count.to_string(),
        1_000..1_000_000 => format!("{:.1}K", count as f64 / 1e3),
        _ => format!("{:.1}M", count as f64 / 1e6),
    }
}
