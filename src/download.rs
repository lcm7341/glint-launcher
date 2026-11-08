use std::path::Path;

use crate::depot::{DepotEvent, DepotProcess, DepotUpdate};
use crate::gd::InstanceName;

pub(crate) struct Download {
    instance: InstanceName,
    process: DepotProcess,
    stage: Stage,
    fraction: Option<f32>,
    file: Option<String>,
    downloaded_bytes: Option<u64>,
    failure: Option<String>,
    needs_sign_in: bool,
    signs_in_with_qr: bool,
    qr_code: Vec<Vec<bool>>,
    signed_in_as: Option<String>,
    logged_tenths: u8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    SigningIn,
    ScanQrCode,
    ApproveInMobileApp,
    Preparing,
    Downloading,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Outcome {
    Finished { downloaded_bytes: Option<u64> },
    NeedsSignIn,
    Failed(String),
}

impl Download {
    pub(crate) fn new(
        instance: InstanceName,
        process: DepotProcess,
        signs_in_with_qr: bool,
    ) -> Self {
        Self {
            instance,
            process,
            signs_in_with_qr,
            stage: Stage::SigningIn,
            fraction: None,
            file: None,
            downloaded_bytes: None,
            failure: None,
            needs_sign_in: false,
            qr_code: Vec::new(),
            signed_in_as: None,
            logged_tenths: 0,
        }
    }

    pub(crate) fn instance(&self) -> &InstanceName {
        &self.instance
    }

    pub(crate) fn fraction(&self) -> Option<f32> {
        self.fraction
    }

    pub(crate) fn detail(&self) -> String {
        match self.stage {
            Stage::SigningIn => "Signing in to Steam".to_owned(),
            Stage::ScanQrCode => "Scan the QR code".to_owned(),
            Stage::ApproveInMobileApp => "Approve in the Steam app".to_owned(),
            Stage::Preparing => "Preparing the download".to_owned(),
            Stage::Downloading => match (&self.fraction, &self.file) {
                (Some(fraction), Some(file)) => format!("{:.0}%  {file}", fraction * 100.0),
                _ => "Downloading".to_owned(),
            },
        }
    }

    pub(crate) fn poll(&mut self, log: &mut Vec<String>) -> Option<Outcome> {
        for update in self.process.take_updates() {
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

    pub(crate) fn signs_in_with_qr(&self) -> bool {
        self.signs_in_with_qr
    }

    pub(crate) fn signed_in_as(&self) -> Option<&str> {
        self.signed_in_as.as_deref()
    }

    pub(crate) fn cancel(&self) {
        self.process.cancel();
    }

    fn describe(&mut self, event: &DepotEvent) -> Option<String> {
        let line = match event {
            DepotEvent::PasswordRequested => "Steam asked for a password".to_owned(),
            DepotEvent::ApproveInMobileApp => "Waiting for approval in the Steam app".to_owned(),
            DepotEvent::SignedIn => "Signed in to Steam".to_owned(),
            DepotEvent::QrCodeFollows => "Showing QR code".to_owned(),
            DepotEvent::QrCodeRow(_) => return None,
            DepotEvent::SignedInAs(username) => format!("Signed in as {username}"),
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
            DepotEvent::PasswordRequested => self.needs_sign_in = true,
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
            DepotEvent::SignedIn | DepotEvent::ManifestFound { .. } => {
                self.stage = Stage::Preparing;
            }
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
        if success && self.failure.is_none() {
            Outcome::Finished {
                downloaded_bytes: self.downloaded_bytes,
            }
        } else if self.needs_sign_in {
            Outcome::NeedsSignIn
        } else {
            Outcome::Failed(
                self.failure
                    .clone()
                    .unwrap_or_else(|| "DepotDownloader quit early".to_owned()),
            )
        }
    }
}
