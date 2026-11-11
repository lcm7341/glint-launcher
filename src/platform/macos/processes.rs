use std::io;
use std::path::PathBuf;
use std::process::Command;

const GAME_PROGRAM: &str = "/Geometry Dash.app/Contents/MacOS/Geometry Dash";

pub(in crate::platform) fn game_running() -> io::Result<bool> {
    game_dirs().map(|dirs| !dirs.is_empty())
}

pub(in crate::platform) fn steam_running() -> io::Result<bool> {
    Ok(programs()?
        .lines()
        .any(|program| program.rsplit('/').next() == Some("steam_osx")))
}

pub(in crate::platform) fn game_dirs() -> io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    for program in programs()?.lines() {
        let Some(at) = program.find(GAME_PROGRAM) else {
            continue;
        };
        let dir = PathBuf::from(&program[..at]);
        if !found.contains(&dir) {
            found.push(dir);
        }
    }
    Ok(found)
}

// full executable paths
fn programs() -> io::Result<String> {
    let output = Command::new("ps").args(["-axo", "comm="]).output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!("ps failed ({})", output.status)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
