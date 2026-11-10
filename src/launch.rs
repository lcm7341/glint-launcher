mod plan;
pub(crate) use plan::{LaunchBlocker, LaunchFacts, LaunchReadiness, LaunchRoute, WineStatus};

mod game_process;
mod proton;
mod running_games;
mod steam_drm;

pub use game_process::GameProcess;
pub use proton::{LaunchRequest, PROTON_NAME, find_proton, find_runtime};
pub use running_games::is_running_in;
pub use steam_drm::has_steam_drm;
mod wine;
pub(crate) use wine::{
    INSTALL_ROSETTA, SetupStep, WINE_VERSION, WineSteam, glint_wine, needs_rosetta, setup_wine,
    wine_steam_running,
};

// without a native override proton loads its own xinput and the loader never runs
fn dll_overrides(game_dir: &std::path::Path) -> String {
    let mut dlls: Vec<String> = std::fs::read_dir(game_dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|entry| entry.file_name().to_str().map(str::to_lowercase))
                .filter_map(|file| {
                    let dll = file.strip_suffix(".dll")?;
                    dll.starts_with("xinput").then(|| dll.to_owned())
                })
                .collect()
        })
        .unwrap_or_default();
    dlls.sort();
    dlls.iter()
        .map(|dll| format!("{dll}=n,b"))
        .collect::<Vec<_>>()
        .join(";")
}
