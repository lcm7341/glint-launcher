use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const FIX_DLL: &[u8] = include_bytes!("../../../steam_fix/userenv.dll");

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

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProgramOverride {
    program: &'static str,
    dll: &'static str,
    value: &'static str,
}

pub(crate) const PROGRAM_OVERRIDES: [ProgramOverride; 2] = [
    ProgramOverride {
        program: "steamwebhelper.exe",
        dll: "userenv",
        value: "native,builtin",
    },
    // steam's overlay hook hangs gd's opengl under winehq
    ProgramOverride {
        program: "GeometryDash.exe",
        dll: "gameoverlayrenderer",
        value: "",
    },
];

impl ProgramOverride {
    pub(crate) fn reg_add_args(&self) -> Vec<String> {
        vec![
            "reg".to_owned(),
            "add".to_owned(),
            format!(
                r"HKCU\Software\Wine\AppDefaults\{}\DllOverrides",
                self.program
            ),
            "/v".to_owned(),
            self.dll.to_owned(),
            "/t".to_owned(),
            "REG_SZ".to_owned(),
            "/d".to_owned(),
            self.value.to_owned(),
            "/f".to_owned(),
        ]
    }

    pub(crate) fn is_in(&self, prefix: &Path) -> bool {
        let Ok(user_reg) = fs::read_to_string(prefix.join("user.reg")) else {
            return false;
        };
        // user.reg doubles backslashes in key names
        let key = format!(
            r"[Software\\Wine\\AppDefaults\\{}\\DllOverrides]",
            self.program
        );
        let value = format!(r#""{}"="{}""#, self.dll, self.value);
        user_reg.split_once(&key).is_some_and(|(_, after)| {
            let section = after.split("\n[").next().unwrap_or_default();
            section.lines().any(|line| line.trim() == value)
        })
    }
}
