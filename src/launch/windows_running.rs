use std::io;
use std::process::Command;

use crate::launch::WINDOWS_PROGRAM;

pub fn windows_gd_running() -> io::Result<bool> {
    program_running(WINDOWS_PROGRAM)
}

pub fn is_steam_running() -> io::Result<bool> {
    program_running("steam.exe")
}

fn program_running(program: &str) -> io::Result<bool> {
    let filter = format!("IMAGENAME eq {program}");
    let mut command = Command::new("tasklist");
    command.args(["/FI", &filter, "/NH", "/FO", "CSV"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
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
