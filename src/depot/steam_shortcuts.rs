use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, PartialEq, Eq)]
pub struct SteamShortcut {
    pub program: PathBuf,
    pub app_id: u32,
}

pub fn steam_shortcuts(steam_folder: &Path) -> Vec<SteamShortcut> {
    let mut shortcuts = Vec::new();
    let Ok(users) = fs::read_dir(steam_folder.join("userdata")) else {
        return shortcuts;
    };
    for user in users.filter_map(Result::ok) {
        let file = user.path().join("config").join("shortcuts.vdf");
        if let Ok(bytes) = fs::read(&file) {
            shortcuts.extend(shortcuts_from_vdf(&bytes));
        }
    }
    shortcuts
}

pub fn shortcut_compat_dir(
    steam_folder: &Path,
    shortcuts: &[SteamShortcut],
    game_dir: &Path,
) -> Option<PathBuf> {
    let game_dir = fs::canonicalize(game_dir).unwrap_or_else(|_| game_dir.to_path_buf());
    shortcuts
        .iter()
        .filter(|shortcut| {
            shortcut
                .program
                .parent()
                .map(|folder| fs::canonicalize(folder).unwrap_or_else(|_| folder.to_path_buf()))
                .is_some_and(|folder| folder == game_dir)
        })
        .map(|shortcut| {
            steam_folder
                .join("steamapps")
                .join("compatdata")
                .join(shortcut.app_id.to_string())
        })
        .find(|compat| compat.is_dir())
}

// binary keyvalues: type, name\0, value. 0 map, 1 string, 2 u32, 8 end
const MAP: u8 = 0x00;
const STRING: u8 = 0x01;
const INT32: u8 = 0x02;
const END: u8 = 0x08;

const SHORTCUT_DEPTH: usize = 2;

pub fn shortcuts_from_vdf(bytes: &[u8]) -> Vec<SteamShortcut> {
    let mut shortcuts = Vec::new();
    let mut reader = Reader { bytes, position: 0 };
    let mut program: Option<PathBuf> = None;
    let mut app_id: Option<u32> = None;
    let mut depth = 0_usize;
    while let Some(kind) = reader.byte() {
        match kind {
            MAP => {
                if reader.text().is_none() {
                    break;
                }
                depth += 1;
                if depth == SHORTCUT_DEPTH {
                    program = None;
                    app_id = None;
                }
            }
            STRING => {
                let (Some(key), Some(value)) = (reader.text(), reader.text()) else {
                    break;
                };
                if depth == SHORTCUT_DEPTH && key.eq_ignore_ascii_case("exe") {
                    program = Some(PathBuf::from(value.trim_matches('"')));
                }
            }
            INT32 => {
                let (Some(key), Some(value)) = (reader.text(), reader.int32()) else {
                    break;
                };
                if depth == SHORTCUT_DEPTH && key.eq_ignore_ascii_case("appid") {
                    app_id = Some(value);
                }
            }
            END => {
                if depth == SHORTCUT_DEPTH
                    && let (Some(program), Some(app_id)) = (program.take(), app_id.take())
                {
                    shortcuts.push(SteamShortcut { program, app_id });
                }
                depth = depth.saturating_sub(1);
            }
            _ => break,
        }
    }
    shortcuts
}

struct Reader<'bytes> {
    bytes: &'bytes [u8],
    position: usize,
}

impl Reader<'_> {
    fn byte(&mut self) -> Option<u8> {
        let byte = *self.bytes.get(self.position)?;
        self.position += 1;
        Some(byte)
    }

    fn text(&mut self) -> Option<String> {
        let rest = self.bytes.get(self.position..)?;
        let length = rest.iter().position(|&byte| byte == 0)?;
        self.position += length + 1;
        Some(String::from_utf8_lossy(&rest[..length]).into_owned())
    }

    fn int32(&mut self) -> Option<u32> {
        let field = self.bytes.get(self.position..self.position + 4)?;
        self.position += 4;
        Some(u32::from_le_bytes(field.try_into().ok()?))
    }
}
