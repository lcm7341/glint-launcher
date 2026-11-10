use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::gd::GD_APP_ID;

pub const PROTON_NAME: &str = "Proton - Experimental";

pub const RUNTIME_NAME: &str = "SteamLinuxRuntime_4";

// gd quits if the steam api cant find steam
#[derive(Clone, PartialEq, Eq)]
pub struct LaunchRequest {
    proton: PathBuf,
    steam_folder: PathBuf,
    game_dir: PathBuf,
    compat_dir: PathBuf,
    runtime: Option<PathBuf>,
}

impl LaunchRequest {
    pub fn new(
        proton: impl Into<PathBuf>,
        steam_folder: impl Into<PathBuf>,
        game_dir: impl Into<PathBuf>,
        compat_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            proton: proton.into(),
            steam_folder: steam_folder.into(),
            game_dir: game_dir.into(),
            compat_dir: compat_dir.into(),
            runtime: None,
        }
    }

    // outside the runtime fullscreen gd loses focus on every click
    #[must_use]
    pub fn with_runtime(mut self, runtime: Option<PathBuf>) -> Self {
        self.runtime = runtime;
        self
    }

    pub fn game_dir(&self) -> &Path {
        &self.game_dir
    }

    pub fn compat_dir(&self) -> &Path {
        &self.compat_dir
    }

    pub fn to_command(&self) -> Command {
        let proton = self.proton.join("proton");
        let program = Path::new(".").join("GeometryDash.exe");
        let mut command = match &self.runtime {
            Some(runtime) => {
                let mut command = Command::new(runtime.join(RUNTIME_ENTRY_POINT));
                command
                    .arg("--verb=waitforexitandrun")
                    .arg("--")
                    .arg(proton)
                    .arg("waitforexitandrun")
                    .arg(program);
                command
            }
            None => {
                let mut command = Command::new(proton);
                command.arg("run").arg(program);
                command
            }
        };
        command.current_dir(&self.game_dir);
        for (key, value) in self.environment() {
            command.env(key, value);
        }
        command
    }

    pub fn environment(&self) -> Vec<(&'static str, OsString)> {
        let app_id = OsString::from(GD_APP_ID.to_string());
        let mut environment = vec![
            (
                "STEAM_COMPAT_DATA_PATH",
                self.compat_dir.clone().into_os_string(),
            ),
            (
                "STEAM_COMPAT_CLIENT_INSTALL_PATH",
                self.steam_folder.clone().into_os_string(),
            ),
            ("SteamAppId", app_id.clone()),
            ("SteamGameId", app_id.clone()),
            ("STEAM_COMPAT_APP_ID", app_id),
        ];
        let overrides = super::dll_overrides(&self.game_dir);
        if !overrides.is_empty() {
            environment.push(("WINEDLLOVERRIDES", OsString::from(overrides)));
        }
        environment
    }
}

const RUNTIME_ENTRY_POINT: &str = "_v2-entry-point";

pub fn find_runtime(steam_folder: &Path) -> Option<PathBuf> {
    let runtime = steam_folder
        .join("steamapps")
        .join("common")
        .join(RUNTIME_NAME);
    runtime
        .join(RUNTIME_ENTRY_POINT)
        .is_file()
        .then_some(runtime)
}

pub fn find_proton(steam_folder: &Path) -> Option<PathBuf> {
    let proton = steam_folder
        .join("steamapps")
        .join("common")
        .join(PROTON_NAME);
    proton.join("proton").is_file().then_some(proton)
}
