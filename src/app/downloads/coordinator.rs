use super::{Credentials, DownloadPlan, Downloads};
use super::{Download, Outcome};
use crate::app::{Action, App, Notice};
use crate::instances::InstanceName;

impl App {
    pub(crate) fn download(&self) -> Option<&Download> {
        self.downloads.active()
    }
    pub(crate) fn can_fetch_depot(&self) -> bool {
        self.downloads.can_fetch()
    }
    pub(crate) fn fetching_depot(&self) -> bool {
        self.downloads.is_fetching()
    }
    pub(in crate::app) fn poll_depot_fetch(&mut self) {
        if let Some((name, sign_in)) = self.downloads.poll_tool(&mut self.activity) {
            let notice = self.start_download(&name, sign_in);
            self.activity.extend(notice);
        }
    }
    pub(in crate::app) fn poll_download(&mut self) {
        let Some(update) = self.downloads.poll(&mut self.activity) else {
            return;
        };
        let name = update.instance;
        let signed_in_as = update.account;
        let was_qr = update.was_qr;
        let outcome = update.outcome;
        if let Some(account) = signed_in_as
            && self.settings.steam_username.as_deref() != Some(account.as_str())
        {
            self.settings.steam_username = Some(account);
            self.save_settings();
        }
        let Some(outcome) = outcome else {
            return;
        };
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
            Outcome::SwitchToQr if !was_qr => {
                self.settings.steam_username = None;
                self.save_settings();
                self.push_log("Signing in with a QR code instead".to_owned());
                let notice = self.start_download(&name, None);
                self.activity.extend(notice);
                return;
            }
            Outcome::SwitchToQr => return,
            Outcome::Failed(reason) => {
                Notice::failed(format!("Download of \"{name}\" failed: {reason}"))
            }
        };
        self.activity.notice(notice);
        self.refresh();
    }

    pub(in crate::app) fn steer_download(&mut self, action: Action) {
        match action {
            Action::AnswerSteam(text) => self.downloads.answer(&text),
            Action::SignInWithQr => self.downloads.switch_to_qr(),
            Action::CancelDownload => self.downloads.cancel(),
            _ => {}
        }
    }

    /// a None sign-in uses the saved account, or a QR code without one
    pub(in crate::app) fn start_download(
        &mut self,
        name: &InstanceName,
        sign_in: Option<Credentials>,
    ) -> Option<Notice> {
        if self.downloads.active().is_some() {
            return Some(Notice::failed("A download is running"));
        }
        let Some(_program) = self.downloads.program() else {
            return self
                .downloads
                .fetch_tool(name, sign_in, &self.notify, &mut self.activity);
        };
        let Some(Ok(instance)) = self.library.find(name) else {
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
        let (account, password) = match sign_in {
            Some(sign_in) => (Some(sign_in.username), Some(sign_in.password)),
            None => (self.settings.steam_username.clone(), None),
        };
        let plan = DownloadPlan {
            instance,
            entry,
            seed: Downloads::seed_for(instance, self.library.entries()),
            account,
            password,
        };
        self.downloads.start(plan, &self.notify, &mut self.activity)
    }
}
