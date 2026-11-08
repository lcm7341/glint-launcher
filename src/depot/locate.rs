use std::env;
use std::path::{Path, PathBuf};

pub fn find_depot(tools_dir: &Path) -> Option<PathBuf> {
    if let Some(path) = env::var_os("GDL_DEPOTDOWNLOADER").map(PathBuf::from)
        && path.is_file()
    {
        return Some(path);
    }
    let path_dirs = env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();
    find_in(std::iter::once(tools_dir.to_path_buf()).chain(path_dirs))
}

fn find_in(dirs: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    dirs.into_iter().find_map(|dir| {
        PROGRAM_NAMES
            .iter()
            .map(|name| dir.join(name))
            .find(|path| path.is_file())
    })
}

#[cfg(target_os = "windows")]
const PROGRAM_NAMES: &[&str] = &["DepotDownloader.exe"];

#[cfg(not(target_os = "windows"))]
const PROGRAM_NAMES: &[&str] = &["DepotDownloader", "depotdownloader"];
