use std::process::Command;

use crate::launch::is_steam_running;

use super::Notice;

pub(super) fn steam_needed_notice() -> Option<Notice> {
    if is_steam_running().unwrap_or(true) {
        return None;
    }
    let started = Command::new("explorer")
        .arg("steam://open/main")
        .spawn()
        .is_ok();
    Some(Notice::failed(if started {
        "Starting Steam, press Play again once it's open"
    } else {
        "Steam isn't running"
    }))
}
