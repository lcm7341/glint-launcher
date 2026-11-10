use super::{SteamCheck, launch_blocker_message, own_mac_home};
use crate::app::windows_steam::steam_needed_notice;
use crate::app::{App, Notice};
use crate::gd::GdPlatform;
use crate::instances::Instance;
use crate::instances::InstanceName;
use crate::launch::{GameProcess, LaunchReadiness, LaunchRoute, PROTON_NAME};
use std::path::PathBuf;
use std::sync::Arc;

impl App {
    pub(crate) fn gd_save_dir(&self, instance: &Instance) -> PathBuf {
        self.play.gd_save_dir(instance, self.save_mode(instance))
    }
    pub(crate) fn describe_save(&self, instance: &Instance) -> &'static str {
        self.play.describe_save(instance, self.save_mode(instance))
    }
    pub(crate) fn native_platform(&self) -> Option<GdPlatform> {
        self.play.native_platform()
    }
    pub(crate) fn is_game_open(&self, instance: &Instance) -> bool {
        self.play.is_game_open(instance)
    }
    pub(crate) fn launch_readiness(&self, instance: &Instance) -> LaunchReadiness {
        self.play.launch_readiness(instance, self.wine.status())
    }
    pub(crate) fn playing(&self) -> Option<&InstanceName> {
        self.play.playing()
    }
    pub(crate) fn cant_play(&self, instance: &Instance) -> Option<String> {
        match self.launch_readiness(instance) {
            LaunchReadiness::Blocked(blocker) => {
                Some(launch_blocker_message(blocker, &instance.config().version))
            }
            LaunchReadiness::SettingUp(step) => Some(format!("{}…", step.describe())),
            LaunchReadiness::Ready(_) | LaunchReadiness::NeedsWineSetup => None,
        }
    }

    pub(in crate::app) fn poll_game(&mut self) {
        if self.play.poll(&mut self.activity) {
            self.finish_save_swap();
        }
    }
    pub(in crate::app) fn start_game(&mut self, name: &InstanceName) -> Option<Notice> {
        if let Some(playing) = self.play.playing() {
            return Some(Notice::failed(format!("\"{}\" is open", playing)));
        }
        let Some(Ok(instance)) = self.library.find(name) else {
            return Some(Notice::failed(format!(
                "There is no instance called \"{name}\""
            )));
        };
        let route = match self.launch_readiness(instance) {
            LaunchReadiness::Ready(route) => route,
            LaunchReadiness::NeedsWineSetup => return self.wine.start_setup(&mut self.activity),
            LaunchReadiness::SettingUp(step) => {
                return Some(Notice::done(format!("{}…", step.describe())));
            }
            LaunchReadiness::Blocked(blocker) => {
                return Some(Notice::failed(launch_blocker_message(
                    blocker,
                    &instance.config().version,
                )));
            }
        };
        let instance = instance.clone();
        if let Err(notice) = self.back_up_save_for(&instance) {
            return Some(notice);
        }
        let instance = &instance;
        let mode = self.save_mode(instance);
        let notify = Arc::clone(&self.notify);
        let notify = move || notify();
        let mut is_save_swapped = false;
        let mut steam = SteamCheck::Host;
        let (started, how) = if let LaunchRoute::Wine { through_steam } = route {
            steam = SteamCheck::Wine;
            match self.wine.start_game(instance, through_steam, notify) {
                Ok(started) => started,
                Err(notice) => return notice,
            }
        } else if route == LaunchRoute::Native {
            if let Some(notice) = steam_needed_notice() {
                return Some(notice);
            }
            let started = match instance.config().platform {
                GdPlatform::MacOs => GameProcess::start_mac(
                    &instance.game_dir(),
                    own_mac_home(instance, mode).as_deref(),
                    notify,
                ),
                GdPlatform::Windows => {
                    is_save_swapped = match self.swap_save_in(instance, mode) {
                        Ok(is_swapped) => is_swapped,
                        Err(reason) => return Some(Notice::failed(reason)),
                    };
                    GameProcess::start_windows(
                        &instance.game_dir(),
                        !instance.is_imported(),
                        notify,
                    )
                }
            };
            (started, "as a native app".to_owned())
        } else {
            // proton needs steam running
            if let Some(notice) = steam_needed_notice() {
                return Some(notice);
            }
            let request = self.play.proton_request(instance, mode)?;
            (
                GameProcess::start(&request, !instance.is_imported(), notify),
                format!("with {PROTON_NAME}"),
            )
        };
        match started {
            Ok(process) => {
                self.push_log(format!(
                    "Starting \"{name}\" (Geometry Dash {}) {how}",
                    instance.config().version
                ));
                self.play.started(instance, process, is_save_swapped, steam);
                self.note_main_save_version(instance);
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
