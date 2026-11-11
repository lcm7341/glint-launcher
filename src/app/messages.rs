use super::ModChange;
use crate::gd::CatalogEntry;
use crate::instances::InstanceName;
use crate::settings::NewInstanceSave;
use std::path::PathBuf;

#[derive(Clone)]
pub(crate) enum Action {
    Create {
        name: Option<InstanceName>,
        entry: CatalogEntry,
        linked: bool,
    },
    SetLinked {
        instance: InstanceName,
        linked: bool,
    },
    SetNewInstanceSave(NewInstanceSave),
    CopyMainSave(InstanceName),
    RestoreBackup {
        instance: InstanceName,
        backup: PathBuf,
    },
    Rename {
        from: InstanceName,
        to: InstanceName,
    },
    Delete(InstanceName),
    OpenFolder(InstanceName),
    OpenLink(String),
    OpenSaveFolder(InstanceName),
    Download(InstanceName),
    CancelDownload,
    InstallUpdate,
    MoveToApplications,
    StayOutsideApplications,
    SignOut,
    SetDefault(InstanceName),
    ToggleFavorite(InstanceName),
    Play(InstanceName),
    AddExisting(PathBuf),
    InstallGeode(InstanceName),
    RemoveGeode(InstanceName),
    Mod {
        instance: InstanceName,
        change: ModChange,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Notice {
    pub(crate) text: String,
    pub(crate) is_failure: bool,
}

impl Notice {
    pub(super) fn done(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_failure: false,
        }
    }

    pub(super) fn failed(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_failure: true,
        }
    }
}
