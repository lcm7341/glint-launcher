use super::{ModContext, ModManager, ModsState};
use crate::app::{Activity, App, Notice};
use crate::gd::GdPlatform;
use crate::instances::{Instance, InstanceName};
use crate::mods::{
    ECHO_21_GD, ECHO_ID, EchoRelease, EchoReleases, echo_version, fetch_echo_releases,
};
use crate::task::{Job, Notify};

#[derive(Default)]
pub(super) enum EchoFetch {
    #[default]
    Idle,
    Loading(Job<Result<EchoReleases, String>>),
    Ready(EchoReleases),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EchoState {
    Available,
    Update,
    Installed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EchoOffer {
    pub(crate) release: EchoRelease,
    pub(crate) is_gd21: bool,
    pub(crate) state: EchoState,
}

impl ModManager {
    pub(in crate::app) fn fetch_echo(&mut self, notify: &Notify) {
        self.echo = EchoFetch::Loading(Job::spawn(notify, fetch_echo_releases));
    }

    pub(in crate::app) fn poll_echo(&mut self, activity: &mut Activity) {
        let EchoFetch::Loading(job) = &self.echo else {
            return;
        };
        let Some(fetched) = job.take() else {
            return;
        };
        self.echo = match fetched {
            Ok(releases) => EchoFetch::Ready(releases),
            Err(reason) => {
                activity.push_log(format!("Couldn't check echogd.com for Echo: {reason}"));
                EchoFetch::Idle
            }
        };
    }

    fn echo_releases(&self) -> Option<&EchoReleases> {
        match &self.echo {
            EchoFetch::Ready(releases) => Some(releases),
            _ => None,
        }
    }
}

impl App {
    pub(crate) fn echo_offer(&self, instance: &Instance) -> Option<EchoOffer> {
        let releases = self.mods.echo_releases()?;
        let config = instance.config();
        if let ModsState::Geode { mods: Ok(mods) } = self.mods_state(instance) {
            let mut release = releases.for_geode(&config.version)?.clone();
            let installed = mods.iter().find(|found| found.id == ECHO_ID);
            if release.description.is_none() {
                release.description = installed.and_then(|found| found.info.description.clone());
            }
            let state = match installed {
                None => EchoState::Available,
                Some(found) if found.version.trim_start_matches('v') == release.version => {
                    EchoState::Installed
                }
                Some(_) => EchoState::Update,
            };
            return Some(EchoOffer {
                release,
                is_gd21: false,
                state,
            });
        }
        let is_gd21 =
            config.platform == GdPlatform::Windows && config.version.to_string() == ECHO_21_GD;
        if !is_gd21 {
            return None;
        }
        let release = releases.for_gd21()?.clone();
        let newest = echo_version(&format!("Echo v{}", release.version));
        let installed = self
            .echo(instance)
            .and_then(|echo| echo_version(&echo.dll.name));
        let state = match installed {
            None => EchoState::Available,
            Some(version) if Some(&version) >= newest.as_ref() => EchoState::Installed,
            Some(_) => EchoState::Update,
        };
        Some(EchoOffer {
            release,
            is_gd21: true,
            state,
        })
    }

    pub(in crate::app) fn start_echo_install(&mut self, name: &InstanceName) -> Option<Notice> {
        let Some(Ok(instance)) = self.library.find(name) else {
            return Some(Notice::failed(format!("There's no instance \"{name}\"")));
        };
        let Some(offer) = self.echo_offer(instance) else {
            return Some(Notice::failed("echogd.com has no Echo for this version"));
        };
        let context = ModContext {
            game_open: self.may_be_game_open(instance),
            notify: &self.notify,
        };
        self.mods
            .start_echo_install(instance, offer, context, &mut self.activity)
    }
}
