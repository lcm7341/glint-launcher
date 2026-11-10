use std::path::Path;
use std::process::Command;

use crate::gd::GD_APP_ID;

use crate::gd::WINDOWS_PROGRAM;
use std::io;

pub(in crate::platform) fn command(game_dir: &Path, _home: Option<&Path>) -> Command {
    let mut command = Command::new(game_dir.join(WINDOWS_PROGRAM));
    command
        .current_dir(game_dir)
        .env("SteamAppId", GD_APP_ID.to_string());
    command
}

pub(in crate::platform) fn prepare(_game_dir: &Path, _home: Option<&Path>) -> io::Result<()> {
    Ok(())
}
