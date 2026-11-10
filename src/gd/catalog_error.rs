use std::fmt;

use crate::gd::{GdPlatform, GdVersion};
use crate::parse_error::ParseError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    Invalid {
        line_number: usize,
        reason: ParseError,
    },
    Duplicate {
        line_number: usize,
        first_line_number: usize,
        version: GdVersion,
        platform: GdPlatform,
    },
}

impl fmt::Display for CatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CatalogError::Invalid {
                line_number,
                reason,
            } => {
                write!(formatter, "line {line_number} of the catalog: {reason}")
            }
            CatalogError::Duplicate {
                line_number,
                first_line_number,
                version,
                platform,
            } => {
                write!(
                    formatter,
                    "line {line_number} of the catalog repeats {version} {platform} from line {first_line_number}"
                )
            }
        }
    }
}
