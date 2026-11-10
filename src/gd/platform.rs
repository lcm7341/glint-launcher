use std::fmt;
use std::str::FromStr;

use crate::parse_error::ParseError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GdPlatform {
    Windows,
    MacOs,
}

impl GdPlatform {
    pub const ALL: [GdPlatform; 2] = [GdPlatform::Windows, GdPlatform::MacOs];

    pub fn depot_id(self) -> u32 {
        match self {
            GdPlatform::Windows => 322_171,
            GdPlatform::MacOs => 322_172,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            GdPlatform::Windows => "Windows",
            GdPlatform::MacOs => "macOS",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            GdPlatform::Windows => "windows",
            GdPlatform::MacOs => "macos",
        }
    }
}

impl fmt::Display for GdPlatform {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for GdPlatform {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        GdPlatform::ALL
            .into_iter()
            .find(|platform| text.eq_ignore_ascii_case(platform.as_str()))
            .ok_or_else(|| ParseError::new(format!("unknown platform \"{text}\"")))
    }
}
