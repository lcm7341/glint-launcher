#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod depot;
mod format;
mod gd;
mod instances;
mod launch;
mod mods;
mod parse_error;
mod paths;
mod platform;
mod settings;
mod steam;
mod task;
mod ui;
mod update;

use crate::app::App;
use crate::ui::theme::glint_theme;
use crate::ui::window::Window;

pub(crate) const TITLE: &str = concat!("glint launcher | version ", env!("CARGO_PKG_VERSION"));

pub(crate) const ICON_RGBA: &[u8] = include_bytes!("../assets/icons/glint-64.rgba");
pub(crate) const ICON_SIDE: u32 = 64;

pub(crate) const SYSTEM_BUTTONS: bool =
    matches!(platform::HostOs::CURRENT, platform::HostOs::MacOs);

fn main() {
    let ui_app = weft::App::new()
        .title(TITLE)
        // matches glint.desktop
        .app_id("glint")
        .icon(ICON_RGBA.to_vec(), ICON_SIDE, ICON_SIDE)
        .decorations(false)
        .system_window_buttons(SYSTEM_BUTTONS)
        .resize_border(6.0)
        .size(1100.0, 720.0)
        .theme(glint_theme());
    let wake = ui_app.wake_handle();
    let app = App::new(move || {
        wake.wake();
    });
    let mut window = Window::new(app);
    ui_app
        .run_ui(move |ui| window.build(ui))
        .expect("failed to open window");
}

