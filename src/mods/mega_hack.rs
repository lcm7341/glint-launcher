use std::fs;
use std::path::{Path, PathBuf};

use crate::mods::{ModError, list_dll_mods, set_dll_mod_enabled};

const LOADER_FILES: &[&str] = &[
    "XInput9_1_0.dll",
    "XInput9_1_0.dll.echo-backup",
    "XInput9_1_0.dll.glint-backup",
];

const DISABLED_SUFFIX: &str = ".disabled";

const MARKER: &[u8] = b"hackproldr.dll";

#[derive(Clone, PartialEq, Eq)]
pub struct MegaHackLoader {
    pub path: PathBuf,
    pub is_enabled: bool,
}

pub fn find_mega_hack_loader(game_dir: &Path) -> Option<MegaHackLoader> {
    LOADER_FILES.iter().find_map(|file| {
        let on = game_dir.join(file);
        let off = game_dir.join(format!("{file}{DISABLED_SUFFIX}"));
        [(on, true), (off, false)]
            .into_iter()
            .find(|(path, _)| is_mega_hack_loader(path))
            .map(|(path, is_enabled)| MegaHackLoader { path, is_enabled })
    })
}

pub fn set_mega_hack_enabled(game_dir: &Path, enabled: bool) -> Result<(), ModError> {
    let Some(loader) = find_mega_hack_loader(game_dir) else {
        return Err(ModError::NoMegaHack);
    };
    if loader.is_enabled == enabled {
        return Ok(());
    }
    let name = loader.path.to_string_lossy();
    let target = match enabled {
        true => PathBuf::from(&name[..name.len() - DISABLED_SUFFIX.len()]),
        false => PathBuf::from(format!("{name}{DISABLED_SUFFIX}")),
    };
    if target.exists() {
        let file = target.file_name().unwrap_or_default().to_string_lossy();
        return Err(ModError::BackupInTheWay(file.into_owned()));
    }
    fs::rename(&loader.path, &target)?;
    Ok(())
}

pub(crate) const EXTENSIONS_OFF: &str = ".glint-extensions-off";

pub fn extensions_enabled(folder: &Path) -> bool {
    !folder.join(EXTENSIONS_OFF).exists()
}

pub fn set_extensions_enabled(folder: &Path, enabled: bool) -> Result<(), ModError> {
    if extensions_enabled(folder) == enabled {
        return Ok(());
    }
    let marker = folder.join(EXTENSIONS_OFF);
    if enabled {
        let was_on = fs::read_to_string(&marker)?;
        for file_name in was_on.lines().filter(|line| !line.is_empty()) {
            let off = format!("{file_name}.disabled");
            if folder.join(&off).is_file() {
                set_dll_mod_enabled(folder, &off, true)?;
            }
        }
        fs::remove_file(marker)?;
        return Ok(());
    }
    let on: Vec<String> = list_dll_mods(folder)?
        .into_iter()
        .filter(|found| found.is_enabled)
        .map(|found| found.file_name)
        .collect();
    fs::create_dir_all(folder)?;
    fs::write(&marker, on.join("\n"))?;
    for file_name in &on {
        set_dll_mod_enabled(folder, file_name, false)?;
    }
    Ok(())
}

fn is_mega_hack_loader(path: &Path) -> bool {
    fs::read(path).is_ok_and(|bytes| bytes.windows(MARKER.len()).any(|window| window == MARKER))
}
