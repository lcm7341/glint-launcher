use std::path::Path;
use std::process::Command;

use crate::platform::PickKind;

pub(in crate::platform) const PROGRAMS: &[&str] = &["powershell.exe"];

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

pub(in crate::platform) fn command(program: &str, pick: PickKind, start: &Path) -> Command {
    let script = match pick {
        PickKind::Folder => POWERSHELL_FOLDER,
        PickKind::DllFile => POWERSHELL_DLL_FILE,
        PickKind::GeodeFile => POWERSHELL_GEODE_FILE,
    };
    let mut command = Command::new(program);
    command
        .args(["-NoProfile", "-NonInteractive", "-STA", "-Command"])
        .arg(format!("{POWERSHELL_PRELUDE}{script}"))
        .env("GLINT_PICKER_START", start);
    crate::platform::hide_console(&mut command);
    command
}
