mod open;
pub(crate) use open::FolderOpener;

use std::env;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

mod file_picker;
pub(crate) use file_picker::FilePicker;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as host;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as host;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod linux;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
use linux as host;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HostOs {
    Windows,
    MacOs,
    Linux,
}

impl HostOs {
    pub(crate) const fn native_game_platform(self) -> Option<crate::gd::GdPlatform> {
        match self {
            Self::Windows => Some(crate::gd::GdPlatform::Windows),
            Self::MacOs => Some(crate::gd::GdPlatform::MacOs),
            Self::Linux => None,
        }
    }

    pub(crate) const CURRENT: Self = if cfg!(target_os = "windows") {
        Self::Windows
    } else if cfg!(target_os = "macos") {
        Self::MacOs
    } else {
        Self::Linux
    };
}

#[derive(Clone, Copy)]
pub(crate) enum PickKind {
    Folder,
    DllFile,
    GeodeFile,
}

pub(crate) fn home_dir() -> Option<PathBuf> {
    host::paths::home_dir()
}
pub(crate) fn data_dir() -> Option<PathBuf> {
    host::paths::data_dir()
}
pub(crate) fn native_save_dir(home: Option<&Path>) -> Option<PathBuf> {
    host::paths::native_save_dir(home)
}
pub(crate) fn steam_folders() -> Vec<PathBuf> {
    host::steam::folders()
}
pub(crate) fn open_command(target: &Path) -> Command {
    host::desktop::open_command(target)
}
pub(crate) fn native_game_running() -> io::Result<bool> {
    host::processes::game_running()
}
pub(crate) fn steam_running() -> io::Result<bool> {
    host::processes::steam_running()
}
pub(crate) fn running_game_dirs() -> io::Result<Vec<PathBuf>> {
    host::processes::game_dirs()
}

fn non_empty_env(key: &str) -> Option<PathBuf> {
    env::var_os(key)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

pub(crate) fn hide_console(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    #[cfg(not(windows))]
    let _ = command;
}

pub(crate) fn native_game_command(
    build: crate::gd::GdPlatform,
    game_dir: &Path,
) -> io::Result<Command> {
    if HostOs::CURRENT.native_game_platform() != Some(build) {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "this build can't run on this computer",
        ));
    }
    host::native_game_command(game_dir)
}
