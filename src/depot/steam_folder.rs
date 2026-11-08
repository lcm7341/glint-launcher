use std::env;
use std::path::PathBuf;

pub fn steam_folder() -> Option<PathBuf> {
    steam_folders()
        .into_iter()
        .find(|folder| folder.join("steamapps").is_dir())
}

pub(crate) fn steam_folders() -> Vec<PathBuf> {
    let home = env::var_os("HOME").map(PathBuf::from);
    let mut folders = Vec::new();
    if cfg!(target_os = "windows") {
        if let Some(program_files) = env::var_os("ProgramFiles(x86)") {
            folders.push(PathBuf::from(program_files).join("Steam"));
        }
    } else if cfg!(target_os = "macos") {
        folders.extend(home.map(|home| {
            home.join("Library")
                .join("Application Support")
                .join("Steam")
        }));
    } else if let Some(home) = home {
        folders.push(home.join(".local").join("share").join("Steam"));
        folders.push(home.join(".steam").join("steam"));
        folders.push(
            home.join(".var")
                .join("app")
                .join("com.valvesoftware.Steam")
                .join(".local")
                .join("share")
                .join("Steam"),
        );
    }
    folders
}
