use std::path::Path;
use std::process::Command;

use crate::gd::GD_APP_ID;

use std::{fs, io};

pub(in crate::platform) const MAC_PROGRAM: &str = "Geometry Dash.app/Contents/MacOS/Geometry Dash";

// CFFIXED_USER_HOME sets the save folder
pub(in crate::platform) fn command(game_dir: &Path, home: Option<&Path>) -> Command {
    let mut command = Command::new(game_dir.join(MAC_PROGRAM));
    command
        .current_dir(game_dir)
        .env("SteamAppId", GD_APP_ID.to_string());
    if let Some(home) = home {
        command.env("CFFIXED_USER_HOME", home);
    }
    command
}

pub(in crate::platform) fn prepare(game_dir: &Path, home: Option<&Path>) -> io::Result<()> {
    if let Some(home) = home {
        make_runnable(&game_dir.join(MAC_PROGRAM))?;
        fs::create_dir_all(home)?;
    }
    Ok(())
}
#[cfg(unix)]
fn make_runnable(program: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(program)?.permissions();
    if permissions.mode() & 0o111 == 0 {
        permissions.set_mode(permissions.mode() | 0o111);
        fs::set_permissions(program, permissions)?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn make_runnable(_program: &Path) -> io::Result<()> {
    Ok(())
}
