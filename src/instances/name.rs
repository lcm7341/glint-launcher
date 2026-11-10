use std::fmt;
use std::str::FromStr;

use crate::parse_error::ParseError;

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InstanceName(String);

const MAX_LENGTH: usize = 64;
const FORBIDDEN: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

impl InstanceName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for InstanceName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for InstanceName {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let reject = |reason: &str| {
            Err(ParseError::new(format!(
                "\"{text}\" can't be an instance name: {reason}"
            )))
        };
        if text.is_empty() {
            return Err(ParseError::new("name can't be empty"));
        }
        if text.chars().count() > MAX_LENGTH {
            return reject(&format!("max {MAX_LENGTH} characters"));
        }
        if let Some(character) = text.chars().find(|character| FORBIDDEN.contains(character)) {
            return reject(&format!("can't contain {character}"));
        }
        if text.chars().any(char::is_control) {
            return reject("no control characters");
        }
        if text.starts_with([' ', '.']) || text.ends_with([' ', '.']) {
            return reject("can't start or end with a space or dot");
        }
        let stem = text.split('.').next().unwrap_or(text);
        if RESERVED
            .iter()
            .any(|reserved| stem.eq_ignore_ascii_case(reserved))
        {
            return reject("reserved name on windows");
        }
        Ok(Self(text.to_owned()))
    }
}
