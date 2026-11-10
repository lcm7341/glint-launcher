use std::path::PathBuf;

pub(in crate::platform) fn folders() -> Vec<PathBuf> {
    let Some(home) = super::paths::home_dir() else {
        return Vec::new();
    };
    vec![
        home.join(".local/share/Steam"),
        home.join(".steam/steam"),
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
    ]
}
