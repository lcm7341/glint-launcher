use std::fs;
use std::io;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use crate::gd::GD_APP_ID;

use crate::launch::LaunchRequest;

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
        let mut child = request
            .to_command()
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
