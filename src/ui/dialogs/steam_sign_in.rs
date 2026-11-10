use weft::core::Rect;
use weft::prelude::{Align, Color, Fixed, FontWeight, Grow, Justify, Ui, vec2};

use crate::app::{Action, App};
use crate::ui::theme::TEXT;

pub(crate) fn qr_dialog(ui: &mut Ui, launcher: &App, actions: &mut Vec<Action>) {
    let Some(code) = launcher.download().and_then(|download| download.qr_code()) else {
        return;
    };
    let modules = trimmed(code);
    let mut is_open = true;
    ui.modal(&mut is_open)
        .title("Sign in to Steam")
        .dismissible(false)
        .style(|style| style.width(Fixed(380.0)))
        .show(|ui| {
            ui.column()
                .gap(14)
                .width(Grow(1))
                .align(Align::Center)
                .show(|ui| {
                    ui.label("Scan with the Steam app:")
                        .style(|style| style.color(TEXT));
                    draw_code(ui, &modules);
                    ui.label("Steam Guard  ›  Scan a QR code, then approve")
                        .style(|style| style.weight(FontWeight::SEMI_BOLD).color(TEXT));
                    ui.label("The code changes every few seconds.")
                        .muted()
                        .font_size(12);
                    ui.row().width(Grow(1)).justify(Justify::End).show(|ui| {
                        if ui.button("Cancel").clicked() {
                            actions.push(Action::CancelDownload);
                        }
                    });
                });
        });
}

fn trimmed(code: &[Vec<bool>]) -> Vec<Vec<bool>> {
    let dark_columns = || {
        code.iter()
            .flat_map(|row| row.iter().enumerate().filter(|(_, dark)| **dark))
    };
    let left = dark_columns().map(|(column, _)| column).min().unwrap_or(0);
    let right = dark_columns().map(|(column, _)| column).max().unwrap_or(0);
    code.iter()
        .filter(|row| row.contains(&true))
        .map(|row| {
            (left..=right)
                .map(|column| row.get(column).copied().unwrap_or(false))
                .collect()
        })
        .collect()
}

fn draw_code(ui: &mut Ui, modules: &[Vec<bool>]) {
    let count = modules.len().max(1) + 4;
    ui.canvas()
        .size(232.0, 232.0)
        .name("Steam sign-in QR code")
        .show(|canvas, rect, _| {
            canvas.fill_rect(
                Rect::from_min_size(vec2(0.0, 0.0), rect.size()),
                Color::WHITE,
            );
            // whole pixels, no seams
            let module = (rect.width() / count as f32).floor().max(1.0);
            let offset = ((rect.width() - module * count as f32) / 2.0).floor();
            for (row, cells) in modules.iter().enumerate() {
                for (column, dark) in cells.iter().enumerate() {
                    if *dark {
                        let at = vec2(
                            offset + (column + 2) as f32 * module,
                            offset + (row + 2) as f32 * module,
                        );
                        let square = Rect::from_min_size(at, vec2(module, module));
                        canvas.fill_rect(square, Color::BLACK);
                    }
                }
            }
        });
}
