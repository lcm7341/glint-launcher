use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::gd::InstanceName;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SwapLock {
    pub(crate) instance: InstanceName,
    pub(crate) save: PathBuf,
    pub(crate) had_usual: bool,
    pub(crate) state: SwapState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SwapState {
    SwappingIn,
    SwappedIn,
    SwappingOut,
}

impl SwapState {
    fn as_str(self) -> &'static str {
        match self {
            Self::SwappingIn => "swapping-in",
            Self::SwappedIn => "swapped-in",
            Self::SwappingOut => "swapping-out",
        }
    }

    fn from_name(text: &str) -> Option<Self> {
        [Self::SwappingIn, Self::SwappedIn, Self::SwappingOut]
            .into_iter()
            .find(|state| state.as_str() == text)
    }
}

impl SwapLock {
    pub(crate) fn read(path: &Path) -> io::Result<Option<Self>> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        Self::from_text(&text).map(Some).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("bad swap lock {}", path.display()),
            )
        })
    }

    pub(crate) fn write(&self, path: &Path) -> io::Result<()> {
        let partial = path.with_extension("lock.partial");
        fs::write(&partial, self.to_text()?)?;
        fs::rename(&partial, path)
    }

    fn to_text(&self) -> io::Result<String> {
        let save = self.save.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("can't write path {}", self.save.display()),
            )
        })?;
        let had_usual = if self.had_usual { "yes" } else { "no" };
        Ok(format!(
            "instance {}\nsave {save}\nhad-usual {had_usual}\nstate {}\n",
            self.instance,
            self.state.as_str()
        ))
    }

    fn from_text(text: &str) -> Option<Self> {
        let (mut instance, mut save, mut had_usual, mut state) = (None, None, None, None);
        for line in text.lines() {
            let Some((key, value)) = line.split_once(' ') else {
                continue;
            };
            match key {
                "instance" => instance = value.parse().ok(),
                "save" => save = Some(PathBuf::from(value)),
                "had-usual" => {
                    had_usual = match value {
                        "yes" => Some(true),
                        "no" => Some(false),
                        _ => None,
                    }
                }
                "state" => state = SwapState::from_name(value),
                _ => {}
            }
        }
        Some(Self {
            instance: instance?,
            save: save?,
            had_usual: had_usual?,
            state: state?,
        })
    }
}
