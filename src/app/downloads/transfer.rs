use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::depot::{CodeKind, DepotEvent, DepotProcess, DepotUpdate, DownloadRequest};
use crate::gd::GdVersion;
use crate::instances::InstanceName;

use crate::task::Notify;

use super::Seeding;

pub(crate) struct Download {
    instance: InstanceName,
    version: GdVersion,
    work: Work,
    stage: Stage,
    fraction: Option<f32>,
    file: Option<String>,
    downloaded_bytes: Option<u64>,
    failure: Option<String>,
    signs_in_with_qr: bool,
    account: Option<String>,
    password: Option<String>,
    wrong_code: bool,
    switch_to_qr: bool,
    qr_code: Vec<Vec<bool>>,
    signed_in_as: Option<String>,
    logged_tenths: u8,
}

enum Work {
    Seeding { seeding: Seeding, then: DepotStart },
    Depot(DepotProcess),
}

pub(crate) struct DepotStart {
    pub(crate) program: PathBuf,
    pub(crate) request: DownloadRequest,
    pub(crate) notify: Notify,
}

pub(crate) enum SignIn {
    Qr,
    Saved(String),
    Password { username: String, password: String },
}

#[derive(Clone, PartialEq, Eq)]
enum Stage {
    SigningIn,
    ScanQrCode,
    ApproveInMobileApp,
    EnterPassword,
    EnterCode { kind: CodeKind, wrong: bool },
    Preparing,
    Downloading,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Outcome {
    Finished { downloaded_bytes: Option<u64> },
    SwitchToQr,
    Failed(String),
}

pub(crate) enum SteamPrompt<'a> {
    Password { account: Option<&'a str> },
    Code { kind: &'a CodeKind, wrong: bool },
}

impl Download {
    pub(crate) fn new(
        instance: InstanceName,
        version: GdVersion,
        process: DepotProcess,
        sign_in: SignIn,
    ) -> Self {
        Self::with_work(instance, version, Work::Depot(process), sign_in)
    }

    pub(crate) fn after_seeding(
        instance: InstanceName,
        version: GdVersion,
        seeding: Seeding,
        then: DepotStart,
        account: String,
    ) -> Self {
        let work = Work::Seeding { seeding, then };
        Self::with_work(instance, version, work, SignIn::Saved(account))
    }

    fn with_work(instance: InstanceName, version: GdVersion, work: Work, sign_in: SignIn) -> Self {
        let (account, password) = match sign_in {
            SignIn::Qr => (None, None),
            SignIn::Saved(account) => (Some(account), None),
            SignIn::Password { username, password } => (Some(username), Some(password)),
        };
        Self {
            instance,
            version,
            work,
            signs_in_with_qr: account.is_none(),
            account,
            password,
            wrong_code: false,
            switch_to_qr: false,
            stage: Stage::SigningIn,
            fraction: None,
            file: None,
            downloaded_bytes: None,
            failure: None,
            qr_code: Vec::new(),
            signed_in_as: None,
            logged_tenths: 0,
        }
    }

    pub(crate) fn instance(&self) -> &InstanceName {
        &self.instance
    }

    pub(crate) fn version(&self) -> &GdVersion {
        &self.version
    }

    pub(crate) fn fraction(&self) -> Option<f32> {
        self.fraction
    }

    pub(crate) fn detail(&self) -> String {
        if let Work::Seeding { seeding, .. } = &self.work {
            return seeding.detail();
        }
        match &self.stage {
            Stage::SigningIn => "Signing in to Steam".to_owned(),
            Stage::ScanQrCode => "Scan the QR code".to_owned(),
            Stage::ApproveInMobileApp => "Approve in the Steam app".to_owned(),
            Stage::EnterPassword => "Enter your Steam password".to_owned(),
            Stage::EnterCode { .. } => "Enter the Steam Guard code".to_owned(),
            Stage::Preparing => "Preparing the download".to_owned(),
            Stage::Downloading => match (&self.fraction, &self.file) {
                (Some(fraction), Some(file)) => format!("{:.0}%  {file}", fraction * 100.0),
                _ => "Downloading".to_owned(),
            },
        }
    }

    pub(crate) fn poll(&mut self, log: &mut Vec<String>) -> Option<Outcome> {
        if let Work::Seeding { seeding, then } = &self.work {
            let result = seeding.take_result()?;
            if seeding.is_cancelled() {
                return Some(Outcome::Failed("cancelled while copying files".to_owned()));
            }
            log.push(match result {
                Ok(bytes) => format!("Copied {} MB from {}", bytes / 1_000_000, seeding.from()),
                Err(reason) => format!("Couldn't copy from {}: {reason}", seeding.from()),
            });
            let notify = Arc::clone(&then.notify);
            match DepotProcess::start(&then.program, &then.request, move || notify()) {
                Ok(process) => self.work = Work::Depot(process),
                Err(error) => {
                    return Some(Outcome::Failed(format!(
                        "couldn't start DepotDownloader at {}: {error}",
                        then.program.display()
                    )));
                }
            }
        }
        let Work::Depot(process) = &self.work else {
            return None;
        };
        for update in process.take_updates() {
            match update {
                DepotUpdate::Event(event) => {
                    log.extend(self.describe(&event));
                    self.apply(event);
                }
                DepotUpdate::Exited { success } => return Some(self.outcome(success)),
            }
        }
        None
    }

    pub(crate) fn qr_code(&self) -> Option<&[Vec<bool>]> {
        (self.stage == Stage::ScanQrCode && !self.qr_code.is_empty()).then_some(&self.qr_code)
    }

    pub(crate) fn prompt(&self) -> Option<SteamPrompt<'_>> {
        match &self.stage {
            Stage::EnterPassword => Some(SteamPrompt::Password {
                account: self.account.as_deref(),
            }),
            Stage::EnterCode { kind, wrong } => Some(SteamPrompt::Code {
                kind,
                wrong: *wrong,
            }),
            _ => None,
        }
    }

    pub(crate) fn approving(&self) -> bool {
        self.stage == Stage::ApproveInMobileApp
    }

    pub(crate) fn answer(&mut self, text: &str) {
        if matches!(self.stage, Stage::EnterPassword | Stage::EnterCode { .. }) {
            self.stage = Stage::SigningIn;
            self.send(text);
        }
    }

    pub(crate) fn switch_to_qr(&mut self) {
        self.switch_to_qr = true;
        self.cancel();
    }

    fn send(&mut self, text: &str) {
        let Work::Depot(process) = &self.work else {
            return;
        };
        if let Err(error) = process.answer(text) {
            self.failure = Some(format!("couldn't answer DepotDownloader: {error}"));
            process.cancel();
        }
    }

    pub(crate) fn signs_in_with_qr(&self) -> bool {
        self.signs_in_with_qr
    }

    pub(crate) fn signed_in_as(&self) -> Option<&str> {
        self.signed_in_as.as_deref()
    }

    pub(crate) fn cancel(&self) {
        match &self.work {
            Work::Seeding { seeding, .. } => seeding.cancel(),
            Work::Depot(process) => process.cancel(),
        }
    }

    fn describe(&mut self, event: &DepotEvent) -> Option<String> {
        let line = match event {
            DepotEvent::PasswordRequested => "Steam asked for a password".to_owned(),
            DepotEvent::CodeRequested(CodeKind::App) => {
                "Steam asked for a Steam Guard code".to_owned()
            }
            DepotEvent::CodeRequested(CodeKind::Email(_)) => {
                "Steam emailed a Steam Guard code".to_owned()
            }
            DepotEvent::WrongCode => "Steam rejected the code".to_owned(),
            DepotEvent::SignInFailed(reason) => format!("Sign-in failed: {reason}"),
            DepotEvent::ApproveInMobileApp => "Waiting for approval in the Steam app".to_owned(),
            DepotEvent::SignedIn => "Signed in to Steam".to_owned(),
            DepotEvent::QrCodeFollows => "Showing QR code".to_owned(),
            DepotEvent::QrCodeRow(_) => return None,
            DepotEvent::SignedInAs(_) => "Signed in to Steam".to_owned(),
            DepotEvent::ManifestFound { manifest, date } => {
                format!("Found manifest {manifest} from {date}")
            }
            DepotEvent::Progress { percent, .. } => {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "percent is 0 to 100"
                )]
                let tenths = (percent / 10.0) as u8;
                if tenths <= self.logged_tenths {
                    return None;
                }
                self.logged_tenths = tenths;
                format!("Downloaded {}%", u16::from(tenths) * 10)
            }
            DepotEvent::Finished {
                bytes,
                uncompressed_bytes,
            } => {
                format!("Done: {bytes} bytes ({uncompressed_bytes} uncompressed)")
            }
            DepotEvent::Retrying(text) | DepotEvent::Failed(text) | DepotEvent::Other(text) => {
                text.clone()
            }
        };
        Some(line)
    }

    fn apply(&mut self, event: DepotEvent) {
        match event {
            DepotEvent::PasswordRequested => match self.password.take() {
                Some(password) => self.send(&password),
                None => self.stage = Stage::EnterPassword,
            },
            DepotEvent::CodeRequested(kind) => {
                let wrong = std::mem::take(&mut self.wrong_code);
                self.stage = Stage::EnterCode { kind, wrong };
            }
            DepotEvent::WrongCode => self.wrong_code = true,
            DepotEvent::SignInFailed(reason) => {
                self.stage = Stage::SigningIn;
                self.failure = Some(sign_in_failure(&reason));
            }
            DepotEvent::QrCodeFollows => {
                self.stage = Stage::ScanQrCode;
                self.qr_code.clear();
            }
            DepotEvent::QrCodeRow(row) => self.qr_code.push(row),
            DepotEvent::SignedInAs(username) => {
                self.signed_in_as = Some(username);
                self.stage = Stage::SigningIn;
                self.qr_code.clear();
            }
            DepotEvent::ApproveInMobileApp => self.stage = Stage::ApproveInMobileApp,
            DepotEvent::SignedIn => {
                if self.signed_in_as.is_none() {
                    self.signed_in_as.clone_from(&self.account);
                }
                self.stage = Stage::Preparing;
            }
            DepotEvent::ManifestFound { .. } => self.stage = Stage::Preparing,
            DepotEvent::Progress { percent, file } => {
                self.stage = Stage::Downloading;
                self.fraction = Some(percent / 100.0);
                self.file = Path::new(&file)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned());
            }
            DepotEvent::Finished { bytes, .. } => self.downloaded_bytes = Some(bytes),
            DepotEvent::Failed(reason) => {
                if reason.starts_with("Unable to download") {
                    self.failure = Some(reason);
                } else {
                    self.failure.get_or_insert(reason);
                }
            }
            DepotEvent::Retrying(_) | DepotEvent::Other(_) => {}
        }
    }

    fn outcome(&self, success: bool) -> Outcome {
        if self.switch_to_qr {
            Outcome::SwitchToQr
        } else if success && self.failure.is_none() {
            Outcome::Finished {
                downloaded_bytes: self.downloaded_bytes,
            }
        } else {
            Outcome::Failed(
                self.failure
                    .clone()
                    .unwrap_or_else(|| "DepotDownloader quit before finishing".to_owned()),
            )
        }
    }
}

fn sign_in_failure(reason: &str) -> String {
    if reason.contains("InvalidPassword") {
        "Steam rejected the username or password".to_owned()
    } else if reason.contains("RateLimitExceeded") {
        "Steam blocked sign-ins for a while after too many tries".to_owned()
    } else {
        format!("Steam sign-in failed: {reason}")
    }
}
