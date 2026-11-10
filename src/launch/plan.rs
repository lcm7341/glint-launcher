//! launch policy, no I/O
use crate::gd::GdPlatform;
use crate::platform::HostOs;

use super::SetupStep;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LaunchRoute {
    Native,
    Wine { through_steam: bool },
    Proton,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum WineStatus {
    NotSetUp,
    SettingUp(SetupStep),
    Ready,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LaunchBlocker {
    EncryptedMacBuild,
    MacBuildOnOtherHost,
    SteamMissing,
    ProtonMissing,
    SteamDrmWithProton,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LaunchReadiness {
    Ready(LaunchRoute),
    NeedsWineSetup,
    SettingUp(SetupStep),
    Blocked(LaunchBlocker),
}

pub(crate) struct LaunchFacts {
    pub(crate) host: HostOs,
    pub(crate) build: GdPlatform,
    pub(crate) encrypted_mac_build: bool,
    pub(crate) steam_drm: bool,
    pub(crate) steam_available: bool,
    pub(crate) proton_available: bool,
    pub(crate) wine: WineStatus,
}

impl LaunchFacts {
    pub(crate) fn readiness(&self) -> LaunchReadiness {
        if self.host.native_game_platform() == Some(self.build) {
            return if self.build == GdPlatform::MacOs && self.encrypted_mac_build {
                LaunchReadiness::Blocked(LaunchBlocker::EncryptedMacBuild)
            } else {
                LaunchReadiness::Ready(LaunchRoute::Native)
            };
        }
        if self.build == GdPlatform::MacOs {
            return LaunchReadiness::Blocked(LaunchBlocker::MacBuildOnOtherHost);
        }
        if self.host == HostOs::MacOs {
            return match self.wine {
                WineStatus::NotSetUp => LaunchReadiness::NeedsWineSetup,
                WineStatus::SettingUp(step) => LaunchReadiness::SettingUp(step),
                WineStatus::Ready => LaunchReadiness::Ready(LaunchRoute::Wine {
                    through_steam: self.steam_drm,
                }),
            };
        }
        if !self.steam_available {
            return LaunchReadiness::Blocked(LaunchBlocker::SteamMissing);
        }
        if !self.proton_available {
            return LaunchReadiness::Blocked(LaunchBlocker::ProtonMissing);
        }
        if self.steam_drm {
            return LaunchReadiness::Blocked(LaunchBlocker::SteamDrmWithProton);
        }
        LaunchReadiness::Ready(LaunchRoute::Proton)
    }
}
