use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::gd::GdVersion;

const KEPT_BACKUPS: usize = 10;

const KEPT_DAYS: u128 = 7;

const DAY_MILLIS: u128 = 86_400_000;

/// Skips the copy when the newest backup already matches
pub fn back_up_save(
    save_dir: &Path,
    backups: &Path,
    version: &GdVersion,
) -> io::Result<Option<PathBuf>> {
    let files = save_files(save_dir)?;
    if files.is_empty() {
        return Ok(None);
    }
    let existing = backup_folders(backups)?;
    if let Some(newest) = existing.last()
        && same_files(&files, newest)?
    {
        return Ok(None);
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis());
    // always after the newest, so names sort oldest first even within one millisecond
    let stamp = existing
        .last()
        .and_then(|newest| stamp_of(newest))
        .map_or(now, |newest| now.max(newest + 1));
    let folder = backups.join(format!("{stamp:013}-{version}"));
    fs::create_dir_all(&folder)?;
    for file in &files {
        if let Some(name) = file.file_name() {
            fs::copy(file, folder.join(name))?;
        }
    }
    let mut all = existing;
    all.push(folder.clone());
    for old in stale_backups(&all, now) {
        let _ = fs::remove_dir_all(old);
    }
    Ok(Some(folder))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveBackup {
    pub folder: PathBuf,
    pub taken: SystemTime,
    pub version: String,
}

pub fn list_backups(backups: &Path) -> io::Result<Vec<SaveBackup>> {
    let mut found: Vec<SaveBackup> = backup_folders(backups)?
        .into_iter()
        .filter_map(|folder| {
            let stamp = stamp_of(&folder)?;
            let name = folder.file_name()?.to_str()?;
            let version = name.split_once('-')?.1.to_owned();
            let millis = u64::try_from(stamp).ok()?;
            Some(SaveBackup {
                taken: UNIX_EPOCH + std::time::Duration::from_millis(millis),
                version,
                folder,
            })
        })
        .collect();
    found.reverse();
    Ok(found)
}

pub fn read_backup(backup: &Path) -> io::Result<Vec<(std::ffi::OsString, Vec<u8>)>> {
    let files = save_files(backup)?;
    if files.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{} has no save files", backup.display()),
        ));
    }
    files
        .into_iter()
        .map(|file| {
            let name = file.file_name().unwrap_or_default().to_owned();
            Ok((name, fs::read(&file)?))
        })
        .collect()
}

pub fn restore_files(files: &[(std::ffi::OsString, Vec<u8>)], save_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(save_dir)?;
    for (name, bytes) in files {
        let mut temp = name.clone();
        temp.push(".glint-restore");
        let temp = save_dir.join(temp);
        fs::write(&temp, bytes)?;
        fs::rename(&temp, save_dir.join(name))?;
    }
    for current in save_files(save_dir)? {
        let kept = current
            .file_name()
            .is_some_and(|name| files.iter().any(|(restored, _)| restored == name));
        if !kept {
            fs::remove_file(current)?;
        }
    }
    Ok(())
}

// the rest of the folder is downloaded songs
fn save_files(save_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let entries = match fs::read_dir(save_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.starts_with("CC")
                && name.ends_with(".dat")
                && entry.file_type().is_ok_and(|kind| kind.is_file())
        })
        .map(|entry| entry.path())
        .collect();
    files.sort();
    Ok(files)
}

/// Also keeps the newest backup from before each version played, so a broken old version
/// can't push out the last good save, and the newest from each recent day
fn stale_backups(folders: &[PathBuf], now: u128) -> Vec<&PathBuf> {
    let mut versions = Vec::new();
    let mut days = Vec::new();
    let mut stale = Vec::new();
    for (age, folder) in folders.iter().rev().enumerate() {
        let stamp = stamp_of(folder).unwrap_or(0);
        let version = folder
            .file_name()
            .and_then(|name| name.to_str()?.split_once('-').map(|(_, version)| version))
            .unwrap_or_default();
        let day = stamp / DAY_MILLIS;
        let newest_of_version = !versions.contains(&version);
        let newest_of_recent_day =
            now.saturating_sub(stamp) < KEPT_DAYS * DAY_MILLIS && !days.contains(&day);
        versions.push(version);
        days.push(day);
        if age >= KEPT_BACKUPS && !newest_of_version && !newest_of_recent_day {
            stale.push(folder);
        }
    }
    stale
}

fn stamp_of(folder: &Path) -> Option<u128> {
    let name = folder.file_name()?.to_str()?;
    name.split_once('-')?.0.parse().ok()
}

fn backup_folders(backups: &Path) -> io::Result<Vec<PathBuf>> {
    let entries = match fs::read_dir(backups) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut folders: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .collect();
    // names start with zero padded milliseconds, so this is oldest first
    folders.sort();
    Ok(folders)
}

fn same_files(files: &[PathBuf], backup: &Path) -> io::Result<bool> {
    if save_files(backup)?.len() != files.len() {
        return Ok(false);
    }
    for file in files {
        let Some(name) = file.file_name() else {
            return Ok(false);
        };
        let copy = backup.join(name);
        let same_size = fs::metadata(&copy).is_ok_and(|copy| {
            fs::metadata(file).is_ok_and(|original| original.len() == copy.len())
        });
        if !same_size || fs::read(file)? != fs::read(&copy)? {
            return Ok(false);
        }
    }
    Ok(true)
}
