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

#[cfg(not(any(windows, target_os = "macos")))]
const PICKERS: &[&str] = &["kdialog", "zenity"];

#[cfg(target_os = "macos")]
const PICKERS: &[&str] = &["osascript"];

#[cfg(windows)]
const PICKERS: &[&str] = &["powershell.exe"];

#[cfg(windows)]
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const POWERSHELL_PRELUDE: &str = "\
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding $false
Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.Application]::EnableVisualStyles()
$owner = New-Object System.Windows.Forms.Form -Property @{ TopMost = $true }
";

const POWERSHELL_FOLDER: &str = "\
$dialog = New-Object System.Windows.Forms.FolderBrowserDialog
$dialog.SelectedPath = $env:GLINT_PICKER_START
if ($dialog.ShowDialog($owner) -eq 'OK') { [Console]::Out.Write($dialog.SelectedPath) }
";

const POWERSHELL_DLL_FILE: &str = "\
$dialog = New-Object System.Windows.Forms.OpenFileDialog
$dialog.InitialDirectory = $env:GLINT_PICKER_START
$dialog.Filter = 'DLL mods (*.dll)|*.dll'
if ($dialog.ShowDialog($owner) -eq 'OK') { [Console]::Out.Write($dialog.FileName) }
";

const POWERSHELL_GEODE_FILE: &str = "$dialog = New-Object System.Windows.Forms.OpenFileDialog
$dialog.InitialDirectory = $env:GLINT_PICKER_START
$dialog.Filter = 'Geode mods (*.geode)|*.geode'
if ($dialog.ShowDialog($owner) -eq 'OK') { [Console]::Out.Write($dialog.FileName) }
";

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

#[derive(Clone, Copy)]
enum Pick {
    Folder,
    DllFile,
    GeodeFile,
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

    pub(crate) fn open_geode_file(&mut self, start: &Path, notify: Notify) {
        self.open_for(Pick::GeodeFile, start, notify);
    }

    fn open_for(&mut self, pick: Pick, start: &Path, notify: Notify) {
        let Some(program) = picker() else {
            return;
        };
        if self.is_open() {
            return;
        }
        let home = env::var_os("HOME").map(PathBuf::from);
        let start = match home {
            Some(home) if !start.is_dir() => home,
            _ => start.to_path_buf(),
        };
        let mut command = command(program, pick, &start);
        self.picking = Some(Job::spawn(&notify, move || {
            command
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

fn command(program: &str, pick: Pick, start: &Path) -> Command {
    let mut command = Command::new(program);
    command
        .args(arguments(program, pick, start))
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command
            .env("GLINT_PICKER_START", start)
            .creation_flags(CREATE_NO_WINDOW);
    }
    command
}

fn arguments(program: &str, pick: Pick, start: &Path) -> Vec<OsString> {
    let start = start.as_os_str().to_owned();
    match (program, pick) {
        ("powershell.exe", pick) => {
            let script = match pick {
                Pick::Folder => POWERSHELL_FOLDER,
                Pick::DllFile => POWERSHELL_DLL_FILE,
                Pick::GeodeFile => POWERSHELL_GEODE_FILE,
            };
            ps_args(&format!("{POWERSHELL_PRELUDE}{script}"))
        }
        ("osascript", pick) => {
            let script = match pick {
                Pick::Folder => MAC_PICK_FOLDER,
                Pick::DllFile => MAC_PICK_DLL,
                Pick::GeodeFile => MAC_PICK_GEODE,
            };
            vec!["-e".into(), script.into(), start]
        }
        ("kdialog", Pick::Folder) => vec!["--getexistingdirectory".into(), start],
        ("kdialog", Pick::DllFile) => {
            vec!["--getopenfilename".into(), start, "DLL mods (*.dll)".into()]
        }
        ("kdialog", Pick::GeodeFile) => {
            vec![
                "--getopenfilename".into(),
                start,
                "Geode mods (*.geode)".into(),
            ]
        }
        (_, Pick::Folder) => vec!["--file-selection".into(), "--directory".into(), start],
        (_, pick) => {
            let filter = match pick {
                Pick::GeodeFile => "--file-filter=Geode mods | *.geode *.GEODE",
                _ => "--file-filter=DLL mods | *.dll *.DLL",
            };
            // zenity needs the trailing slash
            let mut filename = OsString::from("--filename=");
            filename.push(start);
            filename.push("/");
            vec!["--file-selection".into(), filename, filter.into()]
        }
    }
}

fn ps_args(script: &str) -> Vec<OsString> {
    // forms dialogs need sta
    vec![
        "-NoProfile".into(),
        "-NonInteractive".into(),
        "-STA".into(),
        "-Command".into(),
        script.into(),
    ]
}
