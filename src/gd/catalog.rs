use std::cmp::Reverse;

use crate::gd::{CatalogEntry, CatalogError, GdPlatform, GdVersion, ManifestId};
use crate::parse_error::ParseError;

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Catalog {
    entries: Vec<CatalogEntry>,
}

impl Catalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bundled() -> Self {
        Self::from_text(include_str!("../../data/catalog.txt")).expect("bad catalog.txt")
    }

    pub fn from_text(text: &str) -> Result<Self, CatalogError> {
        let mut catalog = Self::new();
        let mut line_numbers = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let line_number = index + 1;
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let entry = parse_entry(line).map_err(|reason| CatalogError::Invalid {
                line_number,
                reason,
            })?;
            if let Some(position) = catalog.position(&entry.version, entry.platform) {
                return Err(CatalogError::Duplicate {
                    line_number,
                    first_line_number: line_numbers[position],
                    version: entry.version,
                    platform: entry.platform,
                });
            }
            catalog.entries.push(entry);
            line_numbers.push(line_number);
        }
        catalog.sort();
        Ok(catalog)
    }

    pub fn for_platform(&self, platform: GdPlatform) -> impl Iterator<Item = &CatalogEntry> {
        self.entries
            .iter()
            .filter(move |entry| entry.platform == platform)
    }

    pub fn newest(&self, platform: GdPlatform) -> Option<&CatalogEntry> {
        self.for_platform(platform).next()
    }

    pub fn find(&self, version: &GdVersion, platform: GdPlatform) -> Option<&CatalogEntry> {
        self.position(version, platform)
            .map(|position| &self.entries[position])
    }

    fn position(&self, version: &GdVersion, platform: GdPlatform) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.platform == platform && entry.version == *version)
    }

    fn sort(&mut self) {
        self.entries.sort_by(|left, right| {
            (left.platform, Reverse(&left.version)).cmp(&(right.platform, Reverse(&right.version)))
        });
    }
}

fn parse_entry(line: &str) -> Result<CatalogEntry, ParseError> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    let [version, platform, manifest] = fields[..] else {
        return Err(ParseError::new(format!(
            "expected \"version platform manifest\", got {} fields",
            fields.len()
        )));
    };
    Ok(CatalogEntry::new(
        version.parse()?,
        platform.parse()?,
        manifest.parse::<ManifestId>()?,
    ))
}
