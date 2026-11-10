use std::io;
use std::path::{Path, PathBuf};
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
        let mut child = crate::platform::open_command(folder).spawn()?;
        thread::spawn(move || child.wait());
        Ok(true)
    }
}
