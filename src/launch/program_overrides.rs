use std::fs;
use std::path::Path;

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
