use std::fmt;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::gd::{
    GdPlatform, MACHO_HEAD_BYTES, PE_HEADER_BYTES, macho_slices, macho_uuid, pe_timestamp,
    uuid_text, version_for_timestamp, version_for_uuid,
};
use crate::instances::InstanceConfig;

use crate::gd::WINDOWS_PROGRAM;

pub const MAC_APP: &str = "Geometry Dash.app";

const MAC_PROGRAM: &str = "Contents/MacOS/Geometry Dash";

pub fn recognize_install(folder: &Path) -> Result<InstanceConfig, RecognizeError> {
    if folder.join(WINDOWS_PROGRAM).is_file() {
        return recognize_windows(folder);
    }
    if folder.join(MAC_APP).join(MAC_PROGRAM).is_file() {
        return recognize_mac(folder);
    }
    if folder.file_name().is_some_and(|name| name == MAC_APP)
        && let Some(parent) = folder.parent()
    {
        return recognize_mac(parent);
    }
    Err(RecognizeError::NoProgram)
}

fn recognize_windows(folder: &Path) -> Result<InstanceConfig, RecognizeError> {
    let program = folder.join(WINDOWS_PROGRAM);
    let header = read_part(&program, 0, PE_HEADER_BYTES)?;
    let timestamp = pe_timestamp(&header).ok_or(RecognizeError::NotWindowsProgram)?;
    let version =
        version_for_timestamp(timestamp).ok_or(RecognizeError::UnknownBuild(timestamp))?;
    Ok(InstanceConfig::new(version, GdPlatform::Windows).with_game_folder(folder))
}

fn recognize_mac(folder: &Path) -> Result<InstanceConfig, RecognizeError> {
    let program = folder.join(MAC_APP).join(MAC_PROGRAM);
    let head = read_part(&program, 0, MACHO_HEAD_BYTES)?;
    let slices = macho_slices(&head).ok_or(RecognizeError::NotMacProgram)?;
    let mut first_uuid = None;
    for offset in slices {
        let slice = read_part(&program, offset, MACHO_HEAD_BYTES)?;
        let Some(uuid) = macho_uuid(&slice) else {
            continue;
        };
        if let Some(version) = version_for_uuid(uuid) {
            return Ok(InstanceConfig::new(version, GdPlatform::MacOs).with_game_folder(folder));
        }
        first_uuid.get_or_insert(uuid);
    }
    let uuid = first_uuid.ok_or(RecognizeError::NotMacProgram)?;
    Err(RecognizeError::UnknownMacBuild(uuid_text(uuid)))
}

fn read_part(program: &Path, offset: u64, length: usize) -> Result<Vec<u8>, RecognizeError> {
    let mut bytes = Vec::with_capacity(length);
    File::open(program)
        .and_then(|mut file| {
            file.seek(SeekFrom::Start(offset))?;
            file.take(length as u64).read_to_end(&mut bytes)
        })
        .map_err(|source| RecognizeError::Unreadable {
            program: program.to_path_buf(),
            source,
        })?;
    Ok(bytes)
}

pub enum RecognizeError {
    NoProgram,
    Unreadable { program: PathBuf, source: io::Error },
    NotWindowsProgram,
    UnknownBuild(u32),
    NotMacProgram,
    UnknownMacBuild(String),
}

impl fmt::Display for RecognizeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoProgram => write!(formatter, "no {WINDOWS_PROGRAM} or {MAC_APP} here"),
            Self::NotMacProgram => {
                write!(formatter, "{MAC_APP} isn't a mac program")
            }
            Self::UnknownMacBuild(uuid) => write!(formatter, "unknown mac build (uuid {uuid})"),
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
