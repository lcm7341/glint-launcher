use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::mods::ModError;

const DISABLED_SUFFIX: &str = ".disabled";

#[derive(Clone, PartialEq, Eq)]
pub struct DllMod {
    pub name: String,
    pub file_name: String,
    pub is_enabled: bool,
}

impl DllMod {
    pub fn from_file_name(file_name: &str) -> Option<Self> {
        let (stem, is_enabled) = match strip_suffix_ci(file_name, DISABLED_SUFFIX) {
            Some(stem) => (stem, false),
            None => (file_name, true),
        };
        let name = strip_suffix_ci(stem, ".dll")?;
        (!name.is_empty()).then(|| Self {
            name: name.to_owned(),
            file_name: file_name.to_owned(),
            is_enabled,
        })
    }
}

pub fn list_dll_mods(folder: &Path) -> std::io::Result<Vec<DllMod>> {
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut mods = Vec::new();
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        if let Some(found) = entry.file_name().to_str().and_then(DllMod::from_file_name) {
            mods.push(found);
        }
    }
    mods.sort_by(|a, b| {
        let by_name = a.name.to_lowercase().cmp(&b.name.to_lowercase());
        by_name.then_with(|| a.file_name.cmp(&b.file_name))
    });
    Ok(mods)
}

pub fn set_dll_mod_enabled(
    folder: &Path,
    file_name: &str,
    enabled: bool,
) -> Result<DllMod, ModError> {
    let current = mod_in_folder(file_name)?;
    if current.is_enabled == enabled {
        return Ok(current);
    }
    let new_name = match enabled {
        true => file_name[..file_name.len() - DISABLED_SUFFIX.len()].to_owned(),
        false => format!("{file_name}{DISABLED_SUFFIX}"),
    };
    let target = folder.join(&new_name);
    if target.exists() {
        return Err(ModError::AlreadyThere(current.name));
    }
    fs::rename(folder.join(file_name), target)?;
    Ok(DllMod::from_file_name(&new_name).expect("valid file name"))
}

pub fn add_dll_mod(folder: &Path, source: &Path) -> Result<DllMod, ModError> {
    let file_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let added = DllMod::from_file_name(file_name)
        .filter(|found| found.is_enabled)
        .ok_or_else(|| ModError::NotADll(file_name.to_owned()))?;
    fs::create_dir_all(folder)?;
    let taken = list_dll_mods(folder)?
        .iter()
        .any(|existing| existing.name.eq_ignore_ascii_case(&added.name));
    if taken {
        return Err(ModError::AlreadyThere(added.name));
    }
    fs::copy(source, folder.join(file_name))?;
    Ok(added)
}

pub fn remove_dll_mod(folder: &Path, file_name: &str) -> Result<(), ModError> {
    mod_in_folder(file_name)?;
    fs::remove_file(folder.join(file_name))?;
    Ok(())
}

fn mod_in_folder(file_name: &str) -> Result<DllMod, ModError> {
    let is_plain = !file_name.contains(['/', '\\']) && !file_name.starts_with('.');
    DllMod::from_file_name(file_name)
        .filter(|_| is_plain)
        .ok_or_else(|| ModError::NotADll(file_name.to_owned()))
}

fn strip_suffix_ci<'a>(text: &'a str, suffix: &str) -> Option<&'a str> {
    let split = text.len().checked_sub(suffix.len())?;
    let (start, end) = (text.get(..split)?, text.get(split..)?);
    end.eq_ignore_ascii_case(suffix).then_some(start)
}
