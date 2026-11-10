use super::{Activity, Notice};
use crate::depot::{DepotProcess, DownloadRequest, find_depot};
use crate::gd::{CatalogEntry, GdVersion};
use crate::instances::InstanceName;
use crate::instances::{GameFiles, Instance, InstanceError};
use crate::task::Notify;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::sync::Arc;

mod coordinator;
mod seeding;
mod tool;
mod transfer;
use seeding::{SeedFolders, Seeding};
use tool::ToolFetch;
pub(crate) use transfer::Download;
use transfer::{DepotStart, Outcome};

pub(super) struct Downloads {
    program: Option<PathBuf>,
    tools_dir: Option<PathBuf>,
    fetching: Option<ToolFetch>,
    active: Option<Download>,
}

pub(super) struct DownloadPlan<'a> {
    pub(super) instance: &'a Instance,
    pub(super) entry: &'a CatalogEntry,
    pub(super) seed: Option<&'a Instance>,
    pub(super) account: Option<String>,
}

pub(super) struct DownloadUpdate {
    pub(super) instance: InstanceName,
    pub(super) account: Option<String>,
    pub(super) was_qr: bool,
    pub(super) outcome: Option<Outcome>,
}

impl Downloads {
    pub(super) fn new(tools_dir: Option<PathBuf>) -> Self {
        Self {
            program: tools_dir.as_deref().and_then(find_depot),
            tools_dir,
            fetching: None,
            active: None,
        }
    }
    pub(super) fn program(&self) -> Option<&Path> {
        self.program.as_deref()
    }
    pub(super) fn active(&self) -> Option<&Download> {
        self.active.as_ref()
    }
    pub(super) fn cancel(&self) {
        if let Some(download) = &self.active {
            download.cancel();
        }
    }
    pub(super) fn poll(&mut self, activity: &mut Activity) -> Option<DownloadUpdate> {
        let download = self.active.as_mut()?;
        let mut lines = Vec::new();
        let outcome = download.poll(&mut lines);
        let update = DownloadUpdate {
            instance: download.instance().clone(),
            account: download.signed_in_as().map(str::to_owned),
            was_qr: download.signs_in_with_qr(),
            outcome,
        };
        for line in lines {
            activity.push_log(line);
        }
        if update.outcome.is_some() {
            self.active = None;
        }
        Some(update)
    }
    pub(super) fn start(
        &mut self,
        plan: DownloadPlan<'_>,
        notify: &Notify,
        activity: &mut Activity,
    ) -> Option<Notice> {
        if self.active.is_some() {
            return Some(Notice::failed("A download is running"));
        }
        let instance = plan.instance;
        let entry = plan.entry;
        let name = instance.name();
        let Some(program) = self.program.clone() else {
            return Some(Notice::failed("DepotDownloader not found"));
        };
        let starts_fresh = !instance.is_imported() && instance.has_empty_game_dir();
        if let Err(error) = instance.mark_download_started() {
            return Some(Notice::failed(format!(
                "Couldn't prepare {}: {error}",
                instance.game_dir().display()
            )));
        }
        let account = plan.account;
        let request =
            DownloadRequest::new(entry.clone(), instance.game_dir()).with_remember_password(true);
        let request = match &account {
            Some(account) => request.with_username(account.clone()),
            None => request.with_qr(true),
        };
        if let Some(account) = &account
            && starts_fresh
            && let Some(seed) = plan.seed
        {
            let from = seed.config().version.clone();
            let listing_request = DownloadRequest::new(entry.clone(), instance.download_temp_dir())
                .with_username(account.clone())
                .with_remember_password(true);
            let folders = SeedFolders {
                seed: seed.game_dir(),
                target: instance.game_dir(),
            };
            let line = format!(
                "Downloading {} {} into \"{name}\" as {account}, copying from \"{}\" ({from})",
                entry.version,
                entry.platform.display_name(),
                seed.name()
            );
            let seeding = Seeding::start(
                program.clone(),
                listing_request,
                folders,
                from,
                Arc::clone(notify),
            );
            let then = DepotStart {
                program,
                request,
                notify: Arc::clone(notify),
            };
            let version = entry.version.clone();
            activity.push_log(line);
            self.active = Some(Download::after_seeding(
                name.clone(),
                version,
                seeding,
                then,
            ));
            return None;
        }
        let notify = Arc::clone(notify);
        match DepotProcess::start(&program, &request, move || notify()) {
            Ok(process) => {
                let version = entry.version.clone();
                let how = match &account {
                    Some(account) => format!("as {account}"),
                    None => "signing in with a QR code".to_owned(),
                };
                activity.push_log(format!(
                    "Downloading {} {} into \"{name}\" {how}",
                    entry.version,
                    entry.platform.display_name()
                ));
                self.active = Some(Download::new(
                    name.clone(),
                    version,
                    process,
                    account.is_none(),
                ));
                None
            }
            Err(error) => Some(Notice::failed(format!(
                "Couldn't start DepotDownloader at {}: {error}",
                program.display()
            ))),
        }
    }
    pub(super) fn seed_for<'a>(
        instance: &Instance,
        instances: &'a [Result<Instance, InstanceError>],
    ) -> Option<&'a Instance> {
        let platform = instance.config().platform;
        let target = &instance.config().version;
        instances
            .iter()
            .filter_map(|entry| entry.as_ref().ok())
            .filter(|other| {
                other.name() != instance.name()
                    && other.config().platform == platform
                    && other.game_files() == GameFiles::Present
                    && other.game_dir() != instance.game_dir()
            })
            .min_by(|a, b| closer(target, &a.config().version, &b.config().version))
    }
}
fn closer(target: &GdVersion, a: &GdVersion, b: &GdVersion) -> Ordering {
    let distance = |version: &GdVersion| (as_decimal(version) - as_decimal(target)).abs();
    distance(a).total_cmp(&distance(b)).then_with(|| b.cmp(a))
}

fn as_decimal(version: &GdVersion) -> f64 {
    version.to_string().parse().unwrap_or(0.0)
}
