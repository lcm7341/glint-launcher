use std::path::{Path, PathBuf};
use std::process::Command;

use super::{HostOs, PickKind, linux, macos, windows};

// Compile every host's contract on the test host, without running native tools.
#[test]
fn host_contracts_match() {
    macro_rules! check {
        ($host:ident) => {{
            let _: fn() -> Option<PathBuf> = $host::paths::home_dir;
            let _: fn() -> Option<PathBuf> = $host::paths::data_dir;
            let _: fn(Option<&Path>) -> Option<PathBuf> = $host::paths::native_save_dir;
            let _: fn() -> Vec<PathBuf> = $host::steam::folders;
            let _: fn(&Path) -> Command = $host::desktop::open_command;
            let _: fn() -> std::io::Result<bool> = $host::processes::game_running;
            let _: fn() -> std::io::Result<bool> = $host::processes::steam_running;
            let _: fn() -> std::io::Result<Vec<PathBuf>> = $host::processes::game_dirs;
            let _: fn(&Path, Option<&Path>) -> std::io::Result<Command> =
                $host::native_game_command;
            let _: fn(&str, PickKind, &Path) -> Command = $host::file_picker::command;
            assert!(!$host::file_picker::PROGRAMS.is_empty());
        }};
    }
    check!(windows);
    check!(macos);
    check!(linux);
}

fn args(command: &Command) -> Vec<String> {
    command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn windows_picker_args() {
    let start = Path::new("C:/Users/test/Downloads/$('quoted path')");
    for kind in [PickKind::Folder, PickKind::DllFile, PickKind::GeodeFile] {
        let command = windows::file_picker::command("powershell.exe", kind, start);
        let arguments = args(&command);
        assert_eq!(
            &arguments[..4],
            ["-NoProfile", "-NonInteractive", "-STA", "-Command"]
        );
        assert!(!arguments[4].contains("$('quoted path')"));
        assert!(arguments[4].contains("$env:GLINT_PICKER_START"));
        assert!(command.get_envs().any(|(key, value)| key == "GLINT_PICKER_START" && value == Some(start.as_os_str())));
    }
}

#[test]
fn macos_picker_args() {
    let start = Path::new("/Users/test/Downloads/a \"quoted\" folder");
    for kind in [PickKind::Folder, PickKind::DllFile, PickKind::GeodeFile] {
        let command = macos::file_picker::command("osascript", kind, start);
        let arguments = args(&command);
        assert_eq!(arguments.len(), 3);
        assert_eq!(arguments[0], "-e");
        assert!(!arguments[1].contains("quoted"));
        assert_eq!(arguments[2], start.to_str().unwrap());
    }
}

#[test]
fn linux_picker_args() {
    let start = Path::new("/home/test/Downloads/a folder");
    for (kind, filter) in [
        (PickKind::Folder, None),
        (PickKind::DllFile, Some("DLL")),
        (PickKind::GeodeFile, Some("Geode")),
    ] {
        let kde = linux::file_picker::command("kdialog", kind, start);
        let arguments = args(&kde);
        assert_eq!(arguments[1], start.to_str().unwrap());
        if let Some(filter) = filter {
            assert!(arguments[2].contains(filter));
        }
        let gtk = linux::file_picker::command("zenity", kind, start);
        let arguments = args(&gtk);
        assert_eq!(arguments[0], "--file-selection");
        assert_eq!(arguments[1], format!("--filename={}/", start.display()));
        match filter {
            Some(filter) => assert!(arguments[2].contains(filter)),
            None => assert_eq!(arguments[2], "--directory"),
        }
    }
}

#[test]
fn native_launch_env() {
    let game = Path::new("/games/a game");
    let home = Path::new("/instances/a/home");
    let mac = macos::launch::command(game, Some(home));
    assert_eq!(mac.get_program(), game.join(crate::gd::MAC_PROGRAM));
    assert_eq!(mac.get_current_dir(), Some(game));
    assert!(
        mac.get_envs()
            .any(|(key, value)| key == "CFFIXED_USER_HOME" && value == Some(home.as_os_str()))
    );
    let shared = macos::launch::command(game, None);
    assert!(!shared.get_envs().any(|(key, _)| key == "CFFIXED_USER_HOME"));
    let win = windows::launch::command(game, None);
    assert_eq!(win.get_program(), game.join(crate::gd::WINDOWS_PROGRAM));
    assert_eq!(win.get_current_dir(), Some(game));
    for command in [&mac, &win] {
        assert!(command.get_envs().any(
            |(key, value)| key == "SteamAppId" && value == Some(std::ffi::OsStr::new("322170"))
        ));
    }
}

#[test]
fn unsupported_operations() {
    assert_eq!(HostOs::Linux.native_game_platform(), None);
    assert_eq!(
        linux::native_game_command(Path::new("/unused"), None)
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::Unsupported
    );
    assert_eq!(
        macos::processes::game_dirs().unwrap_err().kind(),
        std::io::ErrorKind::Unsupported
    );
    assert_eq!(
        windows::processes::game_dirs().unwrap_err().kind(),
        std::io::ErrorKind::Unsupported
    );
}
