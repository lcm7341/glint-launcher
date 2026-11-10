use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

use crate::parse_error::ParseError;

// decimal: 2.113 < 2.2 < 2.2074
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GdVersion {
    major: u16,
    fraction: String,
}

impl Ord for GdVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then_with(|| self.fraction.cmp(&other.fraction))
    }
}

impl PartialOrd for GdVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for GdVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fraction = if self.fraction.is_empty() {
            "0"
        } else {
            &self.fraction
        };
        write!(formatter, "{}.{fraction}", self.major)
    }
}

impl FromStr for GdVersion {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let invalid = || ParseError::new(format!("\"{text}\" isn't a GD version"));
        let (major_text, fraction_text) = text.split_once('.').ok_or_else(invalid)?;
        let is_digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
        if !is_digits(major_text) || !is_digits(fraction_text) {
            return Err(invalid());
        }
        let major = major_text.parse().map_err(|_| invalid())?;
        let fraction = fraction_text.trim_end_matches('0').to_owned();
        Ok(Self { major, fraction })
    }
}
