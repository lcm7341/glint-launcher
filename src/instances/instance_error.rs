use std::fmt;
use std::io;
use std::path::PathBuf;

use crate::instances::InstanceName;
use crate::parse_error::ParseError;

pub enum InstanceError {
    AlreadyExists(InstanceName),
    NotFound(InstanceName),
    BadFolderName {
        folder: PathBuf,
        reason: ParseError,
    },
    BadConfig {
        name: InstanceName,
        reason: ParseError,
    },
    Io {
        action: &'static str,
        path: PathBuf,
        source: io::Error,
    },
}

impl InstanceError {
    pub(crate) fn io(action: &'static str, path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            action,
            path: path.into(),
            source,
        }
    }
}

impl fmt::Display for InstanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyExists(name) => write!(formatter, "\"{name}\" already exists"),
            Self::NotFound(name) => write!(formatter, "no instance called \"{name}\""),
            Self::BadFolderName { folder, reason } => write!(
                formatter,
                "{} isn't an instance: {reason}",
                folder.display()
            ),
            Self::BadConfig { name, reason } => {
                write!(formatter, "bad instance.txt in \"{name}\": {reason}")
            }
            Self::Io {
                action,
                path,
                source,
            } => write!(formatter, "couldn't {action} {}: {source}", path.display()),
        }
    }
}
