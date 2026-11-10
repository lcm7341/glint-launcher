use std::path::{Path, PathBuf};

use crate::mods::{GLINT_MODS_FOLDER, has_glint_loader};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DllLoader {
    MegaHack,
    GlintLoader,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ModsFolder {
    pub loader: DllLoader,
    pub path: PathBuf,
}

pub fn has_mega_hack(game_dir: &Path) -> bool {
    ["hackpro.dll", "hackproldr.dll"]
        .iter()
        .all(|file| game_dir.join(file).is_file())
}

pub const MEGA_HACK_MODS_FOLDER: &str = "extensions";

// glint loader loads its mods with mega hack on or off, so it wins when both are there
pub fn find_mods_folder(game_dir: &Path) -> Option<ModsFolder> {
    let (loader, folder) = if has_glint_loader(game_dir) {
        (DllLoader::GlintLoader, GLINT_MODS_FOLDER)
    } else if has_mega_hack(game_dir) {
        (DllLoader::MegaHack, MEGA_HACK_MODS_FOLDER)
    } else {
        return None;
    };
    Some(ModsFolder {
        loader,
        path: game_dir.join(folder),
    })
}
