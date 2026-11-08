use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::mods::ModError;

const GLINT_LOADER: &[u8] = include_bytes!("../../loader/out/XInput9_1_0.dll");

const LOADER_FILE: &str = "XInput9_1_0.dll";

const BACKUP_FILE: &str = "XInput9_1_0.dll.glint-backup";

pub const GLINT_MODS_FOLDER: &str = "glint-mods";

const MARKER: &[u8] = b"GlintLoader ";

pub fn has_glint_loader(game_dir: &Path) -> bool {
    fs::read(game_dir.join(LOADER_FILE))
        .is_ok_and(|bytes| bytes.windows(MARKER.len()).any(|window| window == MARKER))
}

pub fn install_glint_loader(game_dir: &Path) -> Result<(), ModError> {
    let loader = game_dir.join(LOADER_FILE);
    let backup = game_dir.join(BACKUP_FILE);
    let backed_up = loader.exists() && !has_glint_loader(game_dir);
    if backed_up {
        if backup.exists() {
            return Err(ModError::BackupInTheWay(BACKUP_FILE.to_owned()));
        }
        fs::rename(&loader, &backup)?;
    }
    if let Err(error) = fs::write(&loader, GLINT_LOADER) {
        if backed_up {
            let _ = fs::remove_file(&loader);
            let _ = fs::rename(&backup, &loader);
        }
        return Err(error.into());
    }
    fs::create_dir_all(game_dir.join(GLINT_MODS_FOLDER))?;
    Ok(())
}

pub fn remove_glint_loader(game_dir: &Path) -> Result<(), ModError> {
    if !has_glint_loader(game_dir) {
        return Ok(());
    }
    fs::remove_file(game_dir.join(LOADER_FILE))?;
    match fs::rename(game_dir.join(BACKUP_FILE), game_dir.join(LOADER_FILE)) {
        Err(error) if error.kind() != ErrorKind::NotFound => Err(error.into()),
        _ => Ok(()),
    }
}
