use crate::paths::DataDir;
use std::{fs, io};

use crate::instances::InstanceName;
use crate::parse_error::ParseError;

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub default_instance: Option<InstanceName>,
    pub steam_username: Option<String>,
    pub favorites: Vec<InstanceName>,
    pub new_instance_save: NewInstanceSave,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NewInstanceSave {
    Link,
    #[default]
    Own,
    Ask,
}

impl NewInstanceSave {
    pub const ALL: [Self; 3] = [Self::Link, Self::Own, Self::Ask];

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Link => "Main save",
            Self::Own => "Per-instance",
            Self::Ask => "Ask",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Link => "link",
            Self::Own => "own",
            Self::Ask => "ask",
        }
    }
}

impl Settings {
    pub fn load(data_dir: &DataDir) -> Self {
        fs::read_to_string(data_dir.settings_path())
            .ok()
            .and_then(|text| Self::from_text(&text).ok())
            .unwrap_or_default()
    }
    pub fn save(&self, data_dir: &DataDir) -> io::Result<()> {
        fs::create_dir_all(data_dir.root())?;
        fs::write(data_dir.settings_path(), self.to_text())
    }

    pub fn from_text(text: &str) -> Result<Self, ParseError> {
        let mut settings = Self::default();
        for (index, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once(char::is_whitespace) else {
                continue;
            };
            let value = value.trim();
            match key {
                "default-instance" => {
                    settings.default_instance = Some(value.parse().map_err(|error| {
                        ParseError::new(format!("line {}: {error}", index + 1))
                    })?);
                }
                "steam-username" => settings.steam_username = Some(value.to_owned()),
                "new-instance-save" => {
                    settings.new_instance_save = NewInstanceSave::ALL
                        .into_iter()
                        .find(|save| save.key() == value)
                        .ok_or_else(|| {
                            ParseError::new(format!(
                                "line {}: new-instance-save is \"{value}\", not link, own or ask",
                                index + 1
                            ))
                        })?;
                }
                "favorite" => settings.favorites.push(
                    value
                        .parse()
                        .map_err(|error| ParseError::new(format!("line {}: {error}", index + 1)))?,
                ),
                _ => {}
            }
        }
        Ok(settings)
    }

    pub fn to_text(&self) -> String {
        let mut text = String::new();
        if let Some(name) = &self.default_instance {
            text.push_str(&format!("default-instance {name}\n"));
        }
        if let Some(username) = &self.steam_username {
            text.push_str(&format!("steam-username {username}\n"));
        }
        for favorite in &self.favorites {
            text.push_str(&format!("favorite {favorite}\n"));
        }
        if self.new_instance_save != NewInstanceSave::default() {
            let save = self.new_instance_save.key();
            text.push_str(&format!("new-instance-save {save}\n"));
        }
        text
    }
}
