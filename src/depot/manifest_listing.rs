use std::path::PathBuf;

use crate::gd::CatalogEntry;

#[derive(Clone, PartialEq, Eq, Default)]
pub struct ManifestListing {
    entries: Vec<ListedEntry>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ListedEntry {
    pub path: PathBuf,
    pub size: u64,
    pub is_directory: bool,
}

impl ManifestListing {
    pub fn from_text(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        lines.find(|line| line.trim_start().starts_with("Size Chunks File SHA"))?;
        let entries = lines.filter_map(parse_entry).collect();
        Some(Self { entries })
    }

    pub fn file_name(entry: &CatalogEntry) -> String {
        format!("manifest_{}_{}.txt", entry.depot_id(), entry.manifest)
    }

    pub fn entries(&self) -> &[ListedEntry] {
        &self.entries
    }
}

fn parse_entry(line: &str) -> Option<ListedEntry> {
    let rest = line.trim_end_matches('\r');
    let (size, rest) = next_field(rest)?;
    let (_chunks, rest) = next_field(rest)?;
    let (_sha, rest) = next_field(rest)?;
    let (flags, rest) = next_field(rest)?;
    let size = size.parse().ok()?;
    let flags = u32::from_str_radix(flags, 16).ok()?;
    let name = rest.strip_prefix(' ')?;
    let path = relative_path(name)?;
    Some(ListedEntry {
        path,
        size,
        is_directory: flags & 0x40 != 0,
    })
}

fn next_field(line: &str) -> Option<(&str, &str)> {
    let line = line.trim_start_matches(' ');
    let end = line.find(' ')?;
    Some((&line[..end], &line[end..]))
}

fn relative_path(name: &str) -> Option<PathBuf> {
    let mut path = PathBuf::new();
    for part in name.split(['\\', '/']) {
        if part.is_empty() || part == "." || part == ".." || part.contains(':') {
            return None;
        }
        path.push(part);
    }
    Some(path)
}
