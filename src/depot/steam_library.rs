use std::fs;
use std::path::{Path, PathBuf};

use crate::gd::{steam_key_values, steam_value};

pub fn steam_app_install(steam_folder: &Path, app_id: u32) -> Option<PathBuf> {
    steam_libraries(steam_folder)
        .into_iter()
        .find_map(|library| {
            let steamapps = library.join("steamapps");
            let manifest = fs::read_to_string(steamapps.join(format!("appmanifest_{app_id}.acf")));
            let install_dir = install_dir(&manifest.ok()?)?;
            Some(steamapps.join("common").join(install_dir))
        })
}

pub fn steam_libraries(steam_folder: &Path) -> Vec<PathBuf> {
    let mut libraries = vec![steam_folder.to_owned()];
    let listed = fs::read_to_string(steam_folder.join("steamapps").join("libraryfolders.vdf"));
    for library in library_paths(&listed.unwrap_or_default()) {
        if !libraries.iter().any(|known| same_path(known, &library)) {
            libraries.push(library);
        }
    }
    libraries
}

fn library_paths(vdf: &str) -> Vec<PathBuf> {
    steam_key_values(vdf)
        .filter(|(key, _)| key == "path")
        .map(|(_, path)| PathBuf::from(path))
        .collect()
}

fn install_dir(acf: &str) -> Option<String> {
    steam_value(acf, "installdir").filter(|folder| !folder.is_empty())
}

fn same_path(first: &Path, second: &Path) -> bool {
    let normal = |path: &Path| {
        path.to_string_lossy()
            .replace('\\', "/")
            .trim_end_matches('/')
            .to_lowercase()
    };
    normal(first) == normal(second)
}
