use super::windows_save::usual_save_dir;
use super::{Activity, Notice};
use crate::gd::is_encrypted_mac_build;
use crate::gd::{GD_APP_ID, GdPlatform, GdVersion};
use crate::instances::Instance;
use crate::instances::InstanceName;
use crate::launch::{
    GameProcess, LaunchRequest, PROTON_NAME, WineStatus, find_proton, find_runtime, has_steam_drm,
    is_running_in, wine_steam_running,
};
use crate::launch::{LaunchBlocker, LaunchFacts, LaunchReadiness};
use crate::steam::{
    SteamShortcut, shortcut_compat_dir, steam_app_install, steam_compat_tool, steam_shortcuts,
};
use crate::task::Notify;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

mod coordinator;
mod crash_log;
use crate::format::home_relative;
use crash_log::new_crash_log;
mod running;
use running::RunningGames;

struct Game {
    instance: InstanceName,
    process: GameProcess,
    started: Instant,
    started_at: SystemTime,
    game_dir: PathBuf,
    is_save_swapped: bool,
    steam: SteamCheck,
}

/// Which Steam the game needs, checked if it closes right away
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SteamCheck {
    Host,
    Wine,
}

impl SteamCheck {
    /// Why the game closed right away, or None if steam looks fine
    fn quick_exit_reason(self) -> Option<&'static str> {
        match self {
            // closing the game yourself is quick too, so only blame steam if it's really gone
            Self::Host => match crate::platform::steam_running() {
                Ok(true) => None,
                Ok(false) => Some("Steam isn't running"),
                Err(_) => Some("Is Steam running?"),
            },
            // under wine it's usually a signed out steam, which stays running
            Self::Wine if wine_steam_running() => Some("Is Steam signed in?"),
            Self::Wine => Some("Steam for Windows isn't running"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SaveMode {
    Own,
    Linked,
}

const PROTON_SAVE: &str = "pfx/drive_c/users/steamuser/AppData/Local/GeometryDash";

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

struct LaunchEnvironment {
    steam_folder: Option<PathBuf>,
    proton: Option<PathBuf>,
    runtime: Option<PathBuf>,
    shortcuts: Vec<SteamShortcut>,
}

pub(super) struct PlaySession {
    environment: LaunchEnvironment,
    game: Option<Game>,
    running: RunningGames,
}

impl PlaySession {
    pub(super) fn new(notify: &Notify) -> Self {
        let steam_folder = crate::steam::steam_folder();
        Self {
            environment: LaunchEnvironment {
                proton: steam_folder.as_deref().and_then(find_proton),
                runtime: steam_folder.as_deref().and_then(find_runtime),
                shortcuts: steam_folder
                    .as_deref()
                    .map(steam_shortcuts)
                    .unwrap_or_default(),
                steam_folder,
            },
            game: None,
            running: RunningGames::watch(notify),
        }
    }
    pub(super) fn steam_install(&self) -> Option<PathBuf> {
        let steam_folder = self.environment.steam_folder.as_ref()?;
        let install = steam_app_install(steam_folder, GD_APP_ID)
            .unwrap_or_else(|| steam_folder.join("steamapps/common/Geometry Dash"));
        install.is_dir().then_some(install)
    }
    pub(super) fn proton_request(
        &self,
        instance: &Instance,
        mode: SaveMode,
    ) -> Option<LaunchRequest> {
        let steam_folder = self.environment.steam_folder.as_ref()?;
        let save_source = self.save_source(instance, mode);
        // a prefix shared with steam keeps the proton steam runs it with
        let steams_tool = match &save_source {
            SaveSource::Steam(compat) | SaveSource::Shortcut(compat) => compat
                .file_name()
                .and_then(|app_id| app_id.to_str()?.parse().ok())
                .and_then(|app_id| steam_compat_tool(steam_folder, app_id)),
            SaveSource::Own => None,
        };
        let (proton, runtime) = match steams_tool {
            Some(tool) => (tool.proton, tool.runtime),
            None => (
                self.environment.proton.clone()?,
                self.environment.runtime.clone(),
            ),
        };
        Some(
            LaunchRequest::new(
                proton,
                steam_folder,
                instance.game_dir(),
                save_source.compat_dir(instance),
            )
            .with_runtime(runtime),
        )
    }
    pub(super) fn started(
        &mut self,
        instance: &Instance,
        process: GameProcess,
        is_save_swapped: bool,
        steam: SteamCheck,
    ) {
        self.game = Some(Game {
            instance: instance.name().clone(),
            process,
            started: Instant::now(),
            started_at: SystemTime::now(),
            game_dir: instance.game_dir(),
            is_save_swapped,
            steam,
        });
    }
    fn steam_compat(&self) -> Option<PathBuf> {
        let compat = self
            .environment
            .steam_folder
            .as_ref()?
            .join("steamapps")
            .join("compatdata")
            .join(GD_APP_ID.to_string());
        compat.is_dir().then_some(compat)
    }

    pub(super) fn main_save_dir(&self, instance: &Instance) -> Option<PathBuf> {
        if self.is_windows_on_mac(instance) {
            return None;
        }
        if self.is_native(instance) {
            return match instance.config().platform {
                GdPlatform::MacOs => crate::platform::native_save_dir(None),
                GdPlatform::Windows => usual_save_dir(),
            };
        }
        Some(self.steam_compat()?.join(PROTON_SAVE))
    }

    pub(super) fn save_source(&self, instance: &Instance, mode: SaveMode) -> SaveSource {
        let Some(steam_folder) = &self.environment.steam_folder else {
            return SaveSource::Own;
        };
        if mode == SaveMode::Linked
            && let Some(compat) = self.steam_compat()
        {
            return SaveSource::Steam(compat);
        }
        // an imported folder with a steam shortcut keeps that shortcut's save
        instance
            .config()
            .game_folder
            .as_ref()
            .and_then(|folder| {
                shortcut_compat_dir(steam_folder, &self.environment.shortcuts, folder)
            })
            .map_or(SaveSource::Own, SaveSource::Shortcut)
    }

    pub(super) fn gd_save_dir(&self, instance: &Instance, mode: SaveMode) -> PathBuf {
        if self.is_windows_on_mac(instance) {
            return instance.save_dir();
        }
        if self.is_native(instance) {
            let shares_main = mode == SaveMode::Linked;
            return match instance.config().platform {
                GdPlatform::MacOs => {
                    let home = (!shares_main).then(|| instance.mac_home_dir());
                    crate::platform::native_save_dir(home.as_deref()).unwrap_or_default()
                }
                GdPlatform::Windows => {
                    let is_swapped_in = self.game.as_ref().is_some_and(|game| {
                        game.is_save_swapped && game.instance == *instance.name()
                    });
                    if shares_main || is_swapped_in {
                        usual_save_dir().unwrap_or_default()
                    } else {
                        instance.save_dir()
                    }
                }
            };
        }
        self.save_source(instance, mode)
            .compat_dir(instance)
            .join(PROTON_SAVE)
    }

    pub(super) fn describe_save(&self, instance: &Instance, mode: SaveMode) -> &'static str {
        if self.is_windows_on_mac(instance) {
            return "Its own";
        }
        match mode {
            SaveMode::Linked if self.is_native(instance) => "Your main save",
            SaveMode::Own if self.is_native(instance) => "Its own",
            SaveMode::Own | SaveMode::Linked => self.save_source(instance, mode).describe(),
        }
    }

    pub(super) fn native_platform(&self) -> Option<GdPlatform> {
        crate::platform::HostOs::CURRENT.native_game_platform()
    }

    pub(super) fn is_native(&self, instance: &Instance) -> bool {
        crate::platform::HostOs::CURRENT.native_game_platform() == Some(instance.config().platform)
    }

    pub(super) fn is_windows_on_mac(&self, instance: &Instance) -> bool {
        crate::platform::HostOs::CURRENT.native_game_platform() == Some(GdPlatform::MacOs)
            && instance.config().platform == GdPlatform::Windows
    }

    pub(super) fn is_game_open(&self, instance: &Instance) -> bool {
        if self.playing() == Some(instance.name()) {
            return true;
        }
        let running = self.running.game_dirs();
        !running.is_empty() && is_running_in(&running, &instance.game_dir())
    }

    pub(super) fn launch_readiness(
        &self,
        instance: &Instance,
        wine: WineStatus,
    ) -> LaunchReadiness {
        let config = instance.config();
        let host = crate::platform::HostOs::CURRENT;
        LaunchFacts {
            host,
            build: config.platform,
            encrypted_mac_build: is_encrypted_mac_build(&config.version),
            steam_drm: host.native_game_platform() != Some(config.platform)
                && config.platform == GdPlatform::Windows
                && has_steam_drm(&instance.game_dir().join(crate::gd::WINDOWS_PROGRAM)),
            steam_available: self.environment.steam_folder.is_some(),
            proton_available: self.environment.proton.is_some(),
            wine,
        }
        .readiness()
    }

    pub(super) fn playing(&self) -> Option<&InstanceName> {
        self.game.as_ref().map(|game| &game.instance)
    }

    pub(super) fn poll(&mut self, activity: &mut Activity) -> bool {
        let Some(game) = &self.game else {
            return false;
        };
        if !game.process.has_exited() {
            return false;
        }
        let quick = game.started.elapsed() < Duration::from_secs(8);
        let name = game.instance.clone();
        let is_save_swapped = game.is_save_swapped;
        let steam = game.steam;
        let crash_log = new_crash_log(&game.game_dir, game.started_at);
        self.game = None;
        if let Some(log) = crash_log {
            activity.notice(Notice::failed(format!(
                "{name} crashed. Its crash log is {}",
                home_relative(&log)
            )));
            return is_save_swapped;
        }
        match quick.then(|| steam.quick_exit_reason()).flatten() {
            Some(reason) => {
                activity.notice(Notice::failed(format!(
                    "{name} closed within seconds. {reason}"
                )));
            }
            None => activity.push_log(format!("\"{name}\" closed")),
        }
        is_save_swapped
    }
}
fn own_mac_home(instance: &Instance, mode: SaveMode) -> Option<PathBuf> {
    match mode {
        SaveMode::Own => Some(instance.mac_home_dir()),
        SaveMode::Linked => None,
    }
}

pub(crate) fn launch_blocker_message(blocker: LaunchBlocker, version: &GdVersion) -> String {
    match blocker {
        LaunchBlocker::EncryptedMacBuild => {
            format!("Pre-2.2 mac builds are encrypted. Use a Windows instance of {version}")
        }
        LaunchBlocker::MacBuildOnOtherHost => "Mac builds need a Mac".to_owned(),
        LaunchBlocker::SteamMissing => "Steam not found".to_owned(),
        LaunchBlocker::ProtonMissing => format!("Install {PROTON_NAME} in Steam first"),
        LaunchBlocker::SteamDrmWithProton => {
            format!("{version} has Steam DRM, so it runs from Steam's install")
        }
    }
}
