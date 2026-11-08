use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::gd::{GdPlatform, GdVersion};
use sha2::{Digest, Sha256};

use crate::mods::GeodeError;
use crate::mods::geode_install::{geode_dir, hex};
use crate::mods::geode_mod_version::{GeodeModVersion, fetch_geode_mod_version};
use crate::mods::geode_mods::MODS_FOLDER;
use crate::mods::geode_server::download;

const MAX_MODS: usize = 64;

pub fn install_geode_mod(
    game_dir: &Path,
    id: &str,
    gd: &GdVersion,
    platform: GdPlatform,
) -> Result<Vec<String>, GeodeError> {
    let mods_dir = geode_mods_dir(game_dir);
    let mut fetch = |id: &str, version: Option<&str>| {
        fetch_geode_mod_version(id, version, gd, platform)?.ok_or_else(|| {
            GeodeError::NoModVersion {
                id: id.to_owned(),
                gd: gd.to_string(),
            }
        })
    };
    let is_installed = |id: &str| mod_file(&mods_dir, id).is_file();
    let releases = resolve_install(id, &mut fetch, &is_installed)?;
    let mut files = Vec::new();
    for release in &releases {
        files.push((
            release.id.as_str(),
            download(&release.url)?,
            release.sha256.as_str(),
        ));
    }
    write_geode_mods(&mods_dir, &files)?;
    Ok(releases.into_iter().map(|release| release.name).collect())
}

pub(crate) fn geode_mods_dir(game_dir: &Path) -> PathBuf {
    geode_dir(game_dir).join(MODS_FOLDER)
}

pub(crate) fn resolve_install<Fetch>(
    id: &str,
    fetch: &mut Fetch,
    is_installed: &impl Fn(&str) -> bool,
) -> Result<Vec<GeodeModVersion>, GeodeError>
where
    Fetch: FnMut(&str, Option<&str>) -> Result<GeodeModVersion, GeodeError>,
{
    let mut seen = HashSet::from([id.to_owned()]);
    let mut wanted = vec![(id.to_owned(), None::<String>)];
    let mut releases = Vec::new();
    while let Some((id, version)) = wanted.pop() {
        check_id(&id)?;
        if releases.len() == MAX_MODS {
            return Err(GeodeError::Api(format!(
                "more than {MAX_MODS} dependencies"
            )));
        }
        let release = fetch(&id, version.as_deref())?;
        if release.id != id {
            return Err(GeodeError::Api(format!(
                "it answered {} when asked for {id}",
                release.id
            )));
        }
        for dependency in &release.dependencies {
            let skip = !dependency.is_required
                || dependency.id == "geode.loader"
                || is_installed(&dependency.id)
                || !seen.insert(dependency.id.clone());
            if !skip {
                wanted.push((dependency.id.clone(), exact_version(&dependency.version)));
            }
        }
        releases.push(release);
    }
    Ok(releases)
}

fn exact_version(version: &str) -> Option<String> {
    let exact = version.strip_prefix('=')?.trim().trim_start_matches('v');
    let is_plain = !exact.is_empty()
        && exact
            .chars()
            .all(|letter| letter.is_ascii_alphanumeric() || matches!(letter, '.' | '-' | '+'));
    is_plain.then(|| exact.to_owned())
}

fn check_id(id: &str) -> Result<(), GeodeError> {
    if is_mod_id(id) {
        Ok(())
    } else {
        Err(GeodeError::Api(format!("\"{id}\" isn't a mod id")))
    }
}

pub(crate) fn is_mod_id(id: &str) -> bool {
    !id.is_empty()
        && id.split('.').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|letter| letter.is_ascii_alphanumeric() || matches!(letter, '-' | '_'))
        })
}

fn mod_file(mods_dir: &Path, id: &str) -> PathBuf {
    mods_dir.join(format!("{id}.geode"))
}

pub(crate) fn write_geode_mods(
    mods_dir: &Path,
    files: &[(&str, Vec<u8>, &str)],
) -> Result<(), GeodeError> {
    for (id, bytes, sha256) in files {
        check_id(id)?;
        if hex(&Sha256::digest(bytes)) != *sha256 {
            return Err(GeodeError::Checksum);
        }
    }
    for (id, bytes, _) in files {
        write_geode_mod(mods_dir, id, bytes)?;
    }
    Ok(())
}

pub(crate) fn write_geode_mod(mods_dir: &Path, id: &str, bytes: &[u8]) -> Result<(), GeodeError> {
    fs::create_dir_all(mods_dir).map_err(GeodeError::Io)?;
    let path = mod_file(mods_dir, id);
    let partial = path.with_extension("geode.glint-partial");
    fs::write(&partial, bytes).map_err(GeodeError::Io)?;
    fs::rename(&partial, &path).map_err(GeodeError::Io)
}
