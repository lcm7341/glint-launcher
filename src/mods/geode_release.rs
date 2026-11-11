use std::time::Duration;

use crate::gd::{GdPlatform, GdVersion};
use serde_json::Value;

use crate::mods::GeodeError;
use crate::mods::geode_server::get_text;

const API: &str = "https://api.geode-sdk.org/v1/loader/versions/latest";

const NOT_FOUND: &str = "Latest version not found";

#[derive(Clone, PartialEq, Eq)]
pub struct GeodeRelease {
    pub version: String,
    pub prerelease: bool,
    pub url: String,
    pub sha256: String,
}

pub fn fetch_latest_geode(
    gd: &GdVersion,
    platform: GdPlatform,
) -> Result<Option<GeodeRelease>, GeodeError> {
    let release = match latest_geode(gd, platform, false)? {
        Some(release) => Some(release),
        None => latest_geode(gd, platform, true)?,
    };
    match (release, platform) {
        (Some(release), GdPlatform::MacOs) => mac_installer(release).map(Some),
        (release, _) => Ok(release),
    }
}

fn latest_geode(
    gd: &GdVersion,
    platform: GdPlatform,
    prerelease: bool,
) -> Result<Option<GeodeRelease>, GeodeError> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .new_agent();
    let mut response = agent
        .get(API)
        .query("gd", gd.to_string())
        .query("platform", download_key(platform))
        .query("prerelease", if prerelease { "true" } else { "false" })
        .header("User-Agent", "glint")
        .call()
        .map_err(|error| GeodeError::Network(error.to_string()))?;
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|error| GeodeError::Network(error.to_string()))?;
    parse_release(&body, platform)
}

fn download_key(platform: GdPlatform) -> &'static str {
    match platform {
        GdPlatform::Windows => "win",
        GdPlatform::MacOs => "mac",
    }
}

// only the pkg has the libfmod proxy
fn mac_installer(release: GeodeRelease) -> Result<GeodeRelease, GeodeError> {
    let version = release.version.trim_start_matches('v');
    let url = format!("https://api.github.com/repos/geode-sdk/geode/releases/tags/v{version}");
    let body = get_text(&url, &[])?;
    let json: Value = serde_json::from_str(&body)
        .map_err(|error| GeodeError::Api(format!("github's answer isn't JSON: {error}")))?;
    let name = format!("geode-installer-v{version}-mac.pkg");
    let asset = json["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|asset| asset["name"].as_str() == Some(name.as_str()))
        .ok_or_else(|| GeodeError::Api(format!("geode {version} has no {name}")))?;
    let field = |key: &str| asset[key].as_str().map(str::to_owned);
    let sha256 = field("digest")
        .and_then(|digest| digest.strip_prefix("sha256:").map(str::to_owned))
        .ok_or_else(|| GeodeError::Api(format!("{name} has no checksum")))?;
    Ok(GeodeRelease {
        url: field("browser_download_url")
            .ok_or_else(|| GeodeError::Api(format!("{name} has no download link")))?,
        sha256: sha256.to_lowercase(),
        ..release
    })
}

pub fn parse_release(body: &str, platform: GdPlatform) -> Result<Option<GeodeRelease>, GeodeError> {
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
    let download = &payload["downloads"][download_key(platform)];
    let field = |value: &Value, name: &str| {
        value
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| GeodeError::Api(format!("its answer has no {name}")))
    };
    Ok(Some(GeodeRelease {
        version: field(&payload["version"], "version")?,
        prerelease: payload["prerelease"].as_bool().unwrap_or(false),
        url: field(&download["url"], "download")?,
        sha256: field(&download["hash"], "checksum")?.to_lowercase(),
    }))
}
