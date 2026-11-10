use std::fs;
use std::io;
use std::path::{Path, PathBuf};

// under wine the exe shows up as Z:\path\to\GeometryDash.exe
pub(in crate::platform) fn game_dirs() -> io::Result<Vec<PathBuf>> {
    let processes = fs::read_dir("/proc")?;
    let mut found = Vec::new();
    for process in processes.flatten() {
        let folder = process.path();
        let Ok(command_line) = fs::read(folder.join("cmdline")) else {
            continue;
        };
        let Some(exe) = game_exe(&command_line) else {
            continue;
        };
        let game_dir = match unix_parent(&exe) {
            Some(parent) => Some(parent),
            None => fs::read_link(folder.join("cwd")).ok(),
        };
        if let Some(game_dir) = game_dir
            && !found.contains(&game_dir)
        {
            found.push(game_dir);
        }
    }
    Ok(found)
}

fn game_exe(command_line: &[u8]) -> Option<String> {
    let program = command_line.split(|&byte| byte == 0).next()?;
    let program = String::from_utf8_lossy(program);
    let file = program.rsplit(['\\', '/']).next()?;
    file.eq_ignore_ascii_case("GeometryDash.exe")
        .then(|| program.into_owned())
}

fn unix_parent(exe: &str) -> Option<PathBuf> {
    let unix = if let Some(rest) = exe.strip_prefix("Z:").or_else(|| exe.strip_prefix("z:")) {
        rest.replace('\\', "/")
    } else if exe.starts_with('/') {
        exe.to_owned()
    } else {
        return None;
    };
    Path::new(&unix).parent().map(Path::to_path_buf)
}

pub(in crate::platform) fn game_running() -> io::Result<bool> {
    game_dirs().map(|dirs| !dirs.is_empty())
}
pub(in crate::platform) fn steam_running() -> io::Result<bool> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Linux Steam process detection is not implemented",
    ))
}
