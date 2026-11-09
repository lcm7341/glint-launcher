use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use crate::depot::shortcut_compat_dir;
use crate::gd::{GD_APP_ID, GdPlatform, GdVersion, InstanceName};
use crate::instances::{Instance, same_folder};
use crate::launch::{
    GameProcess, LaunchRequest, PROTON_NAME, has_steam_drm, is_running_in, running_game_dirs,
};

use super::windows_save::usual_save_dir;
use super::windows_steam::steam_needed_notice;
use super::{Launcher, Notice, Notify, WindowsSupport};

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
    pub(super) is_save_swapped: bool,
    pub(super) quick_exit_hint: &'static str,
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
        if self.is_windows_on_mac(instance) {
            return instance.save_dir();
        }
        if self.is_native(instance) {
            return match instance.config().platform {
                GdPlatform::MacOs => own_mac_home(instance)
                    .unwrap_or_else(|| {
                        std::env::var_os("HOME")
                            .map(PathBuf::from)
                            .unwrap_or_default()
                    })
                    .join("Library/Application Support/GeometryDash"),
                GdPlatform::Windows => {
                    let is_swapped_in = self.game.as_ref().is_some_and(|game| {
                        game.is_save_swapped && game.instance == *instance.name()
                    });
                    if instance.is_linked() || is_swapped_in {
                        usual_save_dir().unwrap_or_default()
                    } else {
                        instance.save_dir()
                    }
                }
            };
        }
        self.save_source(instance)
            .compat_dir(instance)
            .join("pfx/drive_c/users/steamuser/AppData/Local/GeometryDash")
    }

    pub(crate) fn describe_save(&self, instance: &Instance) -> &'static str {
        if self.is_windows_on_mac(instance) {
            return "Its own";
        }
        if self.is_native(instance) {
            return if instance.is_linked() {
                "Shared with Steam"
            } else {
                "Its own"
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

    pub(crate) fn is_windows_on_mac(&self, instance: &Instance) -> bool {
        NATIVE_PLATFORM == Some(GdPlatform::MacOs)
            && instance.config().platform == GdPlatform::Windows
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
            let is_encrypted =
                platform == GdPlatform::MacOs && is_encrypted_mac_build(&instance.config().version);
            return is_encrypted.then(|| {
                format!(
                    "Pre-2.2 mac builds are encrypted. Use a Windows instance of {}",
                    instance.config().version
                )
            });
        }
        match (NATIVE_PLATFORM, platform) {
            (Some(GdPlatform::MacOs), GdPlatform::Windows) => {
                return self.wine_cant_play();
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

    fn wine_cant_play(&self) -> Option<String> {
        match self.windows_support() {
            WindowsSupport::SettingUp(step) => Some(format!("{}…", step.describe())),
            WindowsSupport::NotSetUp | WindowsSupport::Ready => None,
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
        let is_save_swapped = game.is_save_swapped;
        let hint = game.quick_exit_hint;
        self.game = None;
        if quick {
            self.notices
                .push(Notice::failed(format!("{name} closed right away. {hint}")));
        } else {
            self.push_log(format!("\"{name}\" closed"));
        }
        if is_save_swapped {
            self.finish_save_swap();
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
        let has_drm = instance.config().platform == GdPlatform::Windows
            && has_steam_drm(&instance.game_dir().join("GeometryDash.exe"));
        if has_drm && !self.is_windows_on_mac(instance) {
            return Some(Notice::failed(format!(
                "{} has Steam DRM and only runs from Steam's own install",
                instance.config().version
            )));
        }
        let notify = Arc::clone(&self.notify);
        let notify = move || notify();
        let mut is_save_swapped = false;
        let mut quick_exit_hint = "Is Steam running?";
        let (started, how) = if self.is_windows_on_mac(instance) {
            if self.windows_support() == WindowsSupport::NotSetUp {
                return self.start_windows_setup();
            }
            quick_exit_hint = "Is Steam signed in?";
            match self.start_on_wine(instance, has_drm, notify) {
                Ok(started) => started,
                Err(notice) => return notice,
            }
        } else if self.is_native(instance) {
            let started = match instance.config().platform {
                GdPlatform::MacOs => GameProcess::start_mac(
                    &instance.game_dir(),
                    own_mac_home(instance).as_deref(),
                    notify,
                ),
                GdPlatform::Windows => {
                    if let Some(notice) = steam_needed_notice() {
                        return Some(notice);
                    }
                    is_save_swapped = match self.swap_save_in(instance) {
                        Ok(is_swapped) => is_swapped,
                        Err(reason) => return Some(Notice::failed(reason)),
                    };
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
                    is_save_swapped,
                    quick_exit_hint,
                });
                None
            }
            Err(error) => {
                if is_save_swapped {
                    self.finish_save_swap();
                }
                Some(Notice::failed(format!("Couldn't start the game: {error}")))
            }
        }
    }
}

fn own_mac_home(instance: &Instance) -> Option<PathBuf> {
    (!instance.is_linked()).then(|| instance.mac_home_dir())
}

// drm wrapped: 1.9, 1.92, 2.113. not from 2.2074 on
pub(crate) fn is_encrypted_mac_build(version: &GdVersion) -> bool {
    *version < "2.2".parse().expect("a fixed version parses")
}
