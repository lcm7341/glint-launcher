use std::fs;
use std::path::{Path, PathBuf};

use crate::steam::library::steam_libraries;
use crate::steam::{steam_app_install, steam_value};

const RUNTIME_ENTRY_POINT: &str = "_v2-entry-point";

const SYSTEM_TOOLS: &str = "/usr/share/steam/compatibilitytools.d";

// a proton steam would run this app with, and the runtime that proton needs
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CompatTool {
    pub proton: PathBuf,
    pub runtime: Option<PathBuf>,
}

// the tool picked for this app in steam's compatibility settings, if it's installed.
// sharing a prefix with steam but not its proton breaks mods like mega hack
pub fn steam_compat_tool(steam_folder: &Path, app_id: u32) -> Option<CompatTool> {
    let config = fs::read_to_string(steam_folder.join("config").join("config.vdf")).ok()?;
    let name = mapped_tool(&config, app_id)?;
    let proton = find_tool(steam_folder, &name)?;
    let runtime = required_runtime(steam_folder, &proton);
    Some(CompatTool { proton, runtime })
}

fn mapped_tool(config: &str, app_id: u32) -> Option<String> {
    let mapping = block(config, "CompatToolMapping")?;
    let app = block(mapping, &app_id.to_string())?;
    steam_value(app, "name").filter(|name| !name.is_empty())
}

fn find_tool(steam_folder: &Path, name: &str) -> Option<PathBuf> {
    let custom = [
        steam_folder.join("compatibilitytools.d"),
        PathBuf::from(SYSTEM_TOOLS),
    ];
    let custom = custom
        .iter()
        .flat_map(|folder| subfolders(folder))
        .find(|tool| {
            fs::read_to_string(tool.join("compatibilitytool.vdf"))
                .is_ok_and(|manifest| block(&manifest, name).is_some())
        });
    custom
        .or_else(|| {
            steam_libraries(steam_folder)
                .iter()
                .flat_map(|library| subfolders(&library.join("steamapps").join("common")))
                .find(|tool| builtin_name(tool).is_some_and(|found| found == name))
        })
        .filter(|tool| tool.join("proton").is_file())
}

// steam's own protons only say their version: "proton-9.0-4f" is proton_9, 6.3 is proton_63
fn builtin_name(tool: &Path) -> Option<String> {
    let version = fs::read_to_string(tool.join("version")).ok()?;
    let release = version.split_whitespace().nth(1)?;
    if release.starts_with("experimental-") {
        return Some("proton_experimental".to_owned());
    }
    if release.starts_with("hotfix-") {
        return Some("proton_hotfix".to_owned());
    }
    let number = release.strip_prefix("proton-")?.split('-').next()?;
    let (major, minor) = number.split_once('.')?;
    Some(match minor {
        "0" => format!("proton_{major}"),
        _ => format!("proton_{major}{minor}"),
    })
}

fn required_runtime(steam_folder: &Path, proton: &Path) -> Option<PathBuf> {
    let manifest = fs::read_to_string(proton.join("toolmanifest.vdf")).ok()?;
    let app_id = steam_value(&manifest, "require_tool_appid")?.parse().ok()?;
    let runtime = steam_app_install(steam_folder, app_id)?;
    runtime
        .join(RUNTIME_ENTRY_POINT)
        .is_file()
        .then_some(runtime)
}

fn subfolders(folder: &Path) -> Vec<PathBuf> {
    fs::read_dir(folder)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .collect()
        })
        .unwrap_or_default()
}

// the text between the braces after "key", at any depth
fn block<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let quoted = format!("\"{key}\"");
    let mut from = 0;
    while let Some(found) = text[from..].find(&quoted) {
        let after = from + found + quoted.len();
        if let Some(inside) = skip_comments(&text[after..]).strip_prefix('{') {
            let start = text.len() - inside.len();
            let mut depth = 1;
            for (index, letter) in inside.char_indices() {
                match letter {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
                if depth == 0 {
                    return Some(&text[start..start + index]);
                }
            }
            return None;
        }
        from = after;
    }
    None
}

fn skip_comments(mut text: &str) -> &str {
    loop {
        text = text.trim_start();
        match text.strip_prefix("//") {
            Some(comment) => text = comment.split_once('\n').map_or("", |(_, rest)| rest),
            None => return text,
        }
    }
}

