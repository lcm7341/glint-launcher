use std::ffi::OsString;
use std::path::PathBuf;

use crate::gd::{CatalogEntry, GD_APP_ID};

// no password, other programs can read the command line
#[derive(Clone, PartialEq, Eq)]
pub struct DownloadRequest {
    entry: CatalogEntry,
    folder: PathBuf,
    username: Option<String>,
    remembers_password: bool,
    signs_in_with_qr: bool,
}

impl DownloadRequest {
    pub fn new(entry: CatalogEntry, folder: impl Into<PathBuf>) -> Self {
        Self {
            entry,
            folder: folder.into(),
            username: None,
            remembers_password: false,
            signs_in_with_qr: false,
        }
    }

    #[must_use]
    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    #[must_use]
    pub fn with_remember_password(mut self, remembers_password: bool) -> Self {
        self.remembers_password = remembers_password;
        self
    }

    pub fn with_qr(mut self, signs_in_with_qr: bool) -> Self {
        self.signs_in_with_qr = signs_in_with_qr;
        self
    }

    pub fn to_arguments(&self) -> Vec<OsString> {
        let mut arguments: Vec<OsString> = vec![
            "-app".into(),
            GD_APP_ID.to_string().into(),
            "-depot".into(),
            self.entry.depot_id().to_string().into(),
            "-manifest".into(),
            self.entry.manifest.to_string().into(),
            // -os defaults to the host and there's no linux build
            "-os".into(),
            self.entry.platform.as_str().into(),
            "-dir".into(),
            self.folder.clone().into(),
        ];
        if self.signs_in_with_qr {
            arguments.push("-qr".into());
        } else if let Some(username) = &self.username {
            arguments.extend(["-username".into(), username.into()]);
        }
        if self.remembers_password {
            arguments.push("-remember-password".into());
        }
        arguments
    }
}
