use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, ErrorKind, Read, Seek};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde_json::{Map, Value};

use crate::mods::ModError;

pub(crate) const MODS_FOLDER: &str = "geode/mods";

const LOADER_SAVED: &str = "geode/mods/geode.loader/saved.json";

#[derive(Clone, PartialEq, Eq)]
pub struct GeodeMod {
    pub id: String,
    pub name: String,
    pub version: String,
    pub is_enabled: bool,
    pub is_pinned: bool,
    pub info: GeodeModInfo,
}

#[derive(Clone, Default, PartialEq, Eq)]
pub struct GeodeModInfo {
    pub developers: Vec<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub geode: Option<String>,
    pub gd: Option<String>,
    pub dependencies: Vec<(String, String)>,
    pub links: Vec<(String, String)>,
    pub updated: Option<SystemTime>,
    pub size: u64,
}

#[derive(Clone)]
struct Metadata {
    modified: Option<SystemTime>,
    length: u64,
    id: String,
    name: String,
    version: String,
    info: GeodeModInfo,
}

#[derive(Default)]
pub struct GeodeModsReader {
    metadata: HashMap<PathBuf, Metadata>,
}

impl GeodeModsReader {
    pub fn list(&mut self, game_dir: &Path, save_dir: &Path) -> io::Result<Vec<GeodeMod>> {
        let entries = match fs::read_dir(game_dir.join(MODS_FOLDER)) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error),
        };
        let saved = read_saved(save_dir).unwrap_or_default();
        let mut mods = Vec::new();
        for entry in entries {
            let path = entry?.path();
            if path
                .extension()
                .is_none_or(|extension| extension != "geode")
            {
                continue;
            }
            let Some(metadata) = self.metadata(&path) else {
                continue;
            };
            let saved_bool = |key: String| saved.get(&key).and_then(Value::as_bool);
            mods.push(GeodeMod {
                is_enabled: saved_bool(format!("should-load-{}", metadata.id)).unwrap_or(true),
                is_pinned: saved_bool(format!("is-pinned-{}", metadata.id)).unwrap_or(false),
                id: metadata.id.clone(),
                name: metadata.name.clone(),
                version: metadata.version.clone(),
                info: metadata.info.clone(),
            });
        }
        mods.sort_by(|a, b| {
            let by_name = a.name.to_lowercase().cmp(&b.name.to_lowercase());
            by_name.then_with(|| a.id.cmp(&b.id))
        });
        Ok(mods)
    }

    fn metadata(&mut self, path: &Path) -> Option<&Metadata> {
        let file = fs::metadata(path).ok()?;
        let (modified, length) = (file.modified().ok(), file.len());
        let is_current = self
            .metadata
            .get(path)
            .is_some_and(|known| known.modified == modified && known.length == length);
        if !is_current {
            let (id, name, version, mut info) = mod_from_json(&mod_json(File::open(path).ok()?)?)?;
            info.updated = modified;
            info.size = length;
            self.metadata.insert(
                path.to_owned(),
                Metadata {
                    modified,
                    length,
                    id,
                    name,
                    version,
                    info,
                },
            );
        }
        self.metadata.get(path)
    }
}

pub fn set_geode_mod_enabled(save_dir: &Path, id: &str, enabled: bool) -> Result<(), ModError> {
    set_saved_bool(save_dir, &format!("should-load-{id}"), enabled)
}

pub fn set_geode_mod_pinned(save_dir: &Path, id: &str, pinned: bool) -> Result<(), ModError> {
    set_saved_bool(save_dir, &format!("is-pinned-{id}"), pinned)
}

pub fn read_about(game_dir: &Path, id: &str) -> Option<String> {
    let path = game_dir.join(MODS_FOLDER).join(format!("{id}.geode"));
    let mut archive = zip::ZipArchive::new(File::open(path).ok()?).ok()?;
    let mut text = String::new();
    archive
        .by_name("about.md")
        .ok()?
        .take(256 * 1024)
        .read_to_string(&mut text)
        .ok()?;
    Some(text)
}

fn set_saved_bool(save_dir: &Path, key: &str, value: bool) -> Result<(), ModError> {
    let path = save_dir.join(LOADER_SAVED);
    let mut saved = match fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<Value>(&text) {
            Ok(Value::Object(saved)) => saved,
            _ => return Err(ModError::UnreadableSettings(path.display().to_string())),
        },
        Err(error) if error.kind() == ErrorKind::NotFound => Map::new(),
        Err(error) => return Err(error.into()),
    };
    saved.insert(key.to_owned(), Value::Bool(value));
    let text = serde_json::to_string(&Value::Object(saved)).expect("serialize");
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let partial = path.with_extension("json.glint-partial");
    fs::write(&partial, text)?;
    fs::rename(&partial, &path)?;
    Ok(())
}

fn read_saved(save_dir: &Path) -> Option<Map<String, Value>> {
    let text = fs::read_to_string(save_dir.join(LOADER_SAVED)).ok()?;
    match serde_json::from_str(&text).ok()? {
        Value::Object(saved) => Some(saved),
        _ => None,
    }
}

pub(crate) fn mod_json(geode_file: impl Read + Seek) -> Option<Value> {
    let mut archive = zip::ZipArchive::new(geode_file).ok()?;
    let mut text = String::new();
    archive
        .by_name("mod.json")
        .ok()?
        .take(1024 * 1024)
        .read_to_string(&mut text)
        .ok()?;
    serde_json::from_str(&text).ok()
}

pub(crate) fn mod_from_json(json: &Value) -> Option<(String, String, String, GeodeModInfo)> {
    let field = |key: &str| json.get(key).and_then(Value::as_str).map(str::to_owned);
    let strings = |key: &str| -> Vec<String> {
        json.get(key)
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    let id = field("id")?;
    let name = field("name").unwrap_or_else(|| id.clone());
    let mut developers = strings("developers");
    developers.extend(field("developer"));
    let info = GeodeModInfo {
        developers,
        description: field("description"),
        tags: strings("tags"),
        geode: field("geode"),
        gd: gd_for(json, "win"),
        dependencies: dependencies(json.get("dependencies")),
        links: json
            .get("links")
            .and_then(Value::as_object)
            .map(|links| {
                links
                    .iter()
                    .filter_map(|(kind, url)| Some((kind.clone(), url.as_str()?.to_owned())))
                    .collect()
            })
            .unwrap_or_default(),
        updated: None,
        size: 0,
    };
    Some((id, name, field("version").unwrap_or_default(), info))
}

pub(crate) fn gd_for(json: &Value, platform_key: &str) -> Option<String> {
    match json.get("gd") {
        Some(Value::String(version)) => Some(version.clone()),
        Some(Value::Object(platforms)) => platforms
            .get(platform_key)
            .and_then(Value::as_str)
            .map(str::to_owned),
        _ => None,
    }
}

fn dependencies(value: Option<&Value>) -> Vec<(String, String)> {
    let version_of = |value: &Value| match value {
        Value::String(version) => version.clone(),
        other => other
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
    };
    match value {
        Some(Value::Object(by_id)) => by_id
            .iter()
            .map(|(id, value)| (id.clone(), version_of(value)))
            .collect(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| Some((item.get("id")?.as_str()?.to_owned(), version_of(item))))
            .collect(),
        _ => Vec::new(),
    }
}
