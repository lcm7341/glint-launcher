use std::path::{Path, PathBuf};

use crate::platform::non_empty_env;

pub(in crate::platform) fn home_dir() -> Option<PathBuf> {
    non_empty_env("HOME")
}
pub(in crate::platform) fn data_dir() -> Option<PathBuf> {
    home_dir().map(|home| home.join("Library/Application Support/glint"))
}
pub(in crate::platform) fn native_save_dir(home: Option<&Path>) -> Option<PathBuf> {
    home.map(Path::to_path_buf)
        .or_else(home_dir)
        .map(|home| home.join("Library/Application Support/GeometryDash"))
}
