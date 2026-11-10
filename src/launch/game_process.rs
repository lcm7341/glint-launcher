use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crate::gd::{GD_APP_ID, GdPlatform};

use crate::launch::wine::is_steam_gd_running;
use crate::launch::{LaunchRequest, WineSteam};

pub const STEAM_APP_ID_FILE: &str = "steam_appid.txt";

pub struct GameProcess {
    exited: Arc<AtomicBool>,
}

impl GameProcess {
    pub fn start(
        request: &LaunchRequest,
        may_write_game_dir: bool,
        notify: impl Fn() + Send + 'static,
    ) -> io::Result<Self> {
        if may_write_game_dir {
            write_app_id_file(request.game_dir())?;
        }
        fs::create_dir_all(request.compat_dir())?;
        Self::watch(request.to_command(), notify)
    }

    pub fn start_windows(
        game_dir: &Path,
        may_write_game_dir: bool,
        notify: impl Fn() + Send + 'static,
    ) -> io::Result<Self> {
        if may_write_game_dir {
            write_app_id_file(game_dir)?;
        }
        Self::watch(
            crate::platform::native_game_command(GdPlatform::Windows, game_dir, None)?,
            notify,
        )
    }

    pub fn start_mac(
        game_dir: &Path,
        own_home: Option<&Path>,
        notify: impl Fn() + Send + 'static,
    ) -> io::Result<Self> {
        Self::watch(
            crate::platform::native_game_command(GdPlatform::MacOs, game_dir, own_home)?,
            notify,
        )
    }

    pub fn start_wine(
        wine_steam: &WineSteam,
        game_dir: &Path,
        save_dir: &Path,
        may_write_game_dir: bool,
        notify: impl Fn() + Send + 'static,
    ) -> io::Result<Self> {
        let app_id_file = game_dir.join(STEAM_APP_ID_FILE);
        // stops steam launching its own install
        if may_write_game_dir && !app_id_file.exists() {
            fs::write(&app_id_file, format!("{GD_APP_ID}\n"))?;
        }
        wine_steam.install_window_fix()?;
        wine_steam.link_save(save_dir)?;
        Self::watch(wine_steam.game_command(game_dir), notify)
    }

    pub fn start_through_steam(
        wine_steam: &WineSteam,
        game_dir: &Path,
        instance: &str,
        save_dir: &Path,
        notify: impl Fn() + Send + 'static,
    ) -> io::Result<Self> {
        wine_steam.install_window_fix()?;
        wine_steam.link_save(save_dir)?;
        wine_steam
            .steam_library()
            .put_instance(game_dir, instance)?;
        wine_steam.ask_steam_to_play()?;
        let exited = Arc::new(AtomicBool::new(false));
        let watched = Arc::clone(&exited);
        thread::spawn(move || {
            let poll = Duration::from_millis(500);
            let mut appeared = false;
            for _ in 0..120 {
                if is_steam_gd_running() {
                    appeared = true;
                    break;
                }
                thread::sleep(poll);
            }
            while appeared && is_steam_gd_running() {
                thread::sleep(poll);
            }
            watched.store(true, Ordering::Release);
            notify();
        });
        Ok(Self { exited })
    }

    fn watch(mut command: Command, notify: impl Fn() + Send + 'static) -> io::Result<Self> {
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        let exited = Arc::new(AtomicBool::new(false));
        let watched = Arc::clone(&exited);
        thread::spawn(move || {
            let _ = child.wait();
            watched.store(true, Ordering::Release);
            notify();
        });
        Ok(Self { exited })
    }

    pub fn has_exited(&self) -> bool {
        self.exited.load(Ordering::Acquire)
    }
}

fn write_app_id_file(game_dir: &Path) -> io::Result<()> {
    let app_id_file = game_dir.join(STEAM_APP_ID_FILE);
    if !app_id_file.exists() {
        fs::write(&app_id_file, format!("{GD_APP_ID}\n"))?;
    }
    Ok(())
}
