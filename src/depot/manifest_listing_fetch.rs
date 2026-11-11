use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use crate::depot::{DepotEvent, DepotProcess, DepotUpdate, DownloadRequest, ManifestListing};

pub fn fetch_manifest_listing(
    program: &Path,
    request: &DownloadRequest,
    cancelled: &AtomicBool,
) -> Result<ManifestListing, String> {
    let request = request.clone().with_manifest_only(true);
    let (wake, woken) = mpsc::channel();
    let process = DepotProcess::start(program, &request, move || {
        let _ = wake.send(());
    })
    .map_err(|error| {
        format!(
            "couldn't start DepotDownloader at {}: {error}",
            program.display()
        )
    })?;
    let mut failure = None;
    loop {
        if cancelled.load(Ordering::Relaxed) {
            process.cancel();
            return Err("cancelled".to_owned());
        }
        let _ = woken.recv_timeout(Duration::from_millis(100));
        for update in process.take_updates() {
            match update {
                DepotUpdate::Event(
                    DepotEvent::PasswordRequested
                    | DepotEvent::CodeRequested(_)
                    | DepotEvent::QrCodeFollows,
                ) => {
                    process.cancel();
                    return Err("Steam wants a new sign-in".to_owned());
                }
                DepotUpdate::Event(DepotEvent::Failed(reason)) => {
                    failure.get_or_insert(reason);
                }
                DepotUpdate::Event(_) => {}
                DepotUpdate::Exited { success: false } => {
                    return Err(failure.unwrap_or_else(|| {
                        "DepotDownloader stopped before listing the files".to_owned()
                    }));
                }
                DepotUpdate::Exited { success: true } => return read_listing(&request),
            }
        }
    }
}

fn read_listing(request: &DownloadRequest) -> Result<ManifestListing, String> {
    let path = request
        .folder()
        .join(ManifestListing::file_name(request.entry()));
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("couldn't read {}: {error}", path.display()))?;
    ManifestListing::from_text(&text)
        .ok_or_else(|| format!("{} isn't a DepotDownloader file list", path.display()))
}
