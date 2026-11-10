use std::path::{Path, PathBuf};

use crate::platform::non_empty_env;

pub(in crate::platform) fn home_dir() -> Option<PathBuf> {
    non_empty_env("HOME")
}
pub(in crate::platform) fn data_dir() -> Option<PathBuf> {
    non_empty_env("XDG_DATA_HOME")
        .or_else(|| home_dir().map(|home| home.join(".local/share")))
        .map(|root| root.join("glint"))
}
pub(in crate::platform) fn native_save_dir(_home: Option<&Path>) -> Option<PathBuf> {
    None
}
