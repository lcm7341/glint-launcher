use std::fmt;
use std::str::FromStr;

use crate::gd::ParseError;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ManifestId(u64);

impl fmt::Display for ManifestId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl FromStr for ManifestId {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(ParseError::new(format!("\"{text}\" isn't a manifest id")));
        }
        text.parse()
            .map(ManifestId)
            .map_err(|_| ParseError::new(format!("\"{text}\" is too big for a manifest id")))
    }
}
