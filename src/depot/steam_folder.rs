use std::env;
use std::path::PathBuf;
use std::process::Command;

pub fn steam_folder() -> Option<PathBuf> {
    steam_folders()
        .into_iter()
        .find(|folder| folder.join("steamapps").is_dir())
}

pub(crate) fn steam_folders() -> Vec<PathBuf> {
    let home = env::var_os("HOME").map(PathBuf::from);
    let mut folders = Vec::new();
    if cfg!(target_os = "windows") {
        folders.extend(registry_steam_folder());
        if let Some(program_files) = env::var_os("ProgramFiles(x86)") {
            folders.push(PathBuf::from(program_files).join("Steam"));
        }
    } else if cfg!(target_os = "macos") {
        folders.extend(home.map(|home| {
            home.join("Library")
                .join("Application Support")
                .join("Steam")
        }));
    } else if let Some(home) = home {
        folders.push(home.join(".local").join("share").join("Steam"));
        folders.push(home.join(".steam").join("steam"));
        folders.push(
            home.join(".var")
                .join("app")
                .join("com.valvesoftware.Steam")
                .join(".local")
                .join("share")
                .join("Steam"),
        );
    }
    folders
}

fn registry_steam_folder() -> Option<PathBuf> {
    if !cfg!(target_os = "windows") {
        return None;
    }
    let mut command = Command::new("reg");
    command.args(["query", r"HKCU\Software\Valve\Steam", "/v", "SteamPath"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    reg_value(&String::from_utf8_lossy(&output.stdout), "SteamPath").map(PathBuf::from)
}

fn reg_value(output: &str, name: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix(name)?;
        let value = rest.trim_start().strip_prefix("REG_SZ")?.trim();
        (!value.is_empty()).then(|| value.to_owned())
    })
}
