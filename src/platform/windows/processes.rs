use std::io;
use std::process::Command;

use crate::platform::hide_console;
use std::path::PathBuf;

pub(in crate::platform) fn game_running() -> io::Result<bool> {
    program_running("GeometryDash.exe")
}

pub(in crate::platform) fn steam_running() -> io::Result<bool> {
    program_running("steam.exe")
}

fn program_running(program: &str) -> io::Result<bool> {
    let filter = format!("IMAGENAME eq {program}");
    let mut command = Command::new("tasklist");
    command.args(["/FI", &filter, "/NH", "/FO", "CSV"]);
    hide_console(&mut command);
    let output = command.output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "tasklist failed ({})",
            output.status
        )));
    }
    Ok(lists_program(
        &String::from_utf8_lossy(&output.stdout),
        program,
    ))
}

fn lists_program(output: &str, program: &str) -> bool {
    output.lines().any(|line| {
        line.split(',')
            .next()
            .map(|name| name.trim().trim_matches('"'))
            .is_some_and(|name| name.eq_ignore_ascii_case(program))
    })
}

pub(in crate::platform) fn game_dirs() -> io::Result<Vec<PathBuf>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Windows game-directory detection is not implemented",
    ))
}
