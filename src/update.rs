mod applications;
mod apply;
mod release;
mod version;

pub(crate) use applications::{bundle_outside_applications, move_to_applications};
pub(crate) use apply::{clean_up_old_files, install, relaunch};
pub(crate) use release::{Release, fetch_latest};
pub(crate) use version::Version;

use std::{fmt, io};

/// glint-launcher.com serves this with each release
pub(crate) const MANIFEST_URL: &str = "https://glint-launcher.com/download/latest.json";

pub(crate) fn current_version() -> Version {
    env!("CARGO_PKG_VERSION")
        .parse()
        .expect("Cargo.toml's version is major.minor.patch")
}

/// off in debug builds unless GLINT_CHECK_UPDATES=1
pub(crate) fn checks_enabled() -> bool {
    !cfg!(debug_assertions) || std::env::var_os("GLINT_CHECK_UPDATES").is_some()
}

/// release file for this platform
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
            Self::Network(error) => write!(f, "couldn't reach glint-launcher.com: {error}"),
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
