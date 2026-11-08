use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use crate::depot::shortcut_compat_dir;
use crate::gd::{GD_APP_ID, GdPlatform, InstanceName};
use crate::instances::{Instance, same_folder};
use crate::launch::{GameProcess, LaunchRequest, PROTON_NAME, is_running_in, running_game_dirs};

use super::{Launcher, Notice, Notify};

const NATIVE_PLATFORM: Option<GdPlatform> = if cfg!(target_os = "macos") {
    Some(GdPlatform::MacOs)
} else if cfg!(target_os = "windows") {
    Some(GdPlatform::Windows)
} else {
    None
};

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
        if self.is_native(instance) {
            return match instance.config().platform {
                GdPlatform::MacOs => own_mac_home(instance)
                    .unwrap_or_else(|| {
                        std::env::var_os("HOME")
                            .map(PathBuf::from)
                            .unwrap_or_default()
                    })
                    .join("Library/Application Support/GeometryDash"),
                GdPlatform::Windows => windows_local_app_data().join("GeometryDash"),
            };
        }
        self.save_source(instance)
            .compat_dir(instance)
            .join("pfx/drive_c/users/steamuser/AppData/Local/GeometryDash")
    }

    pub(crate) fn describe_save(&self, instance: &Instance) -> &'static str {
        if self.is_native(instance) {
            let has_own =
                instance.config().platform == GdPlatform::MacOs && own_mac_home(instance).is_some();
            return if has_own {
                "Its own"
            } else {
                "Shared with Steam"
            };
        }
        self.save_source(instance).describe()
    }

    pub(crate) fn native_platform(&self) -> Option<GdPlatform> {
        NATIVE_PLATFORM
    }

    fn is_native(&self, instance: &Instance) -> bool {
        NATIVE_PLATFORM == Some(instance.config().platform)
    }

    pub(crate) fn is_game_open(&self, instance: &Instance) -> bool {
        if self.playing() == Some(instance.name()) {
            return true;
        }
        let running = self.running.game_dirs();
        !running.is_empty() && is_running_in(&running, &instance.game_dir())
    }

    pub(crate) fn cant_play(&self, instance: &Instance) -> Option<String> {
        let platform = instance.config().platform;
        if self.is_native(instance) {
            return None;
        }
        match (NATIVE_PLATFORM, platform) {
            (Some(GdPlatform::MacOs), GdPlatform::Windows) => {
                return Some("Windows builds don't run on macOS, use the mac build".to_owned());
            }
            (_, GdPlatform::MacOs) => {
                return Some("Mac builds only run on a Mac".to_owned());
            }
            _ => {}
        }
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
        let Some(Ok(instance)) = self.find(name) else {
            return Some(Notice::failed(format!(
                "There is no instance called \"{name}\""
            )));
        };
        if let Some(reason) = self.cant_play(instance) {
            return Some(Notice::failed(reason));
        }
        let notify = Arc::clone(&self.notify);
        let notify = move || notify();
        let (started, how) = if self.is_native(instance) {
            let started = match instance.config().platform {
                GdPlatform::MacOs => GameProcess::start_mac(
                    &instance.game_dir(),
                    own_mac_home(instance).as_deref(),
                    notify,
                ),
                GdPlatform::Windows => {
                    GameProcess::start_windows(&instance.game_dir(), !instance.is_linked(), notify)
                }
            };
            (started, "directly".to_owned())
        } else {
            let (Some(steam_folder), Some(proton)) = (&self.steam_folder, &self.proton) else {
                return None;
            };
            let request = LaunchRequest::new(
                proton,
                steam_folder,
                instance.game_dir(),
                self.save_source(instance).compat_dir(instance),
            )
            .with_runtime(self.runtime.clone());
            (
                GameProcess::start(&request, !instance.is_linked(), notify),
                format!("with {PROTON_NAME}"),
            )
        };
        match started {
            Ok(process) => {
                self.push_log(format!(
                    "Starting \"{name}\" (Geometry Dash {}) {how}",
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

fn own_mac_home(instance: &Instance) -> Option<PathBuf> {
    (!instance.is_linked()).then(|| instance.mac_home_dir())
}

fn windows_local_app_data() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_default()
}
