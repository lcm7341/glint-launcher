pub(in crate::platform) mod desktop;
pub(in crate::platform) mod launch;
pub(in crate::platform) mod paths;
pub(in crate::platform) mod processes;
pub(in crate::platform) mod steam;

pub(in crate::platform) fn native_game_command(
    game_dir: &std::path::Path,
) -> std::io::Result<std::process::Command> {
    launch::prepare(game_dir)?;
    Ok(launch::command(game_dir))
}
