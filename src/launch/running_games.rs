use std::fs;
use std::path::{Path, PathBuf};

// under wine the exe shows up as Z:\path\to\GeometryDash.exe
pub fn running_game_dirs() -> Vec<PathBuf> {
    let Ok(processes) = fs::read_dir("/proc") else {
        return Vec::new();
    };
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
    found
}

pub fn is_running_in(running: &[PathBuf], game_dir: &Path) -> bool {
    let wanted = fs::canonicalize(game_dir).unwrap_or_else(|_| game_dir.to_owned());
    running
        .iter()
        .any(|dir| fs::canonicalize(dir).unwrap_or_else(|_| dir.clone()) == wanted)
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
