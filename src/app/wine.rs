use std::io;
use std::sync::{Arc, Mutex, PoisonError};

use crate::instances::Instance;
use crate::launch::{
    GameProcess, INSTALL_ROSETTA, SetupStep, WineStatus, WineSteam, glint_wine, needs_rosetta,
    setup_wine, wine_steam_running,
};
use crate::paths::DataDir;

use super::{Activity, Notice};
use crate::task::Job;
use crate::task::Notify;

pub(super) struct WineSupport {
    data_dir: Option<DataDir>,
    notify: Notify,
    setup: Option<WineSetup>,
}

struct WineSetup {
    step: Arc<Mutex<SetupStep>>,
    job: Job<Result<(), String>>,
}

impl WineSupport {
    pub(super) fn new(data_dir: Option<DataDir>, notify: Notify) -> Self {
        Self {
            data_dir,
            notify,
            setup: None,
        }
    }
    fn tools_dir(&self) -> Option<std::path::PathBuf> {
        Some(self.data_dir.as_ref()?.tools_dir())
    }

    pub(super) fn steam(&self) -> Option<WineSteam> {
        let wine = glint_wine(&self.tools_dir()?)?;
        let prefix = self.data_dir.as_ref()?.wine_dir();
        Some(WineSteam::new(wine, prefix))
    }

    pub(super) fn status(&self) -> WineStatus {
        if let Some(setup) = &self.setup {
            return WineStatus::SettingUp(
                *setup.step.lock().unwrap_or_else(PoisonError::into_inner),
            );
        }
        match self.steam() {
            Some(wine_steam) if wine_steam.is_steam_installed() => WineStatus::Ready,
            _ => WineStatus::NotSetUp,
        }
    }

    pub(super) fn start_setup(&mut self, activity: &mut Activity) -> Option<Notice> {
        if self.setup.is_some() {
            return Some(Notice::failed("Already setting up"));
        }
        if needs_rosetta() {
            return Some(rosetta_notice());
        }
        let (Some(tools_dir), Some(data_dir)) = (self.tools_dir(), self.data_dir.as_ref()) else {
            return Some(Notice::failed(
                "Glint's application data directory is unavailable",
            ));
        };
        let prefix = data_dir.wine_dir();
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
        activity.push_log("Setting up Wine and Steam for Windows".to_owned());
        self.setup = Some(WineSetup { step, job });
        None
    }

    pub(super) fn start_game(
        &self,
        instance: &Instance,
        has_drm: bool,
        notify: impl Fn() + Send + 'static,
    ) -> Result<(io::Result<GameProcess>, String), Option<Notice>> {
        if needs_rosetta() {
            return Err(Some(rosetta_notice()));
        }
        let wine_steam = self.steam().ok_or(None)?;
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

    pub(super) fn poll(&mut self, activity: &mut Activity) {
        let Some(setup) = &self.setup else {
            return;
        };
        let Some(done) = setup.job.take() else {
            return;
        };
        self.setup = None;
        activity.notice(match done {
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

#[cfg(test)]
mod tests {
    use super::{WineSetup, WineSupport};
    use crate::app::Activity;
    use crate::launch::{SetupStep, WineStatus};
    use crate::test_support::completed_job;
    use std::sync::{Arc, Mutex};

    #[test]
    fn setup_completion() {
        for result in [Ok(()), Err("fixture installation failed".to_owned())] {
            let failed = result.is_err();
            let mut support = WineSupport::new(None, Arc::new(|| {}));
            support.setup = Some(WineSetup {
                step: Arc::new(Mutex::new(SetupStep::StartingSteam)),
                job: completed_job(result),
            });
            assert!(support.status() == WineStatus::SettingUp(SetupStep::StartingSteam));
            let mut activity = Activity::default();
            support.poll(&mut activity);
            support.poll(&mut activity);
            assert!(support.status() == WineStatus::NotSetUp);
            let notices = activity.take_notices();
            assert_eq!(notices.len(), 1);
            assert_eq!(notices[0].is_failure, failed);
        }
    }
}
