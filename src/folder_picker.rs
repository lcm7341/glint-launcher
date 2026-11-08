use crate::job::Job;
use crate::launcher::Notify;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Default)]
pub(crate) struct FolderPicker {
    picking: Option<Job<Option<PathBuf>>>,
}

const PICKERS: &[&str] = &["kdialog", "zenity"];

#[derive(Clone, Copy)]
enum Pick {
    Folder,
    DllFile,
}

impl FolderPicker {
    pub(crate) fn is_available() -> bool {
        picker().is_some()
    }

    pub(crate) fn is_open(&self) -> bool {
        self.picking.is_some()
    }

    pub(crate) fn open(&mut self, start: &Path, notify: Notify) {
        self.open_for(Pick::Folder, start, notify);
    }

    pub(crate) fn open_dll_file(&mut self, start: &Path, notify: Notify) {
        self.open_for(Pick::DllFile, start, notify);
    }

    fn open_for(&mut self, pick: Pick, start: &Path, notify: Notify) {
        let Some(program) = picker() else {
            return;
        };
        if self.is_open() {
            return;
        }
        let arguments = arguments(program, pick, start);
        self.picking = Some(Job::spawn(&notify, move || {
            Command::new(program)
                .args(arguments)
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
                .filter(|path| !path.is_empty())
                .map(PathBuf::from)
        }));
    }

    pub(crate) fn take_picked(&mut self) -> Option<PathBuf> {
        let picked = self.picking.as_ref()?.take()?;
        self.picking = None;
        picked
    }
}

fn picker() -> Option<&'static str> {
    let path = env::var_os("PATH")?;
    let dirs: Vec<PathBuf> = env::split_paths(&path).collect();
    PICKERS
        .iter()
        .copied()
        .find(|program| dirs.iter().any(|dir| dir.join(program).is_file()))
}

fn arguments(program: &str, pick: Pick, start: &Path) -> Vec<OsString> {
    let start = start.as_os_str().to_owned();
    match (program, pick) {
        ("kdialog", Pick::Folder) => vec!["--getexistingdirectory".into(), start],
        ("kdialog", Pick::DllFile) => {
            vec!["--getopenfilename".into(), start, "DLL mods (*.dll)".into()]
        }
        (_, Pick::Folder) => vec!["--file-selection".into(), "--directory".into(), start],
        (_, Pick::DllFile) => {
            let filter = "--file-filter=DLL mods | *.dll *.DLL";
            // zenity needs the trailing slash
            let mut filename = OsString::from("--filename=");
            filename.push(start);
            filename.push("/");
            vec!["--file-selection".into(), filename, filter.into()]
        }
    }
}
