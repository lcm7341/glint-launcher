use std::fs;
use std::path::Path;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::gd::GdVersion;
use crate::mods::GeodeError;
use crate::mods::echo::echo_folder;
use crate::mods::geode_install::hex;
use crate::mods::geode_mod_install::{geode_mods_dir, write_geode_mods};
use crate::mods::geode_server::{download, get_text};

// echogd.com hosts echo itself, it isn't on geode's index
const GEODE_MANIFEST: &str = "https://echogd.com/download/latest.json";
const GD21_MANIFEST: &str = "https://echogd.com/echo21/latest.json";

pub const ECHO_ID: &str = "decks.echo";

pub const ECHO_21_GD: &str = "2.113";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EchoRelease {
    pub version: String,
    pub description: Option<String>,
    pub url: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EchoReleases {
    geode: Vec<(String, EchoRelease)>,
    gd21: Option<EchoRelease>,
}

impl EchoReleases {
    pub fn for_geode(&self, gd: &GdVersion) -> Option<&EchoRelease> {
        let gd = gd.to_string();
        self.geode
            .iter()
            .find(|(version, _)| *version == gd)
            .map(|(_, release)| release)
    }

    pub fn for_gd21(&self) -> Option<&EchoRelease> {
        self.gd21.as_ref()
    }
}

pub fn fetch_echo_releases() -> Result<EchoReleases, String> {
    let fetch = |url: &str| get_text(url, &[]).map_err(|error| format!("{url}: {error}"));
    let geode = parse_geode_manifest(&fetch(GEODE_MANIFEST)?);
    let gd21 = parse_gd21_manifest(&fetch(GD21_MANIFEST)?);
    Ok(EchoReleases { geode, gd21 })
}

// the newest build sits at the top, builds for other gd versions under their version
pub(crate) fn parse_geode_manifest(body: &str) -> Vec<(String, EchoRelease)> {
    let Ok(json) = serde_json::from_str::<Value>(body) else {
        return Vec::new();
    };
    let entries = std::iter::once(&json).chain(
        json.as_object()
            .into_iter()
            .flat_map(|object| object.values().filter(|value| value.is_object())),
    );
    entries
        .filter_map(|entry| {
            let release = release(entry, "geode")?;
            Some((entry["gd"].as_str()?.to_owned(), release))
        })
        .collect()
}

pub(crate) fn parse_gd21_manifest(body: &str) -> Option<EchoRelease> {
    release(&serde_json::from_str(body).ok()?, "dll")
}

fn release(entry: &Value, file: &str) -> Option<EchoRelease> {
    let sha256 = entry["sha256"].as_str()?.to_ascii_lowercase();
    let is_hash = sha256.len() == 64 && sha256.bytes().all(|byte| byte.is_ascii_hexdigit());
    let url = entry[file].as_str()?;
    let version = entry["version"]
        .as_str()?
        .trim_start_matches('v')
        .to_owned();
    let description = entry["description"].as_str().map(str::to_owned);
    (is_hash && url.starts_with("https://echogd.com/")).then(|| EchoRelease {
        version,
        description,
        url: url.to_owned(),
        sha256,
    })
}

pub fn install_echo_geode(game_dir: &Path, release: &EchoRelease) -> Result<(), GeodeError> {
    let bytes = download(&release.url)?;
    write_geode_mods(
        &geode_mods_dir(game_dir),
        &[(ECHO_ID, bytes, release.sha256.as_str())],
    )
}

/// Puts the dll where Echo's own loader looks, which loads the newest version there
pub fn install_echo_21(game_dir: &Path, release: &EchoRelease) -> Result<(), GeodeError> {
    let bytes = download(&release.url)?;
    if hex(&Sha256::digest(&bytes)) != release.sha256 {
        return Err(GeodeError::Checksum);
    }
    let folder = echo_folder(game_dir);
    fs::create_dir_all(&folder).map_err(GeodeError::Io)?;
    let path = folder.join(format!("Echo v{}.dll", release.version));
    let partial = folder.join(format!("Echo v{}.dll.glint-partial", release.version));
    fs::write(&partial, bytes).map_err(GeodeError::Io)?;
    fs::rename(&partial, &path).map_err(GeodeError::Io)
}
