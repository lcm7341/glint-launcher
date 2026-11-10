use std::path::Path;
use std::process::Command;

use crate::platform::PickKind;

pub(in crate::platform) const PROGRAMS: &[&str] = &["osascript"];

const MAC_PICK_FOLDER: &str = "on run argv
activate
POSIX path of (choose folder with prompt \"Pick a Geometry Dash folder\" default location (POSIX file (item 1 of argv)))
end run";
const MAC_PICK_DLL: &str = "on run argv
activate
POSIX path of (choose file with prompt \"Pick a mod DLL\" of type {\"dll\"} default location (POSIX file (item 1 of argv)))
end run";
const MAC_PICK_GEODE: &str = "on run argv
activate
POSIX path of (choose file with prompt \"Pick a Geode mod\" of type {\"geode\"} default location (POSIX file (item 1 of argv)))
end run";

pub(in crate::platform) fn command(program: &str, pick: PickKind, start: &Path) -> Command {
    let script = match pick {
        PickKind::Folder => MAC_PICK_FOLDER,
        PickKind::DllFile => MAC_PICK_DLL,
        PickKind::GeodeFile => MAC_PICK_GEODE,
    };
    let mut command = Command::new(program);
    command.arg("-e").arg(script).arg(start);
    command
}
