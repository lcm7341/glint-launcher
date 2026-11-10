use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::gd::GdPlatform;
use crate::instances::{Instance, InstanceConfig, InstanceStore};
use crate::paths::DataDir;
use crate::task::{Job, Notify};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

pub(crate) struct TestDir(PathBuf);

impl TestDir {
    pub(crate) fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "glint-test-{}-{stamp}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
    pub(crate) fn data_dir(&self) -> DataDir {
        DataDir::new(self.0.join("data"))
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(crate) fn instance(
    data: &DataDir,
    name: &str,
    version: &str,
    platform: GdPlatform,
) -> Instance {
    let store = InstanceStore::new(data.clone());
    let instance = store
        .create(
            name.parse().unwrap(),
            InstanceConfig::new(version.parse().unwrap(), platform),
        )
        .unwrap_or_else(|error| panic!("{error}"));
    fs::write(instance.game_dir().join("fixture.bin"), b"game files").unwrap();
    store
        .load(instance.name())
        .unwrap_or_else(|error| panic!("{error}"))
}

pub(crate) fn completed_job<T: Send + 'static>(value: T) -> Job<T> {
    let (sent, received) = mpsc::channel();
    let notify: Notify = Arc::new(move || {
        let _ = sent.send(());
    });
    let job = Job::spawn(&notify, move || value);
    received
        .recv_timeout(Duration::from_secs(5))
        .expect("background job did not publish its result");
    job
}
