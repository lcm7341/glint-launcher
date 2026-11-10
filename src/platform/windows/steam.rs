use std::path::PathBuf;
use std::process::Command;

use crate::platform::{hide_console, non_empty_env};

pub(in crate::platform) fn folders() -> Vec<PathBuf> {
    let mut folders: Vec<_> = registry_folder().into_iter().collect();
    if let Some(root) = non_empty_env("ProgramFiles(x86)") {
        folders.push(root.join("Steam"));
    }
    folders
}

fn registry_folder() -> Option<PathBuf> {
    let mut command = Command::new("reg");
    command.args(["query", r"HKCU\Software\Valve\Steam", "/v", "SteamPath"]);
    hide_console(&mut command);
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    registry_value(&String::from_utf8_lossy(&output.stdout), "SteamPath").map(PathBuf::from)
}

fn registry_value(output: &str, name: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix(name)?;
        let value = rest.trim_start().strip_prefix("REG_SZ")?.trim();
        (!value.is_empty()).then(|| value.to_owned())
    })
}
