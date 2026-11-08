use std::path::PathBuf;

use crate::depot::{DEPOT_DOWNLOADER_VERSION, fetch_depot};
use crate::gd::InstanceName;
use crate::job::Job;

use super::{Launcher, Notice};

pub(super) struct DepotFetch {
    instance: InstanceName,
    job: Job<Result<PathBuf, String>>,
}

impl Launcher {
    pub(crate) fn can_fetch_depot(&self) -> bool {
        self.tools_dir().is_some()
    }

    pub(crate) fn fetching_depot(&self) -> bool {
        self.depot_fetch.is_some()
    }

    pub(super) fn fetch_depot_for(&mut self, name: &InstanceName) -> Option<Notice> {
        if self.depot_fetch.is_some() {
            return Some(Notice::failed("Already getting DepotDownloader"));
        }
        let Some(tools_dir) = self.tools_dir() else {
            return Some(Notice::failed("DepotDownloader not found"));
        };
        let job = Job::spawn(&self.notify, move || {
            fetch_depot(&tools_dir).map_err(|error| error.to_string())
        });
        self.push_log(format!(
            "Downloading DepotDownloader {DEPOT_DOWNLOADER_VERSION}"
        ));
        self.depot_fetch = Some(DepotFetch {
            instance: name.clone(),
            job,
        });
        None
    }

    pub(super) fn poll_depot_fetch(&mut self) {
        let Some(fetch) = &self.depot_fetch else {
            return;
        };
        let Some(fetched) = fetch.job.take() else {
            return;
        };
        let name = fetch.instance.clone();
        self.depot_fetch = None;
        match fetched {
            Ok(program) => {
                self.push_log(format!("Got DepotDownloader ({})", program.display()));
                self.depot_downloader = Some(program);
                let notice = self.start_download(&name);
                self.notices.extend(notice);
            }
            Err(reason) => {
                let tools = self
                    .tools_dir()
                    .map(|tools| format!(" Or put it in {} yourself", tools.display()))
                    .unwrap_or_default();
                self.notices.push(Notice::failed(format!(
                    "Couldn't get DepotDownloader: {reason}.{tools}"
                )));
            }
        }
    }
}
