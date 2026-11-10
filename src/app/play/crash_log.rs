use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

// mega hack writes crashlogs/, geode geode/crashlogs/, echo for 2.1 .echo/crashlogs/
const CRASH_LOG_FOLDERS: [&str; 3] = ["crashlogs", "geode/crashlogs", ".echo/crashlogs"];

pub(super) fn new_crash_log(game_dir: &Path, since: SystemTime) -> Option<PathBuf> {
    CRASH_LOG_FOLDERS
        .iter()
        .filter_map(|folder| fs::read_dir(game_dir.join(folder)).ok())
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            let modified = metadata.modified().ok()?;
            (metadata.is_file() && modified >= since).then(|| (modified, entry.path()))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}
