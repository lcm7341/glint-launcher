use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use crate::depot::{DownloadRequest, fetch_manifest_listing, seed_folder};
use crate::gd::GdVersion;

use crate::launcher::Notify;

pub(crate) struct Seeding {
    from: GdVersion,
    shared: Arc<Shared>,
}

#[derive(Default)]
struct Shared {
    cancelled: AtomicBool,
    is_copying: AtomicBool,
    copied_bytes: AtomicU64,
    result: Mutex<Option<Result<u64, String>>>,
}

pub(crate) struct SeedFolders {
    pub(crate) seed: PathBuf,
    pub(crate) target: PathBuf,
}

impl Seeding {
    pub(crate) fn start(
        program: PathBuf,
        listing_request: DownloadRequest,
        folders: SeedFolders,
        from: GdVersion,
        notify: Notify,
    ) -> Self {
        let shared = Arc::new(Shared::default());
        let thread_shared = Arc::clone(&shared);
        thread::spawn(move || {
            let result = seed(
                &program,
                &listing_request,
                &folders,
                &thread_shared,
                &notify,
            );
            *thread_shared
                .result
                .lock()
                .unwrap_or_else(PoisonError::into_inner) = Some(result);
            notify();
        });
        Self { from, shared }
    }

    pub(crate) fn from(&self) -> &GdVersion {
        &self.from
    }

    pub(crate) fn detail(&self) -> String {
        if !self.shared.is_copying.load(Ordering::Relaxed) {
            return format!("Listing the files to copy from {}…", self.from);
        }
        let megabytes = self.shared.copied_bytes.load(Ordering::Relaxed) / 1_000_000;
        format!("Copying files from {}… {megabytes} MB", self.from)
    }

    pub(crate) fn take_result(&self) -> Option<Result<u64, String>> {
        self.shared
            .result
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }

    pub(crate) fn cancel(&self) {
        self.shared.cancelled.store(true, Ordering::Relaxed);
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.shared.cancelled.load(Ordering::Relaxed)
    }
}

fn seed(
    program: &Path,
    request: &DownloadRequest,
    folders: &SeedFolders,
    shared: &Shared,
    notify: &Notify,
) -> Result<u64, String> {
    let _ = fs::remove_dir_all(request.folder());
    let listing = fetch_manifest_listing(program, request, &shared.cancelled);
    let _ = fs::remove_dir_all(request.folder());
    let listing = listing?;
    shared.is_copying.store(true, Ordering::Relaxed);
    notify();
    let mut woken_at = 0;
    seed_folder(
        &listing,
        &folders.seed,
        &folders.target,
        &shared.cancelled,
        &mut |copied| {
            shared.copied_bytes.store(copied, Ordering::Relaxed);
            if copied - woken_at >= 10_000_000 {
                woken_at = copied;
                notify();
            }
        },
    )
    .map_err(|error| format!("couldn't copy a file: {error}"))
}
