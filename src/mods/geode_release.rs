use std::time::Duration;

use crate::gd::GdVersion;
use serde_json::Value;

use crate::mods::GeodeError;

const API: &str = "https://api.geode-sdk.org/v1/loader/versions/latest";

const NOT_FOUND: &str = "Latest version not found";

#[derive(Clone, PartialEq, Eq)]
pub struct GeodeRelease {
    pub version: String,
    pub prerelease: bool,
    pub url: String,
    pub sha256: String,
}

pub fn fetch_latest_geode(gd: &GdVersion) -> Result<Option<GeodeRelease>, GeodeError> {
    match latest_geode(gd, false)? {
        Some(release) => Ok(Some(release)),
        None => latest_geode(gd, true),
    }
}

fn latest_geode(gd: &GdVersion, prerelease: bool) -> Result<Option<GeodeRelease>, GeodeError> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .new_agent();
    let mut response = agent
        .get(API)
        .query("gd", gd.to_string())
        .query("platform", "win")
        .query("prerelease", if prerelease { "true" } else { "false" })
        .header("User-Agent", "glint")
        .call()
        .map_err(|error| GeodeError::Network(error.to_string()))?;
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|error| GeodeError::Network(error.to_string()))?;
    parse_release(&body)
}

pub fn parse_release(body: &str) -> Result<Option<GeodeRelease>, GeodeError> {
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
    let windows = &payload["downloads"]["win"];
    let field = |value: &Value, name: &str| {
        value
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| GeodeError::Api(format!("its answer has no {name}")))
    };
    Ok(Some(GeodeRelease {
        version: field(&payload["version"], "version")?,
        prerelease: payload["prerelease"].as_bool().unwrap_or(false),
        url: field(&windows["url"], "Windows download")?,
        sha256: field(&windows["hash"], "checksum")?.to_lowercase(),
    }))
}
