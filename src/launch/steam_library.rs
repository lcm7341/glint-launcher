use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::gd::{GD_APP_ID, steam_value};

const STEAMAPPS: &str = "drive_c/Program Files (x86)/Steam/steamapps";

const GAME_DIR: &str = "Geometry Dash";

const STEAMS_OWN_DIR: &str = "Geometry Dash (Steam)";

const MARKER: &str = ".glint-instance";

#[derive(Clone, PartialEq, Eq)]
pub struct SteamLibrary {
    steamapps: PathBuf,
}

impl SteamLibrary {
    pub fn new(prefix: &Path) -> Self {
        Self {
            steamapps: prefix.join(STEAMAPPS),
        }
    }

    pub fn has_gd(&self) -> bool {
        fs::read_to_string(self.manifest()).is_ok_and(|manifest| state_flags(&manifest) == Some(4))
            && self.game_dir().join("GeometryDash.exe").is_file()
    }

    pub fn is_installing_gd(&self) -> bool {
        self.manifest().is_file() && !self.has_gd()
    }

    pub fn queue_gd_install(&self) -> io::Result<()> {
        fs::create_dir_all(&self.steamapps)?;
        let manifest = format!(
            "\"AppState\"\n{{\n\t\"appid\"\t\t\"{GD_APP_ID}\"\n\t\"Universe\"\t\t\"1\"\n\t\
             \"name\"\t\t\"Geometry Dash\"\n\t\"StateFlags\"\t\t\"1026\"\n\t\
             \"installdir\"\t\t\"{GAME_DIR}\"\n\t\"AutoUpdateBehavior\"\t\t\"1\"\n}}\n"
        );
        fs::write(self.manifest(), manifest)
    }

    pub fn put_instance(&self, game_dir: &Path, instance: &str) -> io::Result<()> {
        let dir = self.game_dir();
        let marker = dir.join(MARKER);
        let holds = fs::read_to_string(&marker).ok();
        if holds.as_deref() == Some(instance) && same_program(game_dir, &dir) {
            return Ok(());
        }
        if dir.exists() {
            if holds.is_some() {
                fs::remove_dir_all(&dir)?;
            } else {
                let steams_own = self.steamapps.join("common").join(STEAMS_OWN_DIR);
                if steams_own.exists() {
                    fs::remove_dir_all(&dir)?;
                } else {
                    fs::rename(&dir, steams_own)?;
                }
            }
        }
        copy_game(game_dir, &dir)?;
        fs::write(marker, instance)
    }

    pub fn applaunch_args() -> [String; 2] {
        ["-applaunch".to_owned(), GD_APP_ID.to_string()]
    }

    fn manifest(&self) -> PathBuf {
        self.steamapps.join(format!("appmanifest_{GD_APP_ID}.acf"))
    }

    fn game_dir(&self) -> PathBuf {
        self.steamapps.join("common").join(GAME_DIR)
    }
}

pub(crate) fn is_steam_gd_running() -> bool {
    Command::new("ps")
        .args(["-axo", "command="])
        .output()
        .is_ok_and(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(is_steam_gd)
        })
}

fn is_steam_gd(command_line: &str) -> bool {
    let lower = command_line.to_ascii_lowercase();
    lower.contains(r"steamapps\common\geometry dash\geometrydash.exe")
}

fn state_flags(manifest: &str) -> Option<u32> {
    steam_value(manifest, "StateFlags")?.parse().ok()
}

fn same_program(game_dir: &Path, copy: &Path) -> bool {
    let size = |dir: &Path| fs::metadata(dir.join("GeometryDash.exe")).map(|meta| meta.len());
    matches!((size(game_dir), size(copy)), (Ok(a), Ok(b)) if a == b)
}

fn copy_game(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_text = name.to_string_lossy();
        if name_text.starts_with('.') || name_text.eq_ignore_ascii_case("steam_appid.txt") {
            continue;
        }
        let target = to.join(&name);
        if entry.file_type()?.is_dir() {
            copy_game(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
