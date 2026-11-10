use crate::task::Notify;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::thread;
use std::time::Duration;

pub(super) struct RunningGames {
    game_dirs: Arc<Mutex<Vec<PathBuf>>>,
    stop: mpsc::Sender<()>,
}

impl RunningGames {
    pub(super) fn watch(notify: &Notify) -> Self {
        let game_dirs = Arc::new(Mutex::new(Vec::new()));
        let shared = Arc::clone(&game_dirs);
        let notify = Arc::clone(notify);
        let (stop, stopped) = mpsc::channel();
        thread::spawn(move || {
            loop {
                let found = crate::platform::running_game_dirs().unwrap_or_default();
                let changed = {
                    let mut known = shared.lock().unwrap_or_else(PoisonError::into_inner);
                    let changed = *known != found;
                    *known = found;
                    changed
                };
                if changed {
                    notify();
                }
                match stopped.recv_timeout(Duration::from_secs(2)) {
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });
        Self { game_dirs, stop }
    }

    pub(super) fn game_dirs(&self) -> Vec<PathBuf> {
        self.game_dirs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Drop for RunningGames {
    fn drop(&mut self) {
        let _ = self.stop.send(());
    }
}
