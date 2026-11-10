use std::path::PathBuf;

use crate::gd::{GdPlatform, GdVersion};
use crate::parse_error::ParseError;

#[derive(Clone, PartialEq, Eq)]
pub struct InstanceConfig {
    pub version: GdVersion,
    pub platform: GdPlatform,
    pub game_folder: Option<PathBuf>,
    /// uses the main save
    pub linked: bool,
}

impl InstanceConfig {
    pub fn new(version: GdVersion, platform: GdPlatform) -> Self {
        Self {
            version,
            platform,
            game_folder: None,
            linked: false,
        }
    }

    /// imported installs start linked
    #[must_use]
    pub fn with_game_folder(mut self, folder: impl Into<PathBuf>) -> Self {
        self.game_folder = Some(folder.into());
        self.linked = true;
        self
    }

    #[must_use]
    pub fn with_linked(mut self, linked: bool) -> Self {
        self.linked = linked;
        self
    }

    pub fn from_text(text: &str) -> Result<Self, ParseError> {
        let mut version = None;
        let mut platform = None;
        let mut game_folder = None;
        let mut linked = None;
        for (index, line) in text.lines().enumerate() {
            let line_number = index + 1;
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let at_line =
                |error: ParseError| ParseError::new(format!("line {line_number}: {error}"));
            let (key, value) = line
                .split_once(char::is_whitespace)
                .ok_or_else(|| at_line(ParseError::new(format!("\"{line}\" has no value"))))?;
            let value = value.trim();
            match key {
                "version" => version = Some(value.parse().map_err(at_line)?),
                "platform" => platform = Some(value.parse().map_err(at_line)?),
                "game-folder" => game_folder = Some(PathBuf::from(value)),
                "save" => {
                    linked = Some(match value {
                        "linked" => true,
                        "own" => false,
                        _ => {
                            return Err(at_line(ParseError::new(format!(
                                "save is \"{value}\", not linked or own"
                            ))));
                        }
                    });
                }
                _ => {
                    return Err(at_line(ParseError::new(format!(
                        "unknown setting \"{key}\""
                    ))));
                }
            }
        }
        let missing = |key: &str| ParseError::new(format!("missing {key}"));
        Ok(Self {
            version: version.ok_or_else(|| missing("version"))?,
            platform: platform.ok_or_else(|| missing("platform"))?,
            // old configs: imported means linked
            linked: linked.unwrap_or(game_folder.is_some()),
            game_folder,
        })
    }

    pub fn to_text(&self) -> String {
        let mut text = format!("version {}\nplatform {}\n", self.version, self.platform);
        if let Some(folder) = &self.game_folder {
            text.push_str(&format!("game-folder {}\n", folder.display()));
        }
        let save = if self.linked { "linked" } else { "own" };
        text.push_str(&format!("save {save}\n"));
        text
    }
}
