use crate::instances::Instance;
use crate::mods::{DllLoader, DllMod, EchoMod, GeodeMod, MegaHackLoader};
use weft::prelude::{Align, Grow, Insets, Ui};

use crate::launcher::{Action, Launcher, ModChange, ModsState};

pub(crate) fn show_mods(
    ui: &mut Ui,
    launcher: &Launcher,
    instance: &Instance,
    actions: &mut Vec<Action>,
) {
    let change = |change| Action::Mod {
        instance: instance.name().clone(),
        change,
    };
    ui.column()
        .gap(8)
        .width(Grow(1))
        .height(Grow(1))
        .show(|ui| {
            let echo = launcher.echo(instance);
            if let Some(echo) = &echo {
                show_echo(ui, echo, &change, actions);
            }
            match launcher.mods_state(instance) {
                ModsState::Geode { mods } => {
                    show_geode_mods(ui, launcher, instance, mods, actions);
                }
                ModsState::NeedsGeode => {
                    ui.label("Mods for this version need Geode.").muted();
                }
                ModsState::NoMacLoader => {
                    ui.label("No mod loader for pre-2.2 mac builds.").muted();
                }
                ModsState::NoLoader { can_install } => {
                    let text = match echo {
                        Some(_) => "Nothing loads other DLL mods here yet.",
                        None => "Nothing loads mods here yet.",
                    };
                    ui.label(text).muted();
                    if ui
                        .button("Install Glint Loader")
                        .enabled(can_install)
                        .tooltip(match can_install {
                            true => "Loads DLLs from glint-mods",
                            false => "Download the game first",
                        })
                        .clicked()
                    {
                        actions.push(change(ModChange::InstallGlintLoader));
                    }
                }
                ModsState::Dll { loader, mods } => {
                    let mega_hack = launcher.mega_hack(instance);
                    let mega_hack_on = mega_hack.as_ref().is_none_or(|found| found.is_enabled);
                    let extensions_on = launcher.extensions_enabled(instance);
                    show_loader_bar(
                        ui,
                        launcher,
                        loader,
                        mega_hack,
                        extensions_on,
                        &change,
                        actions,
                    );
                    let mods_load =
                        loader == DllLoader::GlintLoader || (mega_hack_on && extensions_on);
                    let indent = if loader == DllLoader::MegaHack {
                        2.0 * INDENT
                    } else {
                        0.0
                    };
                    ui.column()
                        .gap(8)
                        .width(Grow(1))
                        .padding(Insets::new(0.0, 0.0, 0.0, indent))
                        .show(|ui| match mods {
                            Err(error) => {
                                ui.label(format!("Couldn't read the mods: {error}")).muted();
                            }
                            Ok(mods) if mods.is_empty() => {
                                ui.label("No mods yet.").muted();
                            }
                            Ok(mods) => {
                                for found in mods {
                                    show_mod(ui, &found, mods_load, &change, actions);
                                }
                            }
                        });
                }
            }
        });
}

fn show_geode_mods(
    ui: &mut Ui,
    launcher: &Launcher,
    instance: &Instance,
    mods: Result<std::rc::Rc<[GeodeMod]>, String>,
    actions: &mut Vec<Action>,
) {
    match mods {
        Err(error) => {
            ui.label(format!("Couldn't read Geode's mods: {error}"))
                .muted();
        }
        Ok(mods) if mods.is_empty() => {
            ui.label("No Geode mods yet.").muted();
        }
        Ok(mods) => crate::geode_mods_view::show_geode_mods(ui, launcher, instance, mods, actions),
    }
}

fn show_echo(
    ui: &mut Ui,
    echo: &EchoMod,
    change: &impl Fn(ModChange) -> Action,
    actions: &mut Vec<Action>,
) {
    ui.push_id("echo", |ui| {
        let mut on = echo.dll.is_enabled;
        let toggled = ui
            .toggle(&mut on, echo.dll.name.clone())
            .style(|style| style.width(Grow(1)))
            .changed();
        if toggled {
            actions.push(change(ModChange::SetEchoEnabled {
                file_name: echo.dll.file_name.clone(),
                enabled: on,
            }));
        }
        if !echo.has_loader {
            ui.label("Echo's loader is missing.").muted().font_size(12);
        }
    });
}

const INDENT: f32 = 24.0;

fn show_loader_bar(
    ui: &mut Ui,
    launcher: &Launcher,
    loader: DllLoader,
    mega_hack: Option<MegaHackLoader>,
    extensions_on: bool,
    change: &impl Fn(ModChange) -> Action,
    actions: &mut Vec<Action>,
) {
    let mega_hack_on = mega_hack.as_ref().is_none_or(|found| found.is_enabled);
    if let Some(mega_hack) = &mega_hack {
        let mut on = mega_hack.is_enabled;
        let toggle = ui.toggle(&mut on, "Mega Hack").show();
        ui.tooltip_for(&toggle, "Off stops it loading");
        if toggle.changed() {
            actions.push(change(ModChange::SetMegaHackEnabled { enabled: on }));
        }
    }
    let indent = if loader == DllLoader::MegaHack {
        INDENT
    } else {
        0.0
    };
    ui.row()
        .gap(8)
        .align(Align::Center)
        .padding(Insets::new(0.0, 0.0, 0.0, indent))
        .show(|ui| {
            match loader {
                DllLoader::MegaHack => {
                    let mut on = extensions_on;
                    let toggle = ui
                        .toggle(&mut on, "Extensions")
                        .enabled(mega_hack_on)
                        .style(|style| style.width(Grow(1)))
                        .show();
                    ui.tooltip_for(&toggle, "Toggle all extensions");
                    if toggle.changed() {
                        actions.push(change(ModChange::SetExtensionsEnabled { enabled: on }));
                    }
                }
                DllLoader::GlintLoader => {
                    ui.label("Glint Loader").style(|style| style.width(Grow(1)));
                }
            }
            let can_add = loader == DllLoader::GlintLoader || extensions_on;
            if Launcher::can_pick_mod() {
                let add = ui
                    .button("Add mod…")
                    .enabled(can_add && !launcher.is_picking_mod())
                    .show();
                if add.clicked() {
                    actions.push(change(ModChange::Add));
                }
            }
            if ui.button("Open folder").clicked() {
                actions.push(change(ModChange::OpenFolder));
            }
            if loader == DllLoader::GlintLoader
                && ui
                    .button("Remove Glint Loader")
                    .tooltip("Keeps glint-mods")
                    .clicked()
            {
                actions.push(change(ModChange::RemoveGlintLoader));
            }
        });
}

fn show_mod(
    ui: &mut Ui,
    found: &DllMod,
    loads: bool,
    change: &impl Fn(ModChange) -> Action,
    actions: &mut Vec<Action>,
) {
    ui.push_id(&found.file_name, |ui| {
        let armed_id = ui.child_id("remove armed");
        let armed = ui.memory(armed_id, |armed: &mut bool| *armed);
        ui.row().gap(8).align(Align::Center).show(|ui| {
            let mut on = found.is_enabled;
            let toggled = ui
                .toggle(&mut on, found.name.clone())
                .enabled(loads)
                .style(|style| style.width(Grow(1)))
                .changed();
            if toggled {
                actions.push(change(ModChange::SetEnabled {
                    file_name: found.file_name.clone(),
                    enabled: on,
                }));
            }
            let label = if armed { "Delete file" } else { "Remove" };
            let remove = ui.button(label);
            let remove = if armed { remove.danger() } else { remove };
            if remove.clicked() && ui.once_per_frame(armed_id.with(&"click")) {
                if armed {
                    actions.push(change(ModChange::Remove {
                        file_name: found.file_name.clone(),
                    }));
                }
                ui.memory(armed_id, |armed: &mut bool| *armed = !*armed);
            }
        });
    });
}
