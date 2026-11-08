use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use crate::depot::shortcut_compat_dir;
use crate::gd::{GD_APP_ID, InstanceName};
use crate::instances::{Instance, same_folder};
use crate::launch::{GameProcess, LaunchRequest, PROTON_NAME, is_running_in, running_game_dirs};

use super::{Launcher, Notice, Notify};

pub(super) struct Game {
    pub(super) instance: InstanceName,
    pub(super) process: GameProcess,
    pub(super) started: Instant,
}

pub(super) struct RunningGames {
    game_dirs: Arc<Mutex<Vec<PathBuf>>>,
}

impl RunningGames {
    pub(super) fn watch(notify: &Notify) -> Self {
        let game_dirs = Arc::new(Mutex::new(Vec::new()));
        let shared = Arc::clone(&game_dirs);
        let notify = Arc::clone(notify);
        thread::spawn(move || {
            loop {
                let found = running_game_dirs();
                let changed = {
                    let mut known = shared.lock().unwrap_or_else(PoisonError::into_inner);
                    let changed = *known != found;
                    *known = found;
                    changed
                };
                if changed {
                    notify();
                }
                thread::sleep(Duration::from_secs(2));
            }
        });
        Self { game_dirs }
    }

    fn game_dirs(&self) -> Vec<PathBuf> {
        self.game_dirs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum SaveSource {
    Steam(PathBuf),
    Shortcut(PathBuf),
    Own,
}

impl SaveSource {
    pub(crate) fn compat_dir(&self, instance: &Instance) -> PathBuf {
        match self {
            SaveSource::Steam(compat) | SaveSource::Shortcut(compat) => compat.clone(),
            SaveSource::Own => instance.proton_dir(),
        }
    }

    pub(crate) fn describe(&self) -> &'static str {
        match self {
            SaveSource::Steam(_) => "Shared with Steam",
            SaveSource::Shortcut(_) => "Shared with its Steam shortcut",
            SaveSource::Own => "Its own",
        }
    }
}

impl Launcher {
    pub(crate) fn save_source(&self, instance: &Instance) -> SaveSource {
        let (Some(steam_folder), Some(game_folder)) =
            (&self.steam_folder, &instance.config().game_folder)
        else {
            return SaveSource::Own;
        };
        let steam_compat = steam_folder
            .join("steamapps")
            .join("compatdata")
            .join(GD_APP_ID.to_string());
        let is_steams = self
            .steam_install()
            .is_some_and(|install| same_folder(&install, game_folder));
        if is_steams && steam_compat.is_dir() {
            return SaveSource::Steam(steam_compat);
        }
        match shortcut_compat_dir(steam_folder, &self.shortcuts, game_folder) {
            Some(compat) => SaveSource::Shortcut(compat),
            None => SaveSource::Own,
        }
    }

    pub(crate) fn gd_save_dir(&self, instance: &Instance) -> PathBuf {
        self.save_source(instance)
            .compat_dir(instance)
            .join("pfx/drive_c/users/steamuser/AppData/Local/GeometryDash")
    }

    pub(crate) fn is_game_open(&self, instance: &Instance) -> bool {
        if self.playing() == Some(instance.name()) {
            return true;
        }
        let running = self.running.game_dirs();
        !running.is_empty() && is_running_in(&running, &instance.game_dir())
    }

    pub(crate) fn cant_play(&self) -> Option<String> {
        match (&self.steam_folder, &self.proton) {
            (None, _) => Some("Steam not found".to_owned()),
            (Some(_), None) => Some(format!("Install {PROTON_NAME} in Steam first")),
            (Some(_), Some(_)) => None,
        }
    }

    pub(crate) fn playing(&self) -> Option<&InstanceName> {
        self.game.as_ref().map(|game| &game.instance)
    }

    pub(super) fn poll_game(&mut self) {
        let Some(game) = &self.game else {
            return;
        };
        if !game.process.has_exited() {
            return;
        }
        let quick = game.started.elapsed() < Duration::from_secs(8);
        let name = game.instance.clone();
        self.game = None;
        if quick {
            self.notices.push(Notice::failed(format!(
                "{name} closed right away. Is Steam running?"
            )));
        } else {
            self.push_log(format!("\"{name}\" closed"));
        }
    }

    pub(super) fn start_game(&mut self, name: &InstanceName) -> Option<Notice> {
        if let Some(playing) = &self.game {
            return Some(Notice::failed(format!(
                "\"{}\" is already open",
                playing.instance
            )));
        }
        let (Some(steam_folder), Some(proton)) = (&self.steam_folder, &self.proton) else {
            return self.cant_play().map(Notice::failed);
        };
        let Some(Ok(instance)) = self.find(name) else {
            return Some(Notice::failed(format!(
                "There is no instance called \"{name}\""
            )));
        };
        let request = LaunchRequest::new(
            proton,
            steam_folder,
            instance.game_dir(),
            self.save_source(instance).compat_dir(instance),
        )
        .with_runtime(self.runtime.clone());
        let notify = Arc::clone(&self.notify);
        match GameProcess::start(&request, !instance.is_linked(), move || notify()) {
            Ok(process) => {
                self.push_log(format!(
                    "Starting \"{name}\" (Geometry Dash {}) with {PROTON_NAME}",
                    instance.config().version
                ));
                self.game = Some(Game {
                    instance: name.clone(),
                    process,
                    started: Instant::now(),
                });
                None
            }
            Err(error) => Some(Notice::failed(format!("Couldn't start the game: {error}"))),
        }
    }
}
