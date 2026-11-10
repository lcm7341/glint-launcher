use std::fmt;
use std::io;

pub enum ModError {
    NotADll(String),
    AlreadyThere(String),
    BackupInTheWay(String),
    UnreadableSettings(String),
    NoMegaHack,
    Io(io::Error),
}

impl fmt::Display for ModError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotADll(file) => write!(formatter, "{file} isn't a .dll"),
            Self::AlreadyThere(name) => write!(formatter, "There's a mod called {name}"),
            Self::BackupInTheWay(file) => write!(formatter, "The game folder has {file}"),
            Self::UnreadableSettings(path) => write!(formatter, "can't read {path}"),
            Self::NoMegaHack => write!(formatter, "Mega Hack's loader is missing"),
            Self::Io(error) => write!(formatter, "{error}"),
        }
    }
}

impl From<io::Error> for ModError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
