use std::fmt;
use std::io;
use std::path::PathBuf;

pub enum SaveSwapError {
    Io {
        action: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    Undone {
        from: PathBuf,
        to: PathBuf,
        source: io::Error,
    },
    Unfinished {
        from: PathBuf,
        to: PathBuf,
        source: io::Error,
        lock: PathBuf,
    },
    Unexpected {
        detail: String,
        lock: PathBuf,
    },
}

impl fmt::Display for SaveSwapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                action,
                path,
                source,
            } => write!(formatter, "couldn't {action} {}: {source}", path.display()),
            Self::Undone { from, to, source } => write!(
                formatter,
                "couldn't move {} to {}: {source}",
                from.display(),
                to.display()
            ),
            Self::Unfinished {
                from,
                to,
                source,
                lock,
            } => write!(
                formatter,
                "couldn't move {} to {}: {source}. Restart Glint to put the saves back (see {})",
                from.display(),
                to.display(),
                lock.display()
            ),
            Self::Unexpected { detail, lock } => write!(
                formatter,
                "{detail}. Fix the saves by hand and delete {}",
                lock.display()
            ),
        }
    }
}
