use std::time::Duration;

use crate::gd::{GdPlatform, GdVersion};

use crate::mods::GeodeError;

pub(crate) const API: &str = "https://api.geode-sdk.org/v1";

pub(crate) const MAX_MOD_BYTES: u64 = 200 * 1024 * 1024;

pub(crate) fn get_text(url: &str, query: &[(&str, String)]) -> Result<String, GeodeError> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .new_agent();
    let mut request = agent.get(url).header("User-Agent", "glint");
    for (name, value) in query {
        request = request.query(*name, value);
    }
    let mut response = request
        .call()
        .map_err(|error| GeodeError::Network(error.to_string()))?;
    response
        .body_mut()
        .read_to_string()
        .map_err(|error| GeodeError::Network(error.to_string()))
}

pub(crate) fn download(url: &str) -> Result<Vec<u8>, GeodeError> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(300)))
        .build()
        .new_agent();
    let mut response = agent
        .get(url)
        .header("User-Agent", "glint")
        .call()
        .map_err(|error| GeodeError::Network(error.to_string()))?;
    response
        .body_mut()
        .with_config()
        .limit(MAX_MOD_BYTES)
        .read_to_vec()
        .map_err(|error| GeodeError::Network(error.to_string()))
}

// the server calls 2.2 2.200
pub(crate) fn server_gd(gd: &GdVersion) -> String {
    match gd.to_string().as_str() {
        "2.2" => "2.200".to_owned(),
        other => other.to_owned(),
    }
}

pub(crate) fn server_platform(platform: GdPlatform) -> &'static str {
    match platform {
        GdPlatform::Windows => "win",
        GdPlatform::MacOs if cfg!(target_arch = "aarch64") => "mac-arm",
        GdPlatform::MacOs => "mac-intel",
    }
}
