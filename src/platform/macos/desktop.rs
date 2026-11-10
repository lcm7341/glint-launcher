use std::path::Path;
use std::process::Command;

pub(in crate::platform) fn open_command(target: &Path) -> Command {
    let mut command = Command::new("open");
    command.arg(target);
    command
}
