use std::fs;
use std::path::{Path, PathBuf};

use crate::mods::{DllMod, list_dll_mods};

const MARKER: &[u8] = b"echo_auto_loader";

#[derive(Clone, PartialEq, Eq)]
pub struct EchoMod {
    pub dll: DllMod,
    pub has_loader: bool,
}

pub fn echo_folder(game_dir: &Path) -> PathBuf {
    game_dir.join(".echo")
}

pub fn find_echo(game_dir: &Path) -> Option<EchoMod> {
    let dll = list_dll_mods(&echo_folder(game_dir))
        .ok()?
        .into_iter()
        .filter_map(|found| echo_version(&found.name).map(|version| (version, found)))
        .max_by(|(left, _), (right, _)| left.cmp(right))?
        .1;
    Some(EchoMod {
        dll,
        has_loader: has_echo_loader(game_dir),
    })
}

fn has_echo_loader(game_dir: &Path) -> bool {
    ["XInput9_1_0.dll", "XInput9_1_0.dll.glint-backup"]
        .iter()
        .any(|file| {
            fs::read(game_dir.join(file))
                .is_ok_and(|bytes| bytes.windows(MARKER.len()).any(|window| window == MARKER))
        })
}

fn echo_version(name: &str) -> Option<Vec<u64>> {
    let version = name.strip_prefix("Echo v")?;
    version
        .split('.')
        .map(|part| part.parse().ok())
        .collect::<Option<Vec<u64>>>()
}
