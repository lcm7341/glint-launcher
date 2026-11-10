use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::mods::ModError;
use crate::mods::mega_hack::EXTENSIONS_OFF;

const GLINT_LOADER: &[u8] = include_bytes!("../../loader/out/XInput9_1_0.dll");

const LOADER_FILE: &str = "XInput9_1_0.dll";

const BACKUP_FILE: &str = "XInput9_1_0.dll.glint-backup";

// disabled mega hack loader
const OFF_LOADER: &str = "XInput9_1_0.dll.disabled";
const OFF_BACKUP: &str = "XInput9_1_0.dll.glint-backup.disabled";

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
    if game_dir.join(OFF_LOADER).exists() && !game_dir.join(OFF_BACKUP).exists() {
        fs::rename(game_dir.join(OFF_LOADER), game_dir.join(OFF_BACKUP))?;
    }
    fs::create_dir_all(game_dir.join(GLINT_MODS_FOLDER))?;
    Ok(())
}

/// dlls and the extensions marker
pub fn move_dll_mods(from: &Path, to: &Path) -> Result<(), ModError> {
    let entries = match fs::read_dir(from) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    fs::create_dir_all(to)?;
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let lower = name.to_string_lossy().to_lowercase();
        let is_mod = lower.ends_with(".dll") || lower.ends_with(".dll.disabled");
        if !entry.file_type()?.is_file() || !(is_mod || lower == EXTENSIONS_OFF) {
            continue;
        }
        let target = to.join(&name);
        if target.exists() {
            return Err(ModError::BackupInTheWay(
                name.to_string_lossy().into_owned(),
            ));
        }
        fs::rename(entry.path(), target)?;
    }
    Ok(())
}

pub fn remove_glint_loader(game_dir: &Path) -> Result<(), ModError> {
    if !has_glint_loader(game_dir) {
        return Ok(());
    }
    fs::remove_file(game_dir.join(LOADER_FILE))?;
    for (from, to) in [(BACKUP_FILE, LOADER_FILE), (OFF_BACKUP, OFF_LOADER)] {
        match fs::rename(game_dir.join(from), game_dir.join(to)) {
            Err(error) if error.kind() != ErrorKind::NotFound => return Err(error.into()),
            _ => {}
        }
    }
    Ok(())
}
