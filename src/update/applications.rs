use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::apply::{app_bundle, old_bundle};
use crate::platform::home_dir;

const APPLICATIONS: &str = "/Applications";
const BUNDLE: &str = "Glint.app";

/// glint.app when it runs from outside an Applications folder, like Downloads or the dmg.
/// macos runs a downloaded app from a read-only copy there, so it can't update itself
pub(crate) fn bundle_outside_applications() -> Option<PathBuf> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let bundle = app_bundle(&env::current_exe().ok()?)?;
    let inside = applications_folders()
        .iter()
        .any(|folder| bundle.starts_with(folder));
    (!inside).then_some(bundle)
}

/// copies glint.app into Applications, deletes the original and returns the copy to open
pub(crate) fn move_to_applications(bundle: &Path) -> io::Result<PathBuf> {
    let mut failed = io::Error::other("no Applications folder");
    for folder in applications_folders() {
        match copy_into(bundle, &folder) {
            Ok(moved) => {
                // fails on the dmg and on macos's read-only copy, those stay
                let _ = fs::remove_dir_all(bundle);
                return Ok(moved);
            }
            Err(error) => failed = error,
        }
    }
    Err(failed)
}

/// ~/Applications when the user can't write to /Applications
fn applications_folders() -> Vec<PathBuf> {
    let mut folders = vec![PathBuf::from(APPLICATIONS)];
    folders.extend(home_dir().map(|home| home.join("Applications")));
    folders
}

fn copy_into(bundle: &Path, folder: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(folder)?;
    let moved = folder.join(BUNDLE);
    let staging = folder.join(".Glint.app.moving");
    let _ = fs::remove_dir_all(&staging);
    // ditto keeps the signature, links and permissions
    if let Err(error) = run(Command::new("ditto").arg(bundle).arg(&staging)) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    // the user already opened this copy, so the moved one skips the gatekeeper prompt
    let _ = Command::new("xattr")
        .args(["-d", "-r", "com.apple.quarantine"])
        .arg(&staging)
        .output();
    if moved.exists() {
        let old = old_bundle(&moved);
        let _ = fs::remove_dir_all(&old);
        fs::rename(&moved, &old)?;
        if let Err(error) = fs::rename(&staging, &moved) {
            let _ = fs::rename(&old, &moved);
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
        let _ = fs::remove_dir_all(&old);
    } else if let Err(error) = fs::rename(&staging, &moved) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    Ok(moved)
}

fn run(command: &mut Command) -> io::Result<()> {
    let output = command.output()?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(io::Error::other(stderr.trim().to_owned()))
}
