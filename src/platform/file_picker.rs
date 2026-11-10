use crate::task::Job;
use crate::task::Notify;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use super::{PickKind, find_program, host};

#[derive(Default)]
pub(crate) struct FilePicker {
    picking: Option<Job<io::Result<Option<PathBuf>>>>,
}

impl FilePicker {
    pub(crate) fn is_available() -> bool {
        find_program(host::file_picker::PROGRAMS).is_some()
    }

    pub(crate) fn is_open(&self) -> bool {
        self.picking.is_some()
    }

    pub(crate) fn open(&mut self, start: &Path, notify: Notify) {
        self.open_for(PickKind::Folder, start, notify);
    }

    pub(crate) fn open_dll_file(&mut self, start: &Path, notify: Notify) {
        self.open_for(PickKind::DllFile, start, notify);
    }

    pub(crate) fn open_geode_file(&mut self, start: &Path, notify: Notify) {
        self.open_for(PickKind::GeodeFile, start, notify);
    }

    fn open_for(&mut self, pick: PickKind, start: &Path, notify: Notify) {
        let Some(program) = find_program(host::file_picker::PROGRAMS) else {
            return;
        };
        if self.is_open() {
            return;
        }
        let home = super::home_dir();
        let start = match home {
            Some(home) if !start.is_dir() => home,
            _ => start.to_path_buf(),
        };
        let mut command = host::file_picker::command(program, pick, &start);
        command.stdin(Stdio::null()).stderr(Stdio::null());
        self.picking = Some(Job::spawn(&notify, move || {
            let output = command.output()?;
            // cancel exits nonzero
            if !output.status.success() {
                return Ok(None);
            }
            let path = String::from_utf8(output.stdout)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            Ok((!path.trim().is_empty()).then(|| PathBuf::from(path.trim())))
        }));
    }

    pub(crate) fn take_result(&mut self) -> Option<io::Result<Option<PathBuf>>> {
        let result = self.picking.as_ref()?.take()?;
        self.picking = None;
        Some(result)
    }
}
