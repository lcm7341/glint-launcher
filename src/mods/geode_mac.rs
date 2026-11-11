use std::fs;
use std::io::{self, Read};
use std::path::Path;

use flate2::read::GzDecoder;
use flate2::read::ZlibDecoder;

use crate::mods::GeodeError;

// mirrors geode's installer pkg
const FRAMEWORKS: &str = "Contents/Frameworks";
const FMOD: &str = "libfmod.dylib";
const RESTORE_FMOD: &str = "restore_fmod.dylib";
const LIBRARIES: [&str; 3] = ["Geode.dylib", "GeodeBootstrapper.dylib", FMOD];
const RESOURCES: &str = "Contents/geode/resources/geode.loader";

const MAX_PAYLOAD_BYTES: u64 = 300 * 1024 * 1024;

pub(crate) fn install_mac_geode(bundle: &Path, pkg: &[u8]) -> Result<(), GeodeError> {
    let files = pkg_payload(pkg)?;
    let frameworks = bundle.join(FRAMEWORKS);
    let has_fmod = frameworks.join(FMOD).is_file() || frameworks.join(RESTORE_FMOD).is_file();
    if !has_fmod {
        return Err(GeodeError::Archive(
            "Geometry Dash.app has no libfmod.dylib".to_owned(),
        ));
    }
    let library = |name: &str| {
        files
            .iter()
            .find(|(path, _)| path == name)
            .map(|(_, bytes)| bytes)
            .ok_or_else(|| GeodeError::Archive(format!("the installer has no {name}")))
    };
    let libraries = LIBRARIES
        .iter()
        .map(|name| Ok((*name, library(name)?)))
        .collect::<Result<Vec<_>, GeodeError>>()?;
    if !frameworks.join(RESTORE_FMOD).exists() {
        fs::rename(frameworks.join(FMOD), frameworks.join(RESTORE_FMOD)).map_err(GeodeError::Io)?;
    }
    for (name, bytes) in libraries {
        write_program(&frameworks.join(name), bytes).map_err(GeodeError::Io)?;
    }
    let resources = bundle.join(RESOURCES);
    for (path, bytes) in &files {
        let Some(name) = path.strip_prefix("resources/") else {
            continue;
        };
        if name.is_empty() || name.contains('/') || name.starts_with('.') {
            continue;
        }
        fs::create_dir_all(&resources).map_err(GeodeError::Io)?;
        fs::write(resources.join(name), bytes).map_err(GeodeError::Io)?;
    }
    Ok(())
}

/// keeps mods
pub(crate) fn remove_mac_geode(bundle: &Path) -> Result<(), GeodeError> {
    let frameworks = bundle.join(FRAMEWORKS);
    if !frameworks.join(RESTORE_FMOD).is_file() {
        return Ok(());
    }
    for name in LIBRARIES {
        match fs::remove_file(frameworks.join(name)) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => {
                return Err(GeodeError::Io(error));
            }
            _ => {}
        }
    }
    fs::rename(frameworks.join(RESTORE_FMOD), frameworks.join(FMOD)).map_err(GeodeError::Io)?;
    match fs::remove_dir_all(bundle.join(RESOURCES)) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(GeodeError::Io(error)),
        _ => Ok(()),
    }
}

fn write_program(path: &Path, bytes: &[u8]) -> io::Result<()> {
    fs::write(path, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

// xar: big-endian header, zlib xml toc, then the heap
fn pkg_payload(pkg: &[u8]) -> Result<Vec<(String, Vec<u8>)>, GeodeError> {
    let broken = |what: &str| GeodeError::Archive(format!("the installer {what}"));
    if pkg.get(..4) != Some(b"xar!".as_slice()) {
        return Err(broken("isn't a xar archive"));
    }
    let number = |range: std::ops::Range<usize>| {
        let bytes = pkg.get(range)?;
        Some(
            bytes
                .iter()
                .fold(0u64, |total, byte| total << 8 | u64::from(*byte)),
        )
    };
    let header_size = number(4..6).ok_or_else(|| broken("is cut short"))? as usize;
    let toc_size = number(8..16).ok_or_else(|| broken("is cut short"))? as usize;
    let heap = header_size + toc_size;
    let mut toc = String::new();
    ZlibDecoder::new(
        pkg.get(header_size..heap)
            .ok_or_else(|| broken("is cut short"))?,
    )
    .read_to_string(&mut toc)
    .map_err(|error| broken(&format!("has a broken file list: {error}")))?;
    let (offset, length) = payload_location(&toc).ok_or_else(|| broken("has no Payload"))?;
    let payload = pkg
        .get(heap + offset..heap + offset + length)
        .ok_or_else(|| broken("is cut short"))?;
    let mut cpio = Vec::new();
    GzDecoder::new(payload)
        .take(MAX_PAYLOAD_BYTES)
        .read_to_end(&mut cpio)
        .map_err(|error| broken(&format!("has a broken Payload: {error}")))?;
    cpio_files(&cpio).ok_or_else(|| broken("has a broken Payload"))
}

fn payload_location(toc: &str) -> Option<(usize, usize)> {
    let name = toc.find("<name>Payload</name>")?;
    let start = toc[..name].rfind("<file")?;
    let end = name + toc[name..].find("</file>")?;
    let file = &toc[start..end];
    let number = |tag: &str| -> Option<usize> {
        let open = format!("<{tag}>");
        let from = file.find(&open)? + open.len();
        let to = from + file[from..].find('<')?;
        file[from..to].trim().parse().ok()
    };
    Some((number("offset")?, number("length")?))
}

// odc cpio: 76 byte octal header, name, data
fn cpio_files(archive: &[u8]) -> Option<Vec<(String, Vec<u8>)>> {
    let mut files = Vec::new();
    let mut at = 0;
    loop {
        let header = archive.get(at..at + 76)?;
        if &header[..6] != b"070707" {
            return None;
        }
        let octal = |range: std::ops::Range<usize>| {
            usize::from_str_radix(std::str::from_utf8(&header[range]).ok()?, 8).ok()
        };
        let mode = octal(18..24)?;
        let name_size = octal(59..65)?;
        let file_size = octal(65..76)?;
        let name_start = at + 76;
        let data_start = name_start + name_size;
        let name = archive.get(name_start..data_start.checked_sub(1)?)?;
        let name = String::from_utf8_lossy(name);
        if name == "TRAILER!!!" {
            return Some(files);
        }
        let data = archive.get(data_start..data_start + file_size)?;
        if mode & 0o170000 == 0o100000 {
            let path = name.trim_start_matches("./").to_owned();
            files.push((path, data.to_vec()));
        }
        at = data_start + file_size;
    }
}
