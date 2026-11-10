mod apply;
mod release;
mod version;

pub(crate) use apply::{clean_up_old_files, install, relaunch};
pub(crate) use release::{Release, fetch_latest};
pub(crate) use version::Version;

use std::{fmt, io};

/// scripts/release.sh writes this next to the builds, in echo-replays' public/download/glint
pub(crate) const MANIFEST_URL: &str = "https://echogd.com/download/glint/latest.json";

pub(crate) fn current_version() -> Version {
    env!("CARGO_PKG_VERSION")
        .parse()
        .expect("Cargo.toml's version is major.minor.patch")
}

/// Debug builds are usually `cargo run`, which shouldn't replace itself;
/// set GLINT_CHECK_UPDATES=1 to check anyway
pub(crate) fn checks_enabled() -> bool {
    !cfg!(debug_assertions) || std::env::var_os("GLINT_CHECK_UPDATES").is_some()
}

/// The release file built for this computer, named as scripts/release.sh publishes it
pub(crate) const ASSET: Option<&str> = if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
    Some("glint-windows-x86_64.exe")
} else if cfg!(target_os = "macos") {
    Some("glint-macos.zip")
} else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
    Some("glint-linux-x86_64")
} else {
    None
};

#[derive(Debug)]
pub(crate) enum UpdateError {
    Network(String),
    Api(String),
    Checksum,
    Archive(String),
    Unsupported(String),
    Io(io::Error),
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(error) => write!(f, "couldn't reach echogd.com: {error}"),
            Self::Api(error) => f.write_str(error),
            Self::Checksum => f.write_str("the download doesn't match its checksum"),
            Self::Archive(error) => write!(f, "the download is broken: {error}"),
            Self::Unsupported(error) => f.write_str(error),
            Self::Io(error) => error.fmt(f),
        }
    }
}

impl From<io::Error> for UpdateError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
