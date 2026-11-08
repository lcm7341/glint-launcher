use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use crate::gd::{GdPlatform, InstanceConfig, PE_HEADER_BYTES, pe_timestamp, version_for_timestamp};

pub const WINDOWS_PROGRAM: &str = "GeometryDash.exe";

pub fn recognize_install(folder: &Path) -> Result<InstanceConfig, RecognizeError> {
    let program = folder.join(WINDOWS_PROGRAM);
    let mut header = Vec::with_capacity(PE_HEADER_BYTES);
    File::open(&program)
        .and_then(|file| file.take(PE_HEADER_BYTES as u64).read_to_end(&mut header))
        .map_err(|source| match source.kind() {
            io::ErrorKind::NotFound => RecognizeError::NoProgram,
            _ => RecognizeError::Unreadable {
                program: program.clone(),
                source,
            },
        })?;
    let timestamp = pe_timestamp(&header).ok_or(RecognizeError::NotWindowsProgram)?;
    let version =
        version_for_timestamp(timestamp).ok_or(RecognizeError::UnknownBuild(timestamp))?;
    Ok(InstanceConfig::new(version, GdPlatform::Windows).with_game_folder(folder))
}

pub enum RecognizeError {
    NoProgram,
    Unreadable { program: PathBuf, source: io::Error },
    NotWindowsProgram,
    UnknownBuild(u32),
}

impl fmt::Display for RecognizeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoProgram => write!(formatter, "no {WINDOWS_PROGRAM} here"),
            Self::Unreadable { program, source } => {
                write!(formatter, "couldn't read {}: {source}", program.display())
            }
            Self::NotWindowsProgram => {
                write!(formatter, "{WINDOWS_PROGRAM} isn't a windows program")
            }
            Self::UnknownBuild(timestamp) => {
                write!(formatter, "unknown build (timestamp {timestamp})")
            }
        }
    }
}
