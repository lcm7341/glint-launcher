use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use sha2::{Digest, Sha256};

use super::{Release, UpdateError};

const MAX_BYTES: u64 = 200 * 1024 * 1024;

/// Where a macOS update unpacks, next to Glint.app
const STAGING: &str = ".glint-update";

/// Downloads the release over the running copy of glint and returns what to relaunch
pub(crate) fn install(release: &Release) -> Result<PathBuf, UpdateError> {
    let exe = env::current_exe()?;
    let bytes = download(release)?;
    if cfg!(target_os = "macos") {
        let bundle = app_bundle(&exe).ok_or_else(|| {
            UpdateError::Unsupported("glint isn't running from Glint.app".to_owned())
        })?;
        replace_bundle(&bundle, &bytes)?;
        Ok(bundle)
    } else {
        replace_program(&exe, &bytes)?;
        Ok(exe)
    }
}

pub(crate) fn relaunch(program: &Path) -> io::Result<()> {
    let mut command = if program.is_dir() {
        let mut open = Command::new("open");
        open.arg("-n").arg(program);
        open
    } else {
        let mut direct = Command::new(program);
        direct.args(env::args_os().skip(1));
        direct
    };
    command.spawn().map(drop)
}

/// Removes what the last update left behind; windows keeps the old program locked until it exits
pub(crate) fn clean_up_old_files() {
    let Ok(exe) = env::current_exe() else {
        return;
    };
    let _ = fs::remove_file(sibling(&exe, "old"));
    let _ = fs::remove_file(sibling(&exe, "new"));
    if cfg!(target_os = "macos")
        && let Some(bundle) = app_bundle(&exe)
        && let Some(parent) = bundle.parent()
    {
        let _ = fs::remove_dir_all(parent.join(STAGING));
        let _ = fs::remove_dir_all(old_bundle(&bundle));
    }
}

fn download(release: &Release) -> Result<Vec<u8>, UpdateError> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(300)))
        .build()
        .new_agent();
    let mut response = agent
        .get(&release.url)
        .header("User-Agent", "glint")
        .call()
        .map_err(|error| UpdateError::Network(error.to_string()))?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(MAX_BYTES)
        .read_to_vec()
        .map_err(|error| UpdateError::Network(error.to_string()))?;
    verify(&bytes, release)?;
    Ok(bytes)
}

fn verify(bytes: &[u8], release: &Release) -> Result<(), UpdateError> {
    let hash: String = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let size_matches = release.size == 0 || release.size == bytes.len() as u64;
    if hash != release.sha256 || !size_matches {
        return Err(UpdateError::Checksum);
    }
    Ok(())
}

fn replace_program(exe: &Path, bytes: &[u8]) -> Result<(), UpdateError> {
    let new = sibling(exe, "new");
    let old = sibling(exe, "old");
    fs::write(&new, bytes)?;
    fs::set_permissions(&new, fs::metadata(exe)?.permissions())?;
    match fs::remove_file(&old) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error.into()),
        _ => {}
    }
    // windows can't overwrite a running program, but it can rename one
    fs::rename(exe, &old)?;
    if let Err(error) = fs::rename(&new, exe) {
        let _ = fs::rename(&old, exe);
        let _ = fs::remove_file(&new);
        return Err(error.into());
    }
    Ok(())
}

fn replace_bundle(bundle: &Path, zip: &[u8]) -> Result<(), UpdateError> {
    let parent = bundle
        .parent()
        .ok_or_else(|| UpdateError::Unsupported("Glint.app has no parent folder".to_owned()))?;
    let staging = parent.join(STAGING);
    remove_dir(&staging)?;
    extract(zip, &staging)?;
    let fresh = staging.join("Glint.app");
    if !fresh.join("Contents/MacOS/glint").is_file() {
        return Err(UpdateError::Archive("it has no Glint.app".to_owned()));
    }
    let old = old_bundle(bundle);
    remove_dir(&old)?;
    fs::rename(bundle, &old)?;
    if let Err(error) = fs::rename(&fresh, bundle) {
        let _ = fs::rename(&old, bundle);
        return Err(error.into());
    }
    let _ = fs::remove_dir_all(&old);
    let _ = fs::remove_dir_all(&staging);
    Ok(())
}

fn extract(zip: &[u8], dest: &Path) -> Result<(), UpdateError> {
    let broken = |error: zip::result::ZipError| UpdateError::Archive(error.to_string());
    let mut archive = zip::ZipArchive::new(Cursor::new(zip)).map_err(broken)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(broken)?;
        let relative = entry
            .enclosed_name()
            .ok_or_else(|| UpdateError::Archive(format!("bad path in zip: {}", entry.name())))?;
        let path = dest.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&path)?;
            continue;
        }
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)?;
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        if entry.is_symlink() {
            make_symlink(&bytes, &path)?;
        } else {
            fs::write(&path, &bytes)?;
            set_mode(&path, entry.unix_mode())?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: Option<u32>) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    match mode {
        Some(mode) => fs::set_permissions(path, fs::Permissions::from_mode(mode & 0o7777)),
        None => Ok(()),
    }
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: Option<u32>) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn make_symlink(target: &[u8], link: &Path) -> Result<(), UpdateError> {
    use std::os::unix::ffi::OsStrExt;
    use std::path::Component;
    let target = Path::new(std::ffi::OsStr::from_bytes(target));
    // links stay inside the bundle
    let escapes = target.is_absolute()
        || target
            .components()
            .any(|component| component == Component::ParentDir);
    if escapes {
        return Err(UpdateError::Archive(format!(
            "bad link in zip: {}",
            target.display()
        )));
    }
    std::os::unix::fs::symlink(target, link)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_symlink(_target: &[u8], link: &Path) -> Result<(), UpdateError> {
    Err(UpdateError::Archive(format!(
        "can't make link {}",
        link.display()
    )))
}

fn remove_dir(path: &Path) -> io::Result<()> {
    match fs::remove_dir_all(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

fn app_bundle(exe: &Path) -> Option<PathBuf> {
    exe.ancestors()
        .find(|folder| {
            folder
                .extension()
                .is_some_and(|extension| extension == "app")
                && folder.join("Contents").is_dir()
        })
        .map(Path::to_path_buf)
}

fn old_bundle(bundle: &Path) -> PathBuf {
    let mut name = OsString::from(".");
    name.push(bundle.file_name().unwrap_or_default());
    name.push(".old");
    bundle.with_file_name(name)
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".");
    name.push(suffix);
    path.with_file_name(name)
}
