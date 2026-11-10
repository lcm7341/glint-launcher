use super::{
    GdPlatform, HostOs, LaunchBlocker, LaunchFacts, LaunchReadiness, LaunchRoute, SetupStep,
    WineStatus,
};

fn facts(host: HostOs, build: GdPlatform) -> LaunchFacts {
    LaunchFacts {
        host,
        build,
        encrypted_mac_build: false,
        steam_drm: false,
        steam_available: true,
        proton_available: true,
        wine: WineStatus::Ready,
    }
}

#[test]
fn host_routes() {
    for (host, build, expected) in [
        (
            HostOs::Windows,
            GdPlatform::Windows,
            Some(LaunchRoute::Native),
        ),
        (HostOs::MacOs, GdPlatform::MacOs, Some(LaunchRoute::Native)),
        (
            HostOs::MacOs,
            GdPlatform::Windows,
            Some(LaunchRoute::Wine {
                through_steam: false,
            }),
        ),
        (
            HostOs::Linux,
            GdPlatform::Windows,
            Some(LaunchRoute::Proton),
        ),
        (HostOs::Windows, GdPlatform::MacOs, None),
        (HostOs::Linux, GdPlatform::MacOs, None),
    ] {
        let actual = facts(host, build).readiness();
        match expected {
            Some(route) => assert!(actual == LaunchReadiness::Ready(route)),
            None => assert!(actual == LaunchReadiness::Blocked(LaunchBlocker::MacBuildOnOtherHost)),
        }
    }
}

#[test]
fn steam_drm_routes() {
    let mut mac = facts(HostOs::MacOs, GdPlatform::Windows);
    mac.steam_drm = true;
    assert!(
        mac.readiness()
            == LaunchReadiness::Ready(LaunchRoute::Wine {
                through_steam: true
            })
    );
    let mut linux = facts(HostOs::Linux, GdPlatform::Windows);
    linux.steam_drm = true;
    assert!(linux.readiness() == LaunchReadiness::Blocked(LaunchBlocker::SteamDrmWithProton));
    let mut windows = facts(HostOs::Windows, GdPlatform::Windows);
    windows.steam_drm = true;
    assert!(windows.readiness() == LaunchReadiness::Ready(LaunchRoute::Native));
}

#[test]
fn wine_setup_states() {
    let mut mac = facts(HostOs::MacOs, GdPlatform::Windows);
    mac.wine = WineStatus::NotSetUp;
    assert!(mac.readiness() == LaunchReadiness::NeedsWineSetup);
    mac.wine = WineStatus::SettingUp(SetupStep::DownloadingSteam);
    assert!(mac.readiness() == LaunchReadiness::SettingUp(SetupStep::DownloadingSteam));
}

#[test]
fn proton_prerequisites() {
    let mut linux = facts(HostOs::Linux, GdPlatform::Windows);
    linux.steam_available = false;
    linux.proton_available = false;
    assert!(linux.readiness() == LaunchReadiness::Blocked(LaunchBlocker::SteamMissing));
    linux.steam_available = true;
    assert!(linux.readiness() == LaunchReadiness::Blocked(LaunchBlocker::ProtonMissing));
}

#[test]
fn encrypted_macos_blocked() {
    let mut mac = facts(HostOs::MacOs, GdPlatform::MacOs);
    mac.encrypted_mac_build = true;
    assert!(mac.readiness() == LaunchReadiness::Blocked(LaunchBlocker::EncryptedMacBuild));
}
