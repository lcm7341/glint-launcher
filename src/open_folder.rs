use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Default)]
pub(crate) struct FolderOpener {
    last: Option<(PathBuf, Instant)>,
}

impl FolderOpener {
    pub(crate) fn open(&mut self, folder: &Path) -> io::Result<bool> {
        self.open_at(folder, Instant::now())
    }

    fn open_at(&mut self, folder: &Path, now: Instant) -> io::Result<bool> {
        if let Some((last, at)) = &self.last
            && last == folder
            && now.duration_since(*at) < Duration::from_secs(4)
        {
            return Ok(false);
        }
        self.last = Some((folder.to_owned(), now));
        if cfg!(test) {
            return Ok(true);
        }
        let mut child = Command::new(OPENER).arg(folder).spawn()?;
        thread::spawn(move || child.wait());
        Ok(true)
    }
}

#[cfg(target_os = "windows")]
const OPENER: &str = "explorer";

#[cfg(target_os = "macos")]
const OPENER: &str = "open";

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const OPENER: &str = "xdg-open";
