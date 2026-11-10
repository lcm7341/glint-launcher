use weft::core::{Rect, Vec2};
use weft::prelude::{Align, Color, FontWeight, Grow, Insets, Sense, Ui, vec2};

use super::Window;
use crate::app::Action;
use crate::ui::icons::Icon;
use crate::ui::theme::{HOVERED, MUTED, PRESSED, TEXT};

const HEIGHT: f32 = 34.0;

const ICON: f32 = 16.0;

pub(super) fn show(ui: &mut Ui, window: &mut Window, system_buttons: bool) {
    let start = if system_buttons { 80.0 } else { 10.0 };
    let bar = ui
        .row()
        .width(Grow(1))
        .gap(8)
        .align(Align::Center)
        .padding(Insets::new(0.0, 0.0, 0.0, start))
        .style(|style| style.min_height(HEIGHT))
        .sense(Sense::CLICK_AND_DRAG)
        .show(|ui| {
            ui.image(&window.icon).size(ICON, ICON).name("Glint");
            ui.label(crate::TITLE)
                .font_size(12)
                .truncate()
                .style(|style| style.color(MUTED).weight(FontWeight::MEDIUM).width(Grow(1)));
            update_button(ui, window);
            if !system_buttons {
                window_buttons(ui);
            }
        })
        .response;
    if bar.double_clicked() {
        ui.toggle_maximized();
    } else if bar.drag_started() {
        ui.drag_window();
    }
}

fn update_button(ui: &mut Ui, window: &mut Window) {
    if let Some(version) = window.app.installing_update() {
        ui.label(format!("Updating to {version}…"))
            .font_size(12)
            .muted()
            .wrap(false);
        return;
    }
    let Some(version) = window.app.update_available() else {
        return;
    };
    let blocker = window.app.update_blocker();
    if ui
        .button(format!("Update to {version}"))
        .primary()
        .height(24)
        .enabled(blocker.is_none())
        .tooltip(blocker.unwrap_or("Download the new Glint and restart"))
        .clicked()
    {
        window.actions.push(Action::InstallUpdate);
    }
}

fn window_buttons(ui: &mut Ui) {
    ui.row().gap(0).show(|ui| {
        if window_button(ui, Icon::Minimize, "Minimize", HOVERED).clicked() {
            ui.set_minimized(true);
        }
        let (icon, name) = if ui.window_state().maximized {
            (Icon::Restore, "Restore")
        } else {
            (Icon::Maximize, "Maximize")
        };
        if window_button(ui, icon, name, HOVERED).clicked() {
            ui.toggle_maximized();
        }
        if window_button(ui, Icon::Close, "Close", Color::hex(0xC42B1C)).clicked() {
            ui.quit();
        }
    });
}

fn window_button(ui: &mut Ui, icon: Icon, name: &str, hovered: Color) -> weft::prelude::Response {
    ui.canvas()
        .size(46.0, HEIGHT)
        .sense(Sense::CLICK)
        .name(name)
        .show(|canvas, rect, response| {
            let fill = if response.pressed() {
                Some(if icon == Icon::Close {
                    hovered
                } else {
                    PRESSED
                })
            } else if response.hovered() {
                Some(hovered)
            } else {
                None
            };
            if let Some(fill) = fill {
                canvas.fill_rect(Rect::from_min_size(Vec2::ZERO, rect.size()), fill);
            }
            let color = if icon == Icon::Close && response.hovered() {
                Color::WHITE
            } else {
                TEXT
            };
            let side = 14.0;
            let at = vec2((rect.width() - side) / 2.0, (rect.height() - side) / 2.0);
            icon.draw(canvas, at, side, color);
        })
}
