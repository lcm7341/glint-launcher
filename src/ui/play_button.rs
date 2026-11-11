use crate::instances::InstanceName;
use crate::instances::{GameFiles, Instance};

use crate::app::{Action, App, launch_blocker_message};
use crate::launch::LaunchReadiness;

pub(crate) struct Play {
    pub(crate) label: &'static str,
    pub(crate) hint: String,
    pub(crate) click: Option<Click>,
}

pub(crate) enum Click {
    Run(Action),
    SetUpWine(InstanceName),
}

impl Play {
    fn off(label: &'static str, hint: impl Into<String>) -> Self {
        Self {
            label,
            hint: hint.into(),
            click: None,
        }
    }

    fn on(label: &'static str, hint: impl Into<String>, click: Click) -> Self {
        Self {
            label,
            hint: hint.into(),
            click: Some(click),
        }
    }
}

pub(crate) fn play(launcher: &App, instance: Option<&Instance>) -> Play {
    let Some(instance) = instance else {
        return Play::off("Play", "Make an instance first");
    };
    let name = instance.name().clone();

    if instance.game_files() == GameFiles::Present {
        if let Some(playing) = launcher.playing() {
            return Play::off("Playing", format!("{playing} is open"));
        }
        return match launcher.launch_readiness(instance) {
            LaunchReadiness::NeedsWineSetup => Play::on(
                "Set up",
                "Set up Wine for pre-2.2 versions",
                Click::SetUpWine(name),
            ),
            LaunchReadiness::SettingUp(step) => {
                let hint = launcher
                    .wine_setup()
                    .map_or_else(|| format!("{}…", step.describe()), |setup| setup.describe());
                Play::off("Setting up", hint)
            }
            LaunchReadiness::Blocked(blocker) => Play::off(
                "Play",
                launch_blocker_message(blocker, &instance.config().version),
            ),
            LaunchReadiness::Ready(_) => Play::on("Play", "", Click::Run(Action::Play(name))),
        };
    }

    if launcher.fetching_depot() {
        return Play::off("Preparing", "Getting DepotDownloader");
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
        (false, _) if launcher.can_fetch_depot() => format!("Download {version}"),
        (false, _) => match launcher.tools_dir() {
            Some(tools) => format!("DepotDownloader not found, put it in {}", tools.display()),
            None => "DepotDownloader not found".to_owned(),
        },
    };
    if has_tool || launcher.can_fetch_depot() {
        Play::on(label, hint, Click::Run(Action::Download(name)))
    } else {
        Play::off(label, hint)
    }
}
