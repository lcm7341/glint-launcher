use std::path::Path;
use std::process::Command;

use crate::gd::GD_APP_ID;

pub const MAC_PROGRAM: &str = "Geometry Dash.app/Contents/MacOS/Geometry Dash";

// CFFIXED_USER_HOME moves where gd saves
pub fn mac_game_command(game_dir: &Path, home: Option<&Path>) -> Command {
    let mut command = Command::new(game_dir.join(MAC_PROGRAM));
    command
        .current_dir(game_dir)
        .env("SteamAppId", GD_APP_ID.to_string());
    if let Some(home) = home {
        command.env("CFFIXED_USER_HOME", home);
    }
    command
}
