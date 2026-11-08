use std::fs;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::depot::ManifestListing;

pub fn seed_folder(
    listing: &ManifestListing,
    seed: &Path,
    target: &Path,
    cancelled: &AtomicBool,
    on_copied: &mut dyn FnMut(u64),
) -> io::Result<u64> {
    if seed == target {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "can't copy a folder into itself",
        ));
    }
    let mut copied = 0;
    for entry in listing.entries() {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        if entry.is_directory {
            continue;
        }
        let from = seed.join(&entry.path);
        let to = target.join(&entry.path);
        let is_plain_file = fs::symlink_metadata(&from).is_ok_and(|metadata| metadata.is_file());
        if !is_plain_file || fs::symlink_metadata(&to).is_ok() {
            continue;
        }
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        copied += fs::copy(&from, &to)?;
        on_copied(copied);
    }
    Ok(copied)
}
