use std::path::PathBuf;

use super::{Activity, App, Notice};
use crate::task::{Job, Notify};
use crate::update::{self, Release, Version};

#[derive(Default)]
pub(super) enum Updater {
    #[default]
    Idle,
    Checking(Job<Result<Release, String>>),
    Available(Release),
    Installing {
        version: Version,
        job: Job<Result<PathBuf, String>>,
    },
    Restarting,
}

impl Updater {
    pub(super) fn start(notify: &Notify) -> Self {
        update::clean_up_old_files();
        if !update::checks_enabled() {
            return Self::Idle;
        }
        Self::Checking(Job::spawn(notify, || {
            update::fetch_latest().map_err(|error| error.to_string())
        }))
    }

    pub(super) fn poll(&mut self, activity: &mut Activity) {
        match self {
            Self::Checking(job) => {
                let Some(checked) = job.take() else {
                    return;
                };
                *self = Self::Idle;
                let current = update::current_version();
                match checked {
                    Ok(release) if release.version > current => {
                        activity.push_log(format!(
                            "Glint {} is out, you have {current}",
                            release.version
                        ));
                        *self = Self::Available(release);
                    }
                    Ok(_) => activity.push_log(format!("Glint {current} is up to date")),
                    // offline: log only
                    Err(reason) => {
                        activity.push_log(format!("Couldn't check for Glint updates: {reason}"));
                    }
                }
            }
            Self::Installing { version, job } => {
                let Some(installed) = job.take() else {
                    return;
                };
                let version = *version;
                *self = Self::Idle;
                let program = match installed {
                    Ok(program) => program,
                    Err(reason) => {
                        activity.notice(Notice::failed(format!("Couldn't update Glint: {reason}")));
                        return;
                    }
                };
                match update::relaunch(&program) {
                    Ok(()) => *self = Self::Restarting,
                    Err(error) => activity.notice(Notice::done(format!(
                        "Updated to Glint {version}. Reopen Glint to use it ({error})"
                    ))),
                }
            }
            Self::Idle | Self::Available(_) | Self::Restarting => {}
        }
    }
}

impl App {
    pub(crate) fn update_available(&self) -> Option<Version> {
        match &self.updater {
            Updater::Available(release) => Some(release.version),
            _ => None,
        }
    }

    pub(crate) fn installing_update(&self) -> Option<Version> {
        match &self.updater {
            Updater::Installing { version, .. } => Some(*version),
            _ => None,
        }
    }

    pub(crate) fn should_quit(&self) -> bool {
        matches!(self.updater, Updater::Restarting)
    }

    pub(crate) fn update_blocker(&self) -> Option<&'static str> {
        if self.playing().is_some() {
            Some("Close Geometry Dash first")
        } else if self.download().is_some() {
            Some("Wait for the download to finish")
        } else if self.mods.installing_geode() {
            Some("Wait for the mod install to finish")
        } else {
            None
        }
    }

    pub(super) fn start_update(&mut self) -> Option<Notice> {
        if let Some(blocker) = self.update_blocker() {
            return Some(Notice::failed(blocker));
        }
        let Updater::Available(release) = &self.updater else {
            return None;
        };
        let release = release.clone();
        let version = release.version;
        self.push_log(format!("Downloading Glint {version} from echogd.com"));
        let job = Job::spawn(&self.notify, move || {
            update::install(&release).map_err(|error| error.to_string())
        });
        self.updater = Updater::Installing { version, job };
        None
    }
}
