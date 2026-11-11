use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::time::Duration;

use crate::launch::WineSteam;
use crate::launch::wine::distribution::{WINE_VERSION, fetch_wine, glint_wine};
use crate::launch::wine::steam::STEAM_SETUP_FILE;

const STEAM_SETUP_URL: &str =
    "https://cdn.cloudflare.steamstatic.com/client/installer/SteamSetup.exe";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SetupStep {
    DownloadingWine,
    MakingWineFolder,
    DownloadingSteam,
    InstallingSteam,
    StartingSteam,
}

impl SetupStep {
    pub fn describe(self) -> &'static str {
        match self {
            SetupStep::DownloadingWine => "Downloading Wine",
            SetupStep::MakingWineFolder => "Making Glint's Wine folder",
            SetupStep::DownloadingSteam => "Downloading Steam for Windows",
            SetupStep::InstallingSteam => "Installing Steam for Windows",
            SetupStep::StartingSteam => "Starting Steam for Windows",
        }
    }
}

/// downloaded bytes, total if known
pub type Progress<'a> = &'a dyn Fn(u64, Option<u64>);

pub fn setup_wine(
    tools_dir: &Path,
    prefix: &Path,
    step: &dyn Fn(SetupStep),
    progress: Progress<'_>,
) -> Result<(), String> {
    let wine = match glint_wine(tools_dir) {
        Some(wine) => wine,
        None => {
            step(SetupStep::DownloadingWine);
            fetch_wine(tools_dir, progress)
                .map_err(|error| format!("Couldn't download Wine {WINE_VERSION}: {error}"))?
        }
    };
    set_up_steam(&WineSteam::new(wine, prefix), step, progress)
}

fn set_up_steam(
    wine_steam: &WineSteam,
    step: &dyn Fn(SetupStep),
    progress: Progress<'_>,
) -> Result<(), String> {
    if !wine_steam.has_prefix() {
        step(SetupStep::MakingWineFolder);
        wine_steam
            .create_prefix()
            .map_err(|error| format!("Couldn't make Glint's Wine folder: {error}"))?;
    }
    step(SetupStep::DownloadingSteam);
    let installer = wine_steam.prefix().join("drive_c").join(STEAM_SETUP_FILE);
    download(STEAM_SETUP_URL, &installer, progress)
        .map_err(|error| format!("Couldn't download Steam for Windows: {error}"))?;
    step(SetupStep::InstallingSteam);
    let installed = wine_steam.run_steam_setup();
    let _ = fs::remove_file(&installer);
    installed.map_err(|error| format!("Couldn't install Steam for Windows: {error}"))?;
    if !wine_steam.is_steam_installed() {
        return Err("Steam installer finished but Steam isn't there".to_owned());
    }
    step(SetupStep::StartingSteam);
    wine_steam
        .start_steam()
        .map_err(|error| format!("Couldn't start Steam for Windows: {error}"))
}

fn download(url: &str, path: &Path, progress: Progress<'_>) -> Result<(), String> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(300)))
        .build()
        .new_agent();
    let mut response = agent
        .get(url)
        .header("User-Agent", "glint")
        .call()
        .map_err(|error| error.to_string())?;
    let total = response.body().content_length();
    let mut body = response
        .body_mut()
        .with_config()
        .limit(32 * 1024 * 1024)
        .reader();
    let mut bytes = Vec::new();
    let mut buffer = vec![0; 1 << 16];
    loop {
        let read = match body.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.to_string()),
        };
        bytes.extend_from_slice(&buffer[..read]);
        progress(bytes.len() as u64, total);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let partial = path.with_extension("part");
    fs::write(&partial, bytes).map_err(|error| error.to_string())?;
    fs::rename(&partial, path).map_err(|error| error.to_string())
}
