use std::fmt;
use std::io;

pub enum FetchError {
    NoReleaseHere,
    Network(String),
    Archive(String),
    Io(io::Error),
}

impl fmt::Display for FetchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoReleaseHere => {
                formatter.write_str("no DepotDownloader build for this platform")
            }
            Self::Network(reason) => write!(formatter, "couldn't download it: {reason}"),
            Self::Archive(reason) => write!(formatter, "its zip is broken: {reason}"),
            Self::Io(error) => write!(formatter, "couldn't save it: {error}"),
        }
    }
}
