#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod add_existing;
mod depot;
mod dialogs;
mod download;
mod folder_picker;
mod form;
mod gd;
mod geode_mods_view;
mod glint_theme;
mod icons;
mod instances;
mod job;
mod launch;
mod launcher;
mod markdown_view;
mod mods;
mod mods_view;
mod open_folder;
mod play;
mod qr_sign_in;
mod sentence;
mod window;

use crate::glint_theme::glint_theme;
use crate::launcher::Launcher;
use crate::window::Window;

pub(crate) const TITLE: &str = concat!("glint launcher | version ", env!("CARGO_PKG_VERSION"));

pub(crate) const ICON_RGBA: &[u8] = include_bytes!("../assets/icons/glint-64.rgba");
pub(crate) const ICON_SIDE: u32 = 64;

pub(crate) const SYSTEM_BUTTONS: bool = cfg!(target_os = "macos");

fn main() {
    let app = weft::App::new()
        .title(TITLE)
        // has to match glint.desktop for the taskbar icon
        .app_id("glint")
        .icon(ICON_RGBA.to_vec(), ICON_SIDE, ICON_SIDE)
        .decorations(false)
        .system_window_buttons(SYSTEM_BUTTONS)
        .resize_border(6.0)
        .size(1100.0, 720.0)
        .theme(glint_theme());
    let wake = app.wake_handle();
    let launcher = Launcher::new(move || {
        wake.wake();
    });
    let mut window = Window::new(launcher);
    app.run_ui(move |ui| window.build(ui))
        .expect("failed to open window");
}
