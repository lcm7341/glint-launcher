use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

use crate::gd::GD_APP_ID;

use crate::launch::launch_request::dll_overrides;
use crate::launch::program_overrides::PROGRAM_OVERRIDES;
use crate::launch::steam_library::SteamLibrary;
use crate::launch::steam_window_fix::{STEAM_WINDOW_FLAGS, place_fix_files};

const STEAM_DIR: &str = "drive_c/Program Files (x86)/Steam";

const STEAM_EXE_WINDOWS: &str = r"C:\Program Files (x86)\Steam\steam.exe";

// 260 char path limit
pub(crate) const STEAM_SETUP_FILE: &str = "SteamSetup.exe";

const GD_SAVE_DIR: &str = "AppData/Local/GeometryDash";

#[derive(Clone, PartialEq, Eq)]
pub struct WineSteam {
    wine: PathBuf,
    prefix: PathBuf,
}

impl WineSteam {
    pub fn new(wine: impl Into<PathBuf>, prefix: impl Into<PathBuf>) -> Self {
        Self {
            wine: wine.into(),
            prefix: prefix.into(),
        }
    }

    pub fn prefix(&self) -> &Path {
        &self.prefix
    }

    pub fn is_steam_installed(&self) -> bool {
        self.prefix.join(STEAM_DIR).join("steam.exe").is_file()
    }

    pub fn has_prefix(&self) -> bool {
        self.prefix.join("system.reg").is_file()
    }

    pub fn create_prefix(&self) -> io::Result<()> {
        let parent = self.prefix.parent().unwrap_or(&self.prefix);
        fs::create_dir_all(parent)?;
        let mut wineboot = self.wine_command(parent);
        wineboot
            .args(["wineboot", "-u"])
            // skip the wine mono/gecko prompt
            .env("WINEDLLOVERRIDES", "mscoree,mshtml=");
        run_to_end(wineboot, "wineboot")?;
        let mut wineserver = Command::new(self.wine.with_file_name("wineserver"));
        wineserver.arg("-w").env("WINEPREFIX", &self.prefix);
        run_to_end(wineserver, "wineserver")
    }

    pub fn run_steam_setup(&self) -> io::Result<()> {
        let mut setup = self.wine_command(&self.prefix.join("drive_c"));
        setup.arg(format!(r"C:\{STEAM_SETUP_FILE}")).arg("/S");
        run_to_end(setup, "Steam's installer")
    }

    pub fn start_steam(&self) -> io::Result<()> {
        self.install_window_fix()?;
        let mut steam = self
            .wine_command(&self.prefix.join(STEAM_DIR))
            .arg(STEAM_EXE_WINDOWS)
            .args(STEAM_WINDOW_FLAGS)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        thread::spawn(move || steam.wait());
        Ok(())
    }

    pub fn install_window_fix(&self) -> io::Result<()> {
        place_fix_files(&self.wine, &self.prefix)?;
        let missing: Vec<_> = PROGRAM_OVERRIDES
            .iter()
            .filter(|setting| !setting.is_in(&self.prefix))
            .collect();
        for setting in missing {
            let mut reg = self.wine_command(&self.prefix);
            reg.args(setting.reg_add_args());
            run_to_end(reg, "wine reg")?;
        }
        Ok(())
    }

    // ActiveUser is 0 until someone signs in
    pub fn is_steam_signed_in(&self) -> bool {
        let mut query = self.wine_command(&self.prefix);
        query.args([
            "reg",
            "query",
            r"HKCU\Software\Valve\Steam\ActiveProcess",
            "/v",
            "ActiveUser",
        ]);
        query
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .is_ok_and(|output| active_user(&String::from_utf8_lossy(&output.stdout)) != 0)
    }

    pub fn steam_library(&self) -> SteamLibrary {
        SteamLibrary::new(&self.prefix)
    }

    pub fn ask_steam_to_play(&self) -> io::Result<()> {
        let mut request = self
            .wine_command(&self.prefix.join(STEAM_DIR))
            .arg(STEAM_EXE_WINDOWS)
            .args(SteamLibrary::applaunch_args())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        thread::spawn(move || request.wait());
        Ok(())
    }

    pub fn restart_steam(&self, done: impl FnOnce() + Send + 'static) {
        let wine_steam = self.clone();
        thread::spawn(move || {
            let mut shutdown = wine_steam.wine_command(&wine_steam.prefix.join(STEAM_DIR));
            shutdown.arg(STEAM_EXE_WINDOWS).arg("-shutdown");
            let _ = run_to_end(shutdown, "Steam for Windows");
            // steam takes a few secs to close
            for _ in 0..60 {
                if !wine_steam_running() {
                    break;
                }
                thread::sleep(std::time::Duration::from_secs(1));
            }
            let _ = wine_steam.start_steam();
            done();
        });
    }

    pub fn game_command(&self, game_dir: &Path) -> Command {
        let mut command = self.wine_command(game_dir);
        command.arg(Path::new(".").join("GeometryDash.exe"));
        let app_id = GD_APP_ID.to_string();
        command
            .env("SteamAppId", &app_id)
            .env("SteamGameId", &app_id);
        let overrides = dll_overrides(game_dir);
        if !overrides.is_empty() {
            command.env("WINEDLLOVERRIDES", overrides);
        }
        command
    }

    pub fn link_save(&self, save_dir: &Path) -> io::Result<()> {
        fs::create_dir_all(save_dir)?;
        let link = self.gd_save_dir();
        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent)?;
        }
        match fs::symlink_metadata(&link) {
            Ok(metadata) if metadata.file_type().is_symlink() => fs::remove_file(&link)?,
            Ok(_) => fs::rename(&link, link.with_file_name("GeometryDash.before-glint"))?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        symlink_dir(save_dir, &link)
    }

    pub fn gd_save_dir(&self) -> PathBuf {
        let users = self.prefix.join("drive_c/users");
        let account = std::env::var("USER").unwrap_or_default();
        if !account.is_empty() && users.join(&account).is_dir() {
            return users.join(account).join(GD_SAVE_DIR);
        }
        let mut names: Vec<String> = fs::read_dir(&users)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter(|entry| entry.path().is_dir())
                    .filter_map(|entry| entry.file_name().into_string().ok())
                    .filter(|name| name != "Public")
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        let user = names.into_iter().next().unwrap_or(account);
        users.join(user).join(GD_SAVE_DIR)
    }

    fn wine_command(&self, working_dir: &Path) -> Command {
        let mut command = Command::new(&self.wine);
        command
            .current_dir(working_dir)
            .env("WINEPREFIX", &self.prefix)
            // wine's fixme spam fills the pipe
            .env("WINEDEBUG", "-all");
        command
    }
}

fn active_user(reg_output: &str) -> u32 {
    reg_output
        .lines()
        .find_map(|line| {
            let mut words = line.split_whitespace();
            (words.next()? == "ActiveUser").then_some(())?;
            let hex = words.nth(1)?.strip_prefix("0x")?;
            u32::from_str_radix(hex, 16).ok()
        })
        .unwrap_or(0)
}

fn run_to_end(mut command: Command, what: &str) -> io::Result<()> {
    let status = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("{what} stopped with {status}")))
    }
}

pub fn wine_steam_running() -> bool {
    Command::new("ps")
        .args(["-axo", "command="])
        .output()
        .is_ok_and(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(is_wine_steam)
        })
}

fn is_wine_steam(command_line: &str) -> bool {
    let is_wine = command_line.to_ascii_lowercase().contains("wine");
    command_line.split_whitespace().any(|word| {
        word.rsplit(['\\', '/'])
            .next()
            .is_some_and(|file| file.eq_ignore_ascii_case("steam.exe"))
            && (is_wine || word.contains('\\'))
    })
}

#[cfg(unix)]
fn symlink_dir(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(not(unix))]
fn symlink_dir(_target: &Path, _link: &Path) -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "mac only"))
}
