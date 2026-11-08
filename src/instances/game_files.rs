use std::fs;
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GameFiles {
    Missing,
    Incomplete,
    Present,
}

pub(crate) const INCOMPLETE_MARKER: &str = ".gdl-download-incomplete";

impl GameFiles {
    pub(crate) fn of(game_dir: &Path) -> Self {
        if game_dir.join(INCOMPLETE_MARKER).exists() {
            return GameFiles::Incomplete;
        }
        let has_files = fs::read_dir(game_dir).is_ok_and(|entries| {
            entries
                .filter_map(Result::ok)
                .any(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        });
        if has_files {
            GameFiles::Present
        } else {
            GameFiles::Missing
        }
    }
}
