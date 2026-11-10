use std::path::PathBuf;

pub(in crate::platform) fn folders() -> Vec<PathBuf> {
    super::paths::home_dir()
        .map(|home| home.join("Library/Application Support/Steam"))
        .into_iter()
        .collect()
}
