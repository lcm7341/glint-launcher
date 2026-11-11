use std::fs::{self, File};
use std::io::{Cursor, Read};
use std::path::Path;

use crate::gd::{GdPlatform, GdVersion};

use crate::mods::GeodeError;
use crate::mods::geode_mod_install::{geode_mods_dir, is_mod_id, write_geode_mod};
use crate::mods::geode_mods::{gd_for, mod_from_json, mod_json};
use crate::mods::geode_server::{MAX_MOD_BYTES, server_gd};

#[derive(Clone, PartialEq, Eq)]
pub struct AddedGeodeMod {
    pub id: String,
    pub name: String,
    pub made_for: Option<String>,
    pub is_made_for_gd: bool,
}

pub fn add_geode_mod_file(
    game_dir: &Path,
    source: &Path,
    gd: &GdVersion,
    platform: GdPlatform,
) -> Result<AddedGeodeMod, GeodeError> {
    let file = source.file_name().map_or_else(
        || source.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    let unreadable = |error| GeodeError::UnreadableModFile {
        file: file.clone(),
        error,
    };
    let length = fs::metadata(source).map_err(unreadable)?.len();
    if length > MAX_MOD_BYTES {
        return Err(GeodeError::NotAGeodeMod(file));
    }
    let mut bytes = Vec::new();
    File::open(source)
        .and_then(|opened| opened.take(MAX_MOD_BYTES).read_to_end(&mut bytes))
        .map_err(unreadable)?;
    let json = mod_json(Cursor::new(&bytes));
    let Some((id, name, _, _)) = json
        .as_ref()
        .and_then(|json| mod_from_json(json, mod_json_platform(platform)))
    else {
        return Err(GeodeError::NotAGeodeMod(file));
    };
    if !is_mod_id(&id) {
        return Err(GeodeError::NotAGeodeMod(file));
    }
    let made_for = json
        .as_ref()
        .and_then(|json| gd_for(json, mod_json_platform(platform)));
    let is_made_for_gd = made_for.as_deref().is_some_and(|made_for| {
        made_for == "*" || made_for == server_gd(gd) || made_for == gd.to_string()
    });
    write_geode_mod(&geode_mods_dir(game_dir), &id, &bytes)?;
    Ok(AddedGeodeMod {
        id,
        name,
        made_for,
        is_made_for_gd,
    })
}

fn mod_json_platform(platform: GdPlatform) -> &'static str {
    match platform {
        GdPlatform::Windows => "win",
        GdPlatform::MacOs => "mac",
    }
}
