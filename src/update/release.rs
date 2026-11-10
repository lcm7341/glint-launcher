use std::time::Duration;

use serde_json::Value;

use super::{ASSET, MANIFEST_URL, UpdateError, Version};

/// The newest published release and its file for this computer
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Release {
    pub(crate) version: Version,
    pub(crate) url: String,
    pub(crate) sha256: String,
    pub(crate) size: u64,
}

pub(crate) fn fetch_latest() -> Result<Release, UpdateError> {
    let Some(asset) = ASSET else {
        return Err(UpdateError::Unsupported(
            "there are no Glint builds for this computer".to_owned(),
        ));
    };
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .new_agent();
    let mut response = agent
        .get(MANIFEST_URL)
        .header("User-Agent", "glint")
        .call()
        .map_err(|error| UpdateError::Network(error.to_string()))?;
    let status = response.status().as_u16();
    if status == 404 {
        return Err(UpdateError::Api(
            "echogd.com has no Glint release".to_owned(),
        ));
    }
    if status != 200 {
        return Err(UpdateError::Api(format!("status {status}")));
    }
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|error| UpdateError::Network(error.to_string()))?;
    parse_release(&body, asset)
}

/// Each file has its own version, since a release can skip a platform
pub(crate) fn parse_release(body: &str, asset: &str) -> Result<Release, UpdateError> {
    let json: Value = serde_json::from_str(body)
        .map_err(|error| UpdateError::Api(format!("its answer isn't JSON: {error}")))?;
    let file = &json["files"][asset];
    if file.is_null() {
        return Err(UpdateError::Api(format!("the release has no {asset}")));
    }
    let field = |name: &str| {
        file[name]
            .as_str()
            .ok_or_else(|| UpdateError::Api(format!("{asset} has no {name}")))
    };
    let version = field("version")?
        .parse()
        .map_err(|error| UpdateError::Api(format!("{asset}'s version: {error}")))?;
    let sha256 = field("sha256")?;
    if sha256.len() != 64 || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(UpdateError::Api(format!("{asset} has a bad checksum")));
    }
    Ok(Release {
        version,
        url: field("url")?.to_owned(),
        sha256: sha256.to_ascii_lowercase(),
        size: file["size"].as_u64().unwrap_or(0),
    })
}
