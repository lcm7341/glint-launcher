use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::instances::InstanceName;

use crate::instances::SaveSwapError;
use crate::instances::swap_lock::{SwapLock, SwapState};
use crate::paths::DataDir;

// only renames so the save is never half moved
#[derive(Clone, PartialEq, Eq)]
pub struct SaveSwap {
    usual: PathBuf,
    parked: PathBuf,
    lock: PathBuf,
    first_backup: PathBuf,
}

impl SaveSwap {
    pub fn new(data_dir: &DataDir, usual: impl Into<PathBuf>) -> Self {
        let usual = usual.into();
        let mut parked_name = usual.file_name().unwrap_or_default().to_owned();
        parked_name.push(".glint-usual");
        Self {
            parked: usual.with_file_name(parked_name),
            usual,
            lock: data_dir.save_swap_lock_path(),
            first_backup: data_dir.save_backups_dir().join("usual-before-first-swap"),
        }
    }

    pub fn has_unfinished(&self) -> bool {
        self.lock.exists()
    }

    pub fn swap_in(&self, instance: &InstanceName, save: &Path) -> Result<(), SaveSwapError> {
        self.restore()?;
        if self.parked.exists() {
            return Err(self.unexpected(format!("unexpected {}", self.parked.display())));
        }
        fs::create_dir_all(save).map_err(|source| io_error("make", save, source))?;
        if let Some(parent) = self.usual.parent() {
            fs::create_dir_all(parent).map_err(|source| io_error("make", parent, source))?;
        }
        let had_usual = self.usual.exists();
        if had_usual {
            self.back_up_once()?;
        }
        let mut lock = SwapLock {
            instance: instance.clone(),
            save: save.to_owned(),
            had_usual,
            state: SwapState::SwappingIn,
        };
        lock.write(&self.lock)
            .map_err(|source| io_error("write", &self.lock, source))?;
        if had_usual && let Err(source) = fs::rename(&self.usual, &self.parked) {
            return Err(self.undo(&self.usual, &self.parked, source, false));
        }
        if let Err(source) = fs::rename(save, &self.usual) {
            return Err(self.undo(save, &self.usual, source, had_usual));
        }
        lock.state = SwapState::SwappedIn;
        let _ = lock.write(&self.lock);
        Ok(())
    }

    pub fn restore(&self) -> Result<Option<InstanceName>, SaveSwapError> {
        let mut lock =
            SwapLock::read(&self.lock).map_err(|source| io_error("read", &self.lock, source))?;
        let Some(lock) = lock.as_mut() else {
            return Ok(None);
        };
        lock.state = SwapState::SwappingOut;
        lock.write(&self.lock)
            .map_err(|source| io_error("write", &self.lock, source))?;
        let save = lock.save.clone();
        let (has_usual, has_parked, has_save) =
            (self.usual.exists(), self.parked.exists(), save.exists());
        if lock.had_usual {
            match (has_usual, has_parked, has_save) {
                (true, true, false) => {
                    self.move_back(&self.usual, &save)?;
                    self.move_back(&self.parked, &self.usual)?;
                }
                (true, true, true) => {
                    return Err(self.unexpected(format!(
                        "both {} and {} exist (usual save parked at {})",
                        self.usual.display(),
                        save.display(),
                        self.parked.display()
                    )));
                }
                (false, true, _) => self.move_back(&self.parked, &self.usual)?,
                (true, false, _) => {}
                (false, false, _) => {
                    return Err(self.unexpected(format!(
                        "usual save missing from {} and {}",
                        self.usual.display(),
                        self.parked.display()
                    )));
                }
            }
        } else {
            if has_parked {
                return Err(self.unexpected(format!("unexpected {}", self.parked.display())));
            }
            match (has_usual, has_save) {
                (true, false) => self.move_back(&self.usual, &save)?,
                (true, true) => {
                    return Err(self.unexpected(format!(
                        "both {} and {} exist",
                        self.usual.display(),
                        save.display()
                    )));
                }
                (false, _) => {}
            }
        }
        fs::create_dir_all(&save).map_err(|source| io_error("make", &save, source))?;
        fs::remove_file(&self.lock).map_err(|source| io_error("remove", &self.lock, source))?;
        Ok(Some(lock.instance.clone()))
    }

    fn move_back(&self, from: &Path, to: &Path) -> Result<(), SaveSwapError> {
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(|source| io_error("make", parent, source))?;
        }
        fs::rename(from, to).map_err(|source| SaveSwapError::Unfinished {
            from: from.to_owned(),
            to: to.to_owned(),
            source,
            lock: self.lock.clone(),
        })
    }

    fn undo(&self, from: &Path, to: &Path, source: io::Error, is_parked: bool) -> SaveSwapError {
        if is_parked && fs::rename(&self.parked, &self.usual).is_err() {
            return SaveSwapError::Unfinished {
                from: from.to_owned(),
                to: to.to_owned(),
                source,
                lock: self.lock.clone(),
            };
        }
        let _ = fs::remove_file(&self.lock);
        SaveSwapError::Undone {
            from: from.to_owned(),
            to: to.to_owned(),
            source,
        }
    }

    fn back_up_once(&self) -> Result<(), SaveSwapError> {
        if self.first_backup.exists() {
            return Ok(());
        }
        let partial = self.first_backup.with_extension("partial");
        if partial.exists() {
            fs::remove_dir_all(&partial).map_err(|source| io_error("remove", &partial, source))?;
        }
        fs::create_dir_all(&partial).map_err(|source| io_error("make", &partial, source))?;
        let entries =
            fs::read_dir(&self.usual).map_err(|source| io_error("read", &self.usual, source))?;
        for entry in entries {
            let entry = entry.map_err(|source| io_error("read", &self.usual, source))?;
            let name = entry.file_name();
            let is_save_file = name
                .to_str()
                .is_some_and(|name| name.starts_with("CC") && name.ends_with(".dat"));
            if is_save_file && entry.path().is_file() {
                fs::copy(entry.path(), partial.join(&name))
                    .map_err(|source| io_error("back up", entry.path(), source))?;
            }
        }
        fs::rename(&partial, &self.first_backup)
            .map_err(|source| io_error("finish the backup in", &partial, source))
    }

    fn unexpected(&self, detail: String) -> SaveSwapError {
        SaveSwapError::Unexpected {
            detail,
            lock: self.lock.clone(),
        }
    }
}

fn io_error(action: &'static str, path: impl Into<PathBuf>, source: io::Error) -> SaveSwapError {
    SaveSwapError::Io {
        action,
        path: path.into(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::SaveSwap;
    use crate::instances::swap_lock::{SwapLock, SwapState};
    use crate::test_support::TestDir;
    use std::fs;

    #[test]
    fn swap_round_trip() {
        let dir = TestDir::new();
        let data = dir.data_dir();
        fs::create_dir_all(data.root()).unwrap();
        let usual = dir.path().join("usual");
        let save = dir.path().join("instance-save");
        fs::create_dir(&usual).unwrap();
        fs::create_dir(&save).unwrap();
        fs::write(usual.join("CCGameManager.dat"), b"original").unwrap();
        fs::write(save.join("CCGameManager.dat"), b"instance").unwrap();
        let swap = SaveSwap::new(&data, &usual);
        let name = "game".parse().unwrap();
        swap.swap_in(&name, &save)
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(
            fs::read(usual.join("CCGameManager.dat")).unwrap(),
            b"instance"
        );
        assert!(swap.has_unfinished());
        fs::write(usual.join("CCGameManager.dat"), b"updated").unwrap();
        let restored = swap.restore().unwrap_or_else(|error| panic!("{error}"));
        assert!(restored == Some(name));
        assert_eq!(
            fs::read(usual.join("CCGameManager.dat")).unwrap(),
            b"original"
        );
        assert_eq!(
            fs::read(save.join("CCGameManager.dat")).unwrap(),
            b"updated"
        );
        assert!(!swap.has_unfinished());
    }

    #[test]
    fn interrupted_recovery() {
        let dir = TestDir::new();
        let data = dir.data_dir();
        fs::create_dir_all(data.root()).unwrap();
        let usual = dir.path().join("usual");
        let parked = dir.path().join("usual.glint-usual");
        let save = dir.path().join("instance-save");
        fs::create_dir(&parked).unwrap();
        fs::create_dir(&save).unwrap();
        fs::write(parked.join("CCGameManager.dat"), b"original").unwrap();
        fs::write(save.join("CCGameManager.dat"), b"instance").unwrap();
        SwapLock {
            instance: "game".parse().unwrap(),
            save: save.clone(),
            had_usual: true,
            state: SwapState::SwappingIn,
        }
        .write(&data.save_swap_lock_path())
        .unwrap();
        SaveSwap::new(&data, &usual)
            .restore()
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(
            fs::read(usual.join("CCGameManager.dat")).unwrap(),
            b"original"
        );
        assert_eq!(
            fs::read(save.join("CCGameManager.dat")).unwrap(),
            b"instance"
        );
        assert!(!data.save_swap_lock_path().exists());
    }

    #[test]
    fn ambiguous_recovery() {
        let dir = TestDir::new();
        let data = dir.data_dir();
        fs::create_dir_all(data.root()).unwrap();
        let usual = dir.path().join("usual");
        let parked = dir.path().join("usual.glint-usual");
        let save = dir.path().join("instance-save");
        for (path, content) in [
            (&usual, b"active"),
            (&parked, b"parked"),
            (&save, b"copied"),
        ] {
            fs::create_dir(path).unwrap();
            fs::write(path.join("CCGameManager.dat"), content).unwrap();
        }
        SwapLock {
            instance: "game".parse().unwrap(),
            save: save.clone(),
            had_usual: true,
            state: SwapState::SwappedIn,
        }
        .write(&data.save_swap_lock_path())
        .unwrap();
        assert!(SaveSwap::new(&data, &usual).restore().is_err());
        assert_eq!(
            fs::read(usual.join("CCGameManager.dat")).unwrap(),
            b"active"
        );
        assert_eq!(
            fs::read(parked.join("CCGameManager.dat")).unwrap(),
            b"parked"
        );
        assert_eq!(fs::read(save.join("CCGameManager.dat")).unwrap(), b"copied");
        assert!(data.save_swap_lock_path().exists());
    }
}
