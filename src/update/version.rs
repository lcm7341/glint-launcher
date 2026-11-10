use std::fmt;
use std::str::FromStr;

use crate::parse_error::ParseError;

/// A release version like 0.0.1, tagged on GitHub as v0.0.1
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl FromStr for Version {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let bad = || ParseError::new(format!("\"{text}\" isn't a version like 1.2.3"));
        let digits = text.strip_prefix('v').unwrap_or(text);
        let mut parts = digits.split('.').map(|part| {
            if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(bad());
            }
            part.parse::<u64>().map_err(|_| bad())
        });
        let mut next = || parts.next().ok_or_else(bad)?;
        let version = Self {
            major: next()?,
            minor: next()?,
            patch: next()?,
        };
        if parts.next().is_some() {
            return Err(bad());
        }
        Ok(version)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
