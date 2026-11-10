use std::path::Path;
use std::process::Command;

use crate::platform::PickKind;

pub(in crate::platform) const PROGRAMS: &[&str] = &["kdialog", "zenity"];

pub(in crate::platform) fn command(program: &str, pick: PickKind, start: &Path) -> Command {
    let mut command = Command::new(program);
    if program == "kdialog" {
        match pick {
            PickKind::Folder => {
                command.arg("--getexistingdirectory").arg(start);
            }
            PickKind::DllFile => {
                command
                    .arg("--getopenfilename")
                    .arg(start)
                    .arg("DLL mods (*.dll)");
            }
            PickKind::GeodeFile => {
                command
                    .arg("--getopenfilename")
                    .arg(start)
                    .arg("Geode mods (*.geode)");
            }
        }
    } else {
        // Zenity expects a filename option, including for a directory's initial location.
        let mut filename = std::ffi::OsString::from("--filename=");
        filename.push(start);
        filename.push("/");
        command.arg("--file-selection").arg(filename);
        match pick {
            PickKind::Folder => {
                command.arg("--directory");
            }
            PickKind::DllFile => {
                command.arg("--file-filter=DLL mods | *.dll *.DLL");
            }
            PickKind::GeodeFile => {
                command.arg("--file-filter=Geode mods | *.geode *.GEODE");
            }
        }
    }
    command
}
