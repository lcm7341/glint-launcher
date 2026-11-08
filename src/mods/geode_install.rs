use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::mods::{GeodeError, GeodeRelease};

const GEODE_FILES: &[&str] = &[
    "Geode.dll",
    "Geode.lib",
    "Geode.pdb",
    "GeodeUpdater.exe",
    "XInput1_4.dll",
];

pub fn has_geode(game_dir: &Path) -> bool {
    game_dir.join("Geode.dll").is_file()
}

pub fn install_geode(game_dir: &Path, release: &GeodeRelease) -> Result<(), GeodeError> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(300)))
        .build()
        .new_agent();
    let mut response = agent
        .get(&release.url)
        .header("User-Agent", "glint")
        .call()
        .map_err(|error| GeodeError::Network(error.to_string()))?;
    let zip = response
        .body_mut()
        .with_config()
        .limit(200 * 1024 * 1024)
        .read_to_vec()
        .map_err(|error| GeodeError::Network(error.to_string()))?;
    install_geode_zip(game_dir, &zip, &release.sha256)
}

pub fn remove_geode(game_dir: &Path) -> Result<(), GeodeError> {
    for file in GEODE_FILES {
        match fs::remove_file(game_dir.join(file)) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                return Err(GeodeError::Io(error));
            }
            _ => {}
        }
    }
    Ok(())
}

pub(crate) fn install_geode_zip(
    game_dir: &Path,
    zip: &[u8],
    sha256: &str,
) -> Result<(), GeodeError> {
    if hex(&Sha256::digest(zip)) != sha256 {
        return Err(GeodeError::Checksum);
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(zip))
        .map_err(|error| GeodeError::Archive(error.to_string()))?;
    let mut files = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| GeodeError::Archive(error.to_string()))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_owned();
        if name.contains(['/', '\\']) || name.starts_with('.') {
            return Err(GeodeError::Archive(format!("bad path in zip: {name}")));
        }
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|error| GeodeError::Archive(error.to_string()))?;
        files.push((name, bytes));
    }
    for (name, bytes) in files {
        fs::write(game_dir.join(name), bytes).map_err(GeodeError::Io)?;
    }
    Ok(())
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
