use std::path::{Path, PathBuf};

use rfd::AsyncFileDialog;

use super::PickKind;
use crate::task::{Job, Notify};

/// the desktop's own file chooser: the portal on linux, NSOpenPanel on macos
#[derive(Default)]
pub(crate) struct FilePicker {
    picking: Option<Job<Option<PathBuf>>>,
}

impl FilePicker {
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
        if self.is_open() {
            return;
        }
        let start = match super::home_dir() {
            Some(home) if !start.is_dir() => home,
            _ => start.to_path_buf(),
        };
        // macos only opens panels on the main thread, the async dialog hands it there
        self.picking = Some(Job::spawn(&notify, move || {
            pollster::block_on(pick_path(pick, &start))
        }));
    }

    /// Some(None) when the dialog was cancelled
    pub(crate) fn take_result(&mut self) -> Option<Option<PathBuf>> {
        let result = self.picking.as_ref()?.take()?;
        self.picking = None;
        Some(result)
    }
}

async fn pick_path(pick: PickKind, start: &Path) -> Option<PathBuf> {
    let dialog = AsyncFileDialog::new().set_directory(start);
    let picked = match pick {
        PickKind::Folder => {
            dialog
                .set_title("Pick a Geometry Dash folder")
                .pick_folder()
                .await
        }
        PickKind::DllFile => {
            dialog
                .set_title("Pick a mod DLL")
                .add_filter("DLL mods", &["dll", "DLL"])
                .pick_file()
                .await
        }
        PickKind::GeodeFile => {
            dialog
                .set_title("Pick a Geode mod")
                .add_filter("Geode mods", &["geode", "GEODE"])
                .pick_file()
                .await
        }
    };
    picked.map(|file| file.path().to_path_buf())
}
