use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use crate::gd::GD_APP_ID;

use crate::launch::{LaunchRequest, MAC_PROGRAM, mac_game_command};

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

    pub fn start_mac(
        game_dir: &Path,
        own_home: Option<&Path>,
        notify: impl Fn() + Send + 'static,
    ) -> io::Result<Self> {
        if let Some(home) = own_home {
            make_runnable(&game_dir.join(MAC_PROGRAM))?;
            fs::create_dir_all(home)?;
        }
        Self::watch(mac_game_command(game_dir, own_home), notify)
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

#[cfg(unix)]
fn make_runnable(program: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(program)?.permissions();
    if permissions.mode() & 0o111 == 0 {
        permissions.set_mode(permissions.mode() | 0o111);
        fs::set_permissions(program, permissions)?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn make_runnable(_program: &Path) -> io::Result<()> {
    Ok(())
}
