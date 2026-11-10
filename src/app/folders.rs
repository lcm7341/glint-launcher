use std::path::Path;

use crate::instances::InstanceName;

use super::{App, Notice};
use crate::format::home_relative;

impl App {
    pub(crate) fn short_path(&self, path: &Path) -> String {
        let root = self.library.data_dir().map(|data_dir| data_dir.root());
        match root.and_then(|root| path.strip_prefix(root).ok()) {
            Some(inside) => inside.display().to_string(),
            None => home_relative(path),
        }
    }

    pub(super) fn open_folder(&mut self, folder: &Path) -> Option<Notice> {
        match self.folder_opener.open(folder) {
            Ok(true) => Some(Notice::done(format!("Opening {}", home_relative(folder)))),
            Ok(false) => None,
            Err(error) => Some(Notice::failed(format!(
                "Couldn't open {}: {error}",
                folder.display()
            ))),
        }
    }

    pub(super) fn open_instance_folder(&mut self, name: &InstanceName) -> Option<Notice> {
        let folder = match self.find(name) {
            Some(Ok(instance)) if instance.is_imported() => instance.game_dir(),
            _ => self.library.data_dir()?.instance_dir(name),
        };
        self.open_folder(&folder)
    }

    pub(super) fn open_link(&mut self, url: &str) -> Option<Notice> {
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            return Some(Notice::failed(format!("{url} isn't a web link")));
        }
        self.open_folder(Path::new(url))
    }

    pub(super) fn open_save_folder(&mut self, name: &InstanceName) -> Option<Notice> {
        let Some(Ok(instance)) = self.find(name) else {
            return None;
        };
        let folder = self.gd_save_dir(instance);
        if !folder.is_dir() {
            return Some(Notice::failed(format!("\"{name}\" has no save")));
        }
        self.open_folder(&folder)
    }
}
