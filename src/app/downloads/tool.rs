use super::Downloads;
use crate::app::{Activity, Notice};
use crate::depot::{DEPOT_DOWNLOADER_VERSION, fetch_depot};
use crate::instances::InstanceName;
use crate::task::{Job, Notify};
use std::path::PathBuf;

pub(super) struct ToolFetch {
    instance: InstanceName,
    job: Job<Result<PathBuf, String>>,
}

impl Downloads {
    pub(super) fn can_fetch(&self) -> bool {
        self.tools_dir.is_some()
    }

    pub(super) fn is_fetching(&self) -> bool {
        self.fetching.is_some()
    }

    pub(super) fn fetch_tool(
        &mut self,
        name: &InstanceName,
        notify: &Notify,
        activity: &mut Activity,
    ) -> Option<Notice> {
        if self.fetching.is_some() {
            return Some(Notice::failed("Already getting DepotDownloader"));
        }
        let Some(tools_dir) = self.tools_dir.clone() else {
            return Some(Notice::failed("DepotDownloader not found"));
        };
        let job = Job::spawn(notify, move || {
            fetch_depot(&tools_dir).map_err(|error| error.to_string())
        });
        activity.push_log(format!(
            "Downloading DepotDownloader {DEPOT_DOWNLOADER_VERSION}"
        ));
        self.fetching = Some(ToolFetch {
            instance: name.clone(),
            job,
        });
        None
    }

    pub(super) fn poll_tool(&mut self, activity: &mut Activity) -> Option<InstanceName> {
        let fetched = self.fetching.as_ref()?.job.take()?;
        let name = self.fetching.take()?.instance;
        match fetched {
            Ok(program) => {
                activity.push_log(format!("Got DepotDownloader ({})", program.display()));
                self.program = Some(program);
                Some(name)
            }
            Err(reason) => {
                let tools = self
                    .tools_dir
                    .as_ref()
                    .map(|tools| format!(" Or put it in {} yourself", tools.display()))
                    .unwrap_or_default();
                activity.notice(Notice::failed(format!(
                    "Couldn't get DepotDownloader: {reason}.{tools}"
                )));
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ToolFetch;
    use crate::app::{Activity, downloads::Downloads};
    use crate::test_support::{TestDir, completed_job};

    #[test]
    fn fetch_completion() {
        let dir = TestDir::new();
        let mut downloads = Downloads::new(Some(dir.path().join("tools")));
        let program = dir.path().join("DepotDownloader");
        let name = "waiting".parse().unwrap();
        downloads.fetching = Some(ToolFetch {
            instance: name,
            job: completed_job(Ok(program.clone())),
        });
        let mut activity = Activity::default();
        assert_eq!(
            downloads.poll_tool(&mut activity).unwrap().as_str(),
            "waiting"
        );
        assert!(!downloads.is_fetching());
        assert_eq!(downloads.program(), Some(program.as_path()));
        assert!(downloads.poll_tool(&mut activity).is_none());
    }

    #[test]
    fn fetch_failure() {
        let dir = TestDir::new();
        let mut downloads = Downloads::new(Some(dir.path().join("tools")));
        downloads.fetching = Some(ToolFetch {
            instance: "waiting".parse().unwrap(),
            job: completed_job(Err("fixture download failed".to_owned())),
        });
        let mut activity = Activity::default();
        assert!(downloads.poll_tool(&mut activity).is_none());
        assert!(!downloads.is_fetching());
        let notices = activity.take_notices();
        assert_eq!(notices.len(), 1);
        assert!(notices[0].is_failure);
        assert!(notices[0].text.contains("fixture download failed"));
    }
}
