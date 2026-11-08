use std::path::Path;
use std::process::Command;

use crate::gd::GD_APP_ID;

pub const WINDOWS_PROGRAM: &str = "GeometryDash.exe";

pub fn windows_game_command(game_dir: &Path) -> Command {
    let mut command = Command::new(game_dir.join(WINDOWS_PROGRAM));
    command
        .current_dir(game_dir)
        .env("SteamAppId", GD_APP_ID.to_string());
    command
}
