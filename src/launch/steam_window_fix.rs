use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const FIX_DLL: &[u8] = include_bytes!("../../steam_fix/userenv.dll");

pub(crate) const STEAM_WINDOW_FLAGS: [&str; 2] =
    ["-cef-disable-gpu", "-cef-disable-gpu-compositing"];

pub(crate) fn place_fix_files(wine: &Path, prefix: &Path) -> io::Result<()> {
    let system32 = prefix.join("drive_c/windows/system32");
    fs::create_dir_all(&system32)?;
    let wines_own = wine_builtin_userenv(wine)
        .ok_or_else(|| io::Error::other("wine's userenv.dll is missing"))?;
    write_if_different(&system32.join("userenv_orig.dll"), &fs::read(wines_own)?)?;
    write_if_different(&system32.join("userenv.dll"), FIX_DLL)
}

fn wine_builtin_userenv(wine: &Path) -> Option<PathBuf> {
    let root = wine.parent()?.parent()?;
    let dll = root.join("lib/wine/x86_64-windows/userenv.dll");
    dll.is_file().then_some(dll)
}

fn write_if_different(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if fs::read(path).is_ok_and(|existing| existing == bytes) {
        return Ok(());
    }
    fs::write(path, bytes)
}
