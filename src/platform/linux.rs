pub(in crate::platform) mod desktop;
pub(in crate::platform) mod file_picker;
pub(in crate::platform) mod paths;
pub(in crate::platform) mod processes;
pub(in crate::platform) mod steam;

pub(in crate::platform) fn native_game_command(
    _game_dir: &std::path::Path,
    _home: Option<&std::path::Path>,
) -> std::io::Result<std::process::Command> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Geometry Dash has no native Linux build",
    ))
}
