use std::time::Duration;

use serde_json::Value;

use super::{ASSET, REPO, UpdateError, Version};

/// The newest published release and its file for this computer
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Release {
    pub(crate) version: Version,
    pub(crate) page: String,
    pub(crate) url: String,
    pub(crate) sha256: String,
    pub(crate) size: u64,
}

pub(crate) fn fetch_latest() -> Result<Release, UpdateError> {
    let Some(asset) = ASSET else {
        return Err(UpdateError::Unsupported(
            "there are no glint builds for this computer".to_owned(),
        ));
    };
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .new_agent();
    let mut response = agent
        .get(format!(
            "https://api.github.com/repos/{REPO}/releases/latest"
        ))
        .header("User-Agent", "glint")
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .call()
        .map_err(|error| UpdateError::Network(error.to_string()))?;
    let status = response.status().as_u16();
    // private repos look like missing ones without a token
    if status == 404 {
        return Err(UpdateError::Api(format!("{REPO} has no public releases")));
    }
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|error| UpdateError::Network(error.to_string()))?;
    if status != 200 {
        let message = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|json| json["message"].as_str().map(str::to_owned))
            .unwrap_or_default();
        return Err(UpdateError::Api(format!("status {status} {message}")));
    }
    parse_release(&body, asset)
}

pub(crate) fn parse_release(body: &str, asset: &str) -> Result<Release, UpdateError> {
    let json: Value = serde_json::from_str(body)
        .map_err(|error| UpdateError::Api(format!("its answer isn't JSON: {error}")))?;
    let tag = json["tag_name"]
        .as_str()
        .ok_or_else(|| UpdateError::Api("the release has no tag".to_owned()))?;
    let version = tag
        .parse()
        .map_err(|error| UpdateError::Api(format!("the release tag: {error}")))?;
    let file = json["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|file| file["name"].as_str() == Some(asset))
        .ok_or_else(|| UpdateError::Api(format!("release {tag} has no {asset}")))?;
    let url = file["browser_download_url"]
        .as_str()
        .ok_or_else(|| UpdateError::Api(format!("{asset} has no download link")))?;
    // github hashes every uploaded asset as "sha256:<hex>"
    let sha256 = file["digest"]
        .as_str()
        .and_then(|digest| digest.strip_prefix("sha256:"))
        .filter(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| UpdateError::Api(format!("{asset} has no checksum")))?;
    Ok(Release {
        version,
        page: json["html_url"].as_str().unwrap_or_default().to_owned(),
        url: url.to_owned(),
        sha256: sha256.to_ascii_lowercase(),
        size: file["size"].as_u64().unwrap_or(0),
    })
}
