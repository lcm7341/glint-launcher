use std::env;
use std::path::{Path, PathBuf};

pub(crate) fn capitalized(text: &str) -> String {
    let mut characters = text.chars();
    characters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(characters).collect()
    })
}

pub(crate) fn home_relative(path: &Path) -> String {
    match env::var_os("HOME").and_then(|home| path.strip_prefix(home).ok().map(PathBuf::from)) {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}
