use std::path::Path;

use crate::platform::{open_command, steam_running};

use super::Notice;

pub(super) fn steam_needed_notice() -> Option<Notice> {
    if steam_running().unwrap_or(true) {
        return None;
    }
    let started = open_command(Path::new("steam://open/main")).spawn().is_ok();
    Some(Notice::failed(if started {
        "Starting Steam, press Play again once it's open"
    } else {
        "Steam isn't running"
    }))
}
