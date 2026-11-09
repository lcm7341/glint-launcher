use std::fs::File;
use std::io::Read;
use std::path::Path;

const STEAM_STUB_SECTION: &[u8; 8] = b".bind\0\0\0";

pub fn has_steam_drm(exe: &Path) -> bool {
    let mut header = vec![0; 4096];
    let Ok(read) = File::open(exe).and_then(|mut file| file.read(&mut header)) else {
        return false;
    };
    header.truncate(read);
    section_names(&header).is_some_and(|names| names.contains(STEAM_STUB_SECTION))
}

fn section_names(header: &[u8]) -> Option<Vec<[u8; 8]>> {
    let u16_at = |at: usize| Some(u16::from_le_bytes(header.get(at..at + 2)?.try_into().ok()?));
    let u32_at = |at: usize| Some(u32::from_le_bytes(header.get(at..at + 4)?.try_into().ok()?));
    if header.get(..2)? != b"MZ" {
        return None;
    }
    let pe = u32_at(0x3c)? as usize;
    if header.get(pe..pe + 4)? != b"PE\0\0" {
        return None;
    }
    let sections = u16_at(pe + 6)? as usize;
    let optional_header = u16_at(pe + 20)? as usize;
    let table = pe + 24 + optional_header;
    (0..sections)
        .map(|i| {
            header
                .get(table + i * 40..table + i * 40 + 8)?
                .try_into()
                .ok()
        })
        .collect()
}
