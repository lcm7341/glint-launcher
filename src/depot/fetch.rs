use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::depot::FetchError;

pub const DEPOT_DOWNLOADER_VERSION: &str = "3.4.0";

const PROGRAM: &str = if cfg!(target_os = "windows") {
    "DepotDownloader.exe"
} else {
    "DepotDownloader"
};

pub fn release_asset() -> Option<String> {
    let system = if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        return None;
    };
    let processor = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86_64") {
        "x64"
    } else {
        return None;
    };
    Some(format!("DepotDownloader-{system}-{processor}.zip"))
}

pub fn fetch_depot(tools_dir: &Path) -> Result<PathBuf, FetchError> {
    let asset = release_asset().ok_or(FetchError::NoReleaseHere)?;
    let url = format!(
        "https://github.com/SteamRE/DepotDownloader/releases/download/DepotDownloader_{DEPOT_DOWNLOADER_VERSION}/{asset}"
    );
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(300)))
        .build()
        .new_agent();
    let mut response = agent
        .get(&url)
        .header("User-Agent", "glint")
        .call()
        .map_err(|error| FetchError::Network(error.to_string()))?;
    let zip = response
        .body_mut()
        .with_config()
        .limit(100 * 1024 * 1024)
        .read_to_vec()
        .map_err(|error| FetchError::Network(error.to_string()))?;
    install_from_zip(tools_dir, &zip)
}

pub(crate) fn install_from_zip(tools_dir: &Path, zip: &[u8]) -> Result<PathBuf, FetchError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(zip))
        .map_err(|error| FetchError::Archive(error.to_string()))?;
    let mut entry = archive
        .by_name(PROGRAM)
        .map_err(|_| FetchError::Archive(format!("there's no {PROGRAM} in it")))?;
    let mut program = Vec::new();
    entry
        .read_to_end(&mut program)
        .map_err(|error| FetchError::Archive(error.to_string()))?;
    fs::create_dir_all(tools_dir).map_err(FetchError::Io)?;
    let path = tools_dir.join(PROGRAM);
    let partial = tools_dir.join(format!("{PROGRAM}.part"));
    fs::write(&partial, program).map_err(FetchError::Io)?;
    make_runnable(&partial)?;
    fs::rename(&partial, &path).map_err(FetchError::Io)?;
    Ok(path)
}

#[cfg(unix)]
fn make_runnable(path: &Path) -> Result<(), FetchError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).map_err(FetchError::Io)
}

#[cfg(not(unix))]
fn make_runnable(_path: &Path) -> Result<(), FetchError> {
    Ok(())
}
