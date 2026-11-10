use super::{App, Notice};
use crate::instances::FoundInstall;
use crate::instances::InstanceName;
use std::path::{Path, PathBuf};

impl App {
    pub(crate) fn steam_install(&self) -> Option<PathBuf> {
        self.play.steam_install()
    }
    pub(crate) fn found_installs(&self) -> &[FoundInstall] {
        self.library.found()
    }
    pub(crate) fn refresh_found(&mut self) {
        let install = self.steam_install();
        self.library.refresh_found(install);
    }
    pub(crate) fn adopt_steam_install(&mut self) {
        let install = self.steam_install();
        self.library
            .adopt_steam_install(install, &mut self.activity);
    }
    pub(super) fn add_existing(
        &mut self,
        folder: &Path,
        name: Option<InstanceName>,
    ) -> Option<Notice> {
        self.library.add_existing(folder, name, &mut self.activity)
    }
    pub(super) fn note_deleted(&mut self, name: &InstanceName) {
        let install = self.steam_install();
        self.library.note_deleted(name, install);
    }
}
