use crate::gd::{GdPlatform, GdVersion};
use serde_json::Value;

use crate::mods::GeodeError;
use crate::mods::geode_server::{API, get_text, server_gd, server_platform};

const NOT_FOUND: &str = "Couldn't find valid mod version for given filters";

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct GeodeModVersion {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) url: String,
    pub(crate) sha256: String,
    pub(crate) dependencies: Vec<ModDependency>,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ModDependency {
    pub(crate) id: String,
    pub(crate) version: String,
    pub(crate) is_required: bool,
}

pub(crate) fn fetch_geode_mod_version(
    id: &str,
    version: Option<&str>,
    gd: &GdVersion,
    platform: GdPlatform,
) -> Result<Option<GeodeModVersion>, GeodeError> {
    let body = match version {
        Some(version) => get_text(&format!("{API}/mods/{id}/versions/{version}"), &[])?,
        None => get_text(
            &format!("{API}/mods/{id}/versions/latest"),
            &[
                ("gd", server_gd(gd)),
                ("platforms", server_platform(platform).to_owned()),
            ],
        )?,
    };
    parse_mod_version(&body)
}

pub(crate) fn parse_mod_version(body: &str) -> Result<Option<GeodeModVersion>, GeodeError> {
    let json: Value = serde_json::from_str(body)
        .map_err(|error| GeodeError::Api(format!("its answer isn't JSON: {error}")))?;
    let error = json["error"].as_str().unwrap_or_default();
    if error == NOT_FOUND {
        return Ok(None);
    }
    if !error.is_empty() {
        return Err(GeodeError::Api(error.to_owned()));
    }
    let payload = &json["payload"];
    let field = |name: &str| {
        payload[name]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| GeodeError::Api(format!("its answer has no {name}")))
    };
    let dependencies = payload["dependencies"]
        .as_array()
        .map(|dependencies| dependencies.iter().filter_map(dependency).collect())
        .unwrap_or_default();
    Ok(Some(GeodeModVersion {
        id: field("mod_id")?,
        name: field("name")?,
        version: field("version")?,
        url: field("download_link")?,
        sha256: field("hash")?.to_lowercase(),
        dependencies,
    }))
}

fn dependency(entry: &Value) -> Option<ModDependency> {
    Some(ModDependency {
        id: entry["mod_id"].as_str()?.to_owned(),
        version: entry["version"].as_str().unwrap_or_default().to_owned(),
        is_required: entry["required"]
            .as_bool()
            .unwrap_or_else(|| entry["importance"] == "required"),
    })
}
