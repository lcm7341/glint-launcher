mod game_process;
mod launch_request;
mod mac_game;
mod running_games;
mod windows_game;
mod windows_running;

pub use game_process::GameProcess;
pub use launch_request::{LaunchRequest, PROTON_NAME, find_proton, find_runtime};
pub use mac_game::{MAC_PROGRAM, mac_game_command};
pub use running_games::{is_running_in, running_game_dirs};
pub use windows_game::{WINDOWS_PROGRAM, windows_game_command};
pub use windows_running::{is_steam_running, windows_gd_running};
