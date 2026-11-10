use std::path::PathBuf;

pub fn steam_folder() -> Option<PathBuf> {
    crate::platform::steam_folders()
        .into_iter()
        .find(|folder| folder.join("steamapps").is_dir())
}
