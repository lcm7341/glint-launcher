use std::sync::Arc;

use crate::depot::{DepotProcess, DownloadRequest};
use crate::gd::InstanceName;

use super::{Launcher, Notice};
use crate::download::{Download, Outcome};

impl Launcher {
    pub(crate) fn download(&self) -> Option<&Download> {
        self.download.as_ref()
    }

    pub(super) fn poll_download(&mut self) {
        let Some(download) = &mut self.download else {
            return;
        };
        let mut lines = Vec::new();
        let outcome = download.poll(&mut lines);
        let name = download.instance().clone();
        let signed_in_as = download.signed_in_as().map(str::to_owned);
        let was_qr = download.signs_in_with_qr();
        for line in lines {
            self.push_log(line);
        }
        if let Some(account) = signed_in_as
            && self.settings.steam_username.as_deref() != Some(account.as_str())
        {
            self.settings.steam_username = Some(account);
            self.save_settings();
        }
        let Some(outcome) = outcome else {
            return;
        };
        self.download = None;
        let notice = match outcome {
            Outcome::Finished { downloaded_bytes } => {
                let finished = match self.find(&name) {
                    Some(Ok(instance)) => instance.mark_download_finished(),
                    _ => Ok(()),
                };
                let size = downloaded_bytes
                    .map(|bytes| format!(" ({} MB)", bytes / 1_000_000))
                    .unwrap_or_default();
                match finished {
                    Ok(()) => Notice::done(format!("Downloaded \"{name}\"{size}")),
                    Err(error) => Notice::failed(format!(
                        "Downloaded \"{name}\" but couldn't mark it complete: {error}"
                    )),
                }
            }
            Outcome::NeedsSignIn if !was_qr => {
                self.settings.steam_username = None;
                self.save_settings();
                self.push_log("Sign-in expired, showing QR code".to_owned());
                let notice = self.start_download(&name);
                self.notices.extend(notice);
                return;
            }
            Outcome::NeedsSignIn => Notice::failed("Steam asked for a password, try again"),
            Outcome::Failed(reason) => {
                Notice::failed(format!("Download of \"{name}\" failed: {reason}"))
            }
        };
        self.notices.push(notice);
        self.refresh();
    }

    pub(super) fn start_download(&mut self, name: &InstanceName) -> Option<Notice> {
        if self.download.is_some() {
            return Some(Notice::failed("Already downloading"));
        }
        let Some(program) = self.depot_downloader.clone() else {
            return self.fetch_depot_for(name);
        };
        let Some(Ok(instance)) = self.find(name) else {
            return Some(Notice::failed(format!(
                "There is no instance called \"{name}\""
            )));
        };
        let config = instance.config();
        let Some(entry) = self.catalog.find(&config.version, config.platform) else {
            return Some(Notice::failed(format!(
                "No download for {} on {}",
                config.version,
                config.platform.display_name()
            )));
        };
        if let Err(error) = instance.mark_download_started() {
            return Some(Notice::failed(format!(
                "Couldn't prepare {}: {error}",
                instance.game_dir().display()
            )));
        }
        let account = self.settings.steam_username.clone();
        let request =
            DownloadRequest::new(entry.clone(), instance.game_dir()).with_remember_password(true);
        let request = match &account {
            Some(account) => request.with_username(account.clone()),
            None => request.with_qr(true),
        };
        let notify = Arc::clone(&self.notify);
        match DepotProcess::start(&program, &request, move || notify()) {
            Ok(process) => {
                let how = match &account {
                    Some(account) => format!("as {account}"),
                    None => "signing in with a QR code".to_owned(),
                };
                self.push_log(format!(
                    "Downloading {} {} into \"{name}\" {how}",
                    entry.version,
                    entry.platform.display_name()
                ));
                self.download = Some(Download::new(name.clone(), process, account.is_none()));
                None
            }
            Err(error) => Some(Notice::failed(format!(
                "Couldn't start DepotDownloader at {}: {error}",
                program.display()
            ))),
        }
    }
}
