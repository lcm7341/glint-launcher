use crate::instances::{GameFiles, Instance};

use crate::launcher::{Action, Launcher};

pub(crate) struct Play {
    pub(crate) label: &'static str,
    pub(crate) hint: String,
    pub(crate) action: Option<Action>,
}

impl Play {
    fn off(label: &'static str, hint: impl Into<String>) -> Self {
        Self {
            label,
            hint: hint.into(),
            action: None,
        }
    }

    fn on(label: &'static str, hint: impl Into<String>, action: Action) -> Self {
        Self {
            label,
            hint: hint.into(),
            action: Some(action),
        }
    }
}

pub(crate) fn play(launcher: &Launcher, instance: Option<&Instance>) -> Play {
    let Some(instance) = instance else {
        return Play::off("Play", "Make an instance first");
    };
    let name = instance.name().clone();

    if instance.game_files() == GameFiles::Present {
        if let Some(playing) = launcher.playing() {
            return Play::off("Playing", format!("{playing} is open"));
        }
        return match launcher.cant_play(instance) {
            Some(reason) => Play::off("Play", reason),
            None => Play::on("Play", "Steam has to be running", Action::Play(name)),
        };
    }

    if let Some(download) = launcher.download() {
        return if download.instance() == instance.name() {
            Play::off("Downloading", "")
        } else {
            Play::off("Download", "Another download is running")
        };
    }

    let label = if instance.game_files() == GameFiles::Incomplete {
        "Finish download"
    } else {
        "Download"
    };
    let version = &instance.config().version;
    let has_tool = launcher.depot_downloader().is_some();
    let hint = match (has_tool, launcher.steam_account()) {
        (true, Some(account)) => format!("Download {version} as {account}"),
        (true, None) => {
            format!("Download {version} (sign in with a QR code)")
        }
        (false, _) => match launcher.tools_dir() {
            Some(tools) => format!("DepotDownloader not found, put it in {}", tools.display()),
            None => "DepotDownloader not found".to_owned(),
        },
    };
    if has_tool {
        Play::on(label, hint, Action::Download(name))
    } else {
        Play::off(label, hint)
    }
}
