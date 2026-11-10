use std::io;
use std::path::PathBuf;
use std::process::Command;

pub(in crate::platform) fn game_running() -> io::Result<bool> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "macOS native game detection is not implemented",
    ))
}
pub(in crate::platform) fn steam_running() -> io::Result<bool> {
    let output = Command::new("ps").args(["-axo", "comm="]).output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!("ps failed ({})", output.status)));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|program| program.rsplit('/').next() == Some("steam_osx")))
}
pub(in crate::platform) fn game_dirs() -> io::Result<Vec<PathBuf>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "macOS game-directory detection is not implemented",
    ))
}
