use std::io;
use std::path::PathBuf;

pub(in crate::platform) fn game_running() -> io::Result<bool> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "macOS native game detection is not implemented",
    ))
}
pub(in crate::platform) fn steam_running() -> io::Result<bool> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "macOS host Steam detection is not implemented",
    ))
}
pub(in crate::platform) fn game_dirs() -> io::Result<Vec<PathBuf>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "macOS game-directory detection is not implemented",
    ))
}
