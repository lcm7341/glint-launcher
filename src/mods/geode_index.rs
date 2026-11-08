use crate::gd::{GdPlatform, GdVersion};
use serde_json::Value;

use crate::mods::GeodeError;
use crate::mods::geode_server::{API, get_text, server_gd, server_platform};

pub const INDEX_PAGE_SIZE: u32 = 20;

#[derive(Clone, PartialEq, Eq)]
pub struct IndexMod {
    pub id: String,
    pub name: String,
    pub developers: Vec<String>,
    pub description: Option<String>,
    pub version: String,
    pub downloads: u64,
}

#[derive(Clone, Default, PartialEq, Eq)]
pub struct IndexPage {
    pub mods: Vec<IndexMod>,
    pub total: u64,
}

pub fn search_geode_index(
    gd: &GdVersion,
    platform: GdPlatform,
    query: &str,
    page: u32,
) -> Result<IndexPage, GeodeError> {
    let mut parameters = vec![
        ("gd", server_gd(gd)),
        ("platforms", server_platform(platform).to_owned()),
        ("page", page.to_string()),
        ("per_page", INDEX_PAGE_SIZE.to_string()),
        ("sort", "downloads".to_owned()),
    ];
    let query = query.trim();
    if !query.is_empty() {
        parameters.push(("query", query.to_owned()));
    }
    let body = get_text(&format!("{API}/mods"), &parameters)?;
    parse_index_page(&body)
}

pub(crate) fn parse_index_page(body: &str) -> Result<IndexPage, GeodeError> {
    let json: Value = serde_json::from_str(body)
        .map_err(|error| GeodeError::Api(format!("its answer isn't JSON: {error}")))?;
    let error = json["error"].as_str().unwrap_or_default();
    if !error.is_empty() {
        return Err(GeodeError::Api(error.to_owned()));
    }
    let payload = &json["payload"];
    let Some(data) = payload["data"].as_array() else {
        return Err(GeodeError::Api("its answer has no list of mods".to_owned()));
    };
    let mods = data.iter().filter_map(index_mod).collect();
    Ok(IndexPage {
        mods,
        total: payload["count"].as_u64().unwrap_or_default(),
    })
}

fn index_mod(entry: &Value) -> Option<IndexMod> {
    let release = entry["versions"].as_array()?.first()?;
    let text = |value: &Value| value.as_str().map(str::to_owned);
    let developers = entry["developers"]
        .as_array()
        .map(|developers| {
            developers
                .iter()
                .filter_map(|developer| {
                    text(&developer["display_name"]).or_else(|| text(&developer["username"]))
                })
                .collect()
        })
        .unwrap_or_default();
    let id = text(&entry["id"])?;
    Some(IndexMod {
        name: text(&release["name"]).unwrap_or_else(|| id.clone()),
        id,
        developers,
        description: text(&release["description"]).filter(|text| !text.trim().is_empty()),
        version: text(&release["version"])?,
        downloads: entry["download_count"].as_u64().unwrap_or_default(),
    })
}
