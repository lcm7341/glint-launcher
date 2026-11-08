use std::fmt;
use std::io;

pub enum GeodeError {
    Network(String),
    Api(String),
    Checksum,
    Archive(String),
    Io(io::Error),
    NoModVersion { id: String, gd: String },
    NotAGeodeMod(String),
    UnreadableModFile { file: String, error: io::Error },
}

impl fmt::Display for GeodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(error) => write!(formatter, "couldn't reach Geode's server: {error}"),
            Self::Api(error) => write!(formatter, "Geode's server: {error}"),
            Self::Checksum => {
                write!(formatter, "checksum mismatch")
            }
            Self::Archive(error) => write!(formatter, "the download can't be unpacked: {error}"),
            Self::Io(error) => write!(formatter, "couldn't write the files: {error}"),
            Self::NoModVersion { id, gd } => {
                write!(formatter, "no {id} for GD {gd} on this platform")
            }
            Self::NotAGeodeMod(file) => write!(formatter, "{file} isn't a .geode file"),
            Self::UnreadableModFile { file, error } => {
                write!(formatter, "couldn't read {file}: {error}")
            }
        }
    }
}
