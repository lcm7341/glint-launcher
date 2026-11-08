use crate::gd::{InstanceName, ParseError};

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub default_instance: Option<InstanceName>,
    pub steam_username: Option<String>,
    pub favorites: Vec<InstanceName>,
}

impl Settings {
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
        text
    }
}
