use std::fs;
use std::path::{Path, PathBuf};

pub fn is_running_in(running: &[PathBuf], game_dir: &Path) -> bool {
    let wanted = fs::canonicalize(game_dir).unwrap_or_else(|_| game_dir.to_owned());
    running
        .iter()
        .any(|dir| fs::canonicalize(dir).unwrap_or_else(|_| dir.clone()) == wanted)
}
