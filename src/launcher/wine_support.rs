use std::io;
use std::sync::{Arc, Mutex, PoisonError};

use crate::instances::Instance;
use crate::launch::{
    GameProcess, INSTALL_ROSETTA, SetupStep, WineSteam, glint_wine, needs_rosetta, setup_wine,
    wine_steam_running,
};

use super::{Launcher, Notice};
use crate::job::Job;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowsSupport {
    NotSetUp,
    SettingUp(SetupStep),
    Ready,
}

#[derive(Default)]
pub(super) struct WineSupport {
    setup: Option<WindowsSetup>,
}

struct WindowsSetup {
    step: Arc<Mutex<SetupStep>>,
    job: Job<Result<(), String>>,
}

impl Launcher {
    pub(super) fn wine_steam(&self) -> Option<WineSteam> {
        let wine = glint_wine(&self.tools_dir()?)?;
        let prefix = self.store.as_ref()?.data_dir().wine_dir();
        Some(WineSteam::new(wine, prefix))
    }

    pub(crate) fn windows_support(&self) -> WindowsSupport {
        if let Some(setup) = &self.wine.setup {
            return WindowsSupport::SettingUp(
                *setup.step.lock().unwrap_or_else(PoisonError::into_inner),
            );
        }
        match self.wine_steam() {
            Some(wine_steam) if wine_steam.is_steam_installed() => WindowsSupport::Ready,
            _ => WindowsSupport::NotSetUp,
        }
    }

    pub(super) fn start_windows_setup(&mut self) -> Option<Notice> {
        if self.wine.setup.is_some() {
            return Some(Notice::failed("Already setting up"));
        }
        if needs_rosetta() {
            return Some(rosetta_notice());
        }
        let (Some(tools_dir), Some(store)) = (self.tools_dir(), &self.store) else {
            return Some(Notice::failed("HOME isn't set"));
        };
        let prefix = store.data_dir().wine_dir();
        let step = Arc::new(Mutex::new(SetupStep::MakingWineFolder));
        let shared_step = Arc::clone(&step);
        let notify = Arc::clone(&self.notify);
        let job = Job::spawn(&self.notify, move || {
            let report = |now: SetupStep| {
                *shared_step.lock().unwrap_or_else(PoisonError::into_inner) = now;
                notify();
            };
            setup_wine(&tools_dir, &prefix, &report)
        });
        self.push_log("Setting up Wine and Steam for Windows".to_owned());
        self.wine.setup = Some(WindowsSetup { step, job });
        None
    }

    pub(super) fn start_on_wine(
        &self,
        instance: &Instance,
        has_drm: bool,
        notify: impl Fn() + Send + 'static,
    ) -> Result<(io::Result<GameProcess>, String), Option<Notice>> {
        if needs_rosetta() {
            return Err(Some(rosetta_notice()));
        }
        let wine_steam = self.wine_steam().ok_or(None)?;
        if !wine_steam_running() {
            return Err(Some(match wine_steam.start_steam() {
                Ok(()) => Notice::done("Starting Steam for Windows, sign in and press Play again"),
                Err(error) => Notice::failed(format!("Couldn't start Steam for Windows: {error}")),
            }));
        }
        if !wine_steam.is_steam_signed_in() {
            return Err(Some(Notice::failed(
                "Steam for Windows isn't signed in yet",
            )));
        }
        if !has_drm {
            let started = GameProcess::start_wine(
                &wine_steam,
                &instance.game_dir(),
                &instance.save_dir(),
                !instance.is_linked(),
                notify,
            );
            return Ok((started, "with Wine".to_owned()));
        }
        if let Some(notice) = self.library_notice(&wine_steam, instance) {
            return Err(Some(notice));
        }
        let started = GameProcess::start_through_steam(
            &wine_steam,
            &instance.game_dir(),
            instance.name().as_str(),
            &instance.save_dir(),
            notify,
        );
        Ok((started, "through Steam for Windows".to_owned()))
    }

    pub(super) fn library_notice(
        &self,
        wine_steam: &WineSteam,
        instance: &Instance,
    ) -> Option<Notice> {
        let library = wine_steam.steam_library();
        if library.has_gd() {
            return None;
        }
        if library.is_installing_gd() {
            return Some(Notice::done("Steam is still installing GD"));
        }
        if let Err(error) = library.queue_gd_install() {
            return Some(Notice::failed(format!(
                "Couldn't queue the GD install: {error}"
            )));
        }
        let notify = Arc::clone(&self.notify);
        wine_steam.restart_steam(move || notify());
        Some(Notice::done(format!(
            "{} has Steam DRM, so Steam is restarting to install GD (~250 MB). Press Play again when it's done",
            instance.config().version
        )))
    }

    pub(super) fn poll_wine_setup(&mut self) {
        let Some(setup) = &self.wine.setup else {
            return;
        };
        let Some(done) = setup.job.take() else {
            return;
        };
        self.wine.setup = None;
        self.notices.push(match done {
            Ok(()) => Notice::done("Steam for Windows is starting, sign in then press Play"),
            Err(reason) => Notice::failed(reason),
        });
    }
}

fn rosetta_notice() -> Notice {
    Notice::failed(format!(
        "Wine needs Rosetta. Run {INSTALL_ROSETTA} in Terminal"
    ))
}
