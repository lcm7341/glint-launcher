use weft::prelude::{Grow, Insets, Ui};
use weft::text::FontFamily;
use weft::ui::ScrollTarget;

use crate::glint_theme::WELL;
use crate::launcher::Launcher;

pub(super) fn show(ui: &mut Ui, launcher: &Launcher, seen: usize) -> usize {
    let log = launcher.log();
    ui.column()
        .padding(Insets::symmetric(10.0, 14.0))
        .gap(8)
        .width(Grow(1))
        .height(Grow(1))
        .style(|style| style.bg(WELL))
        .show(|ui| {
            if log.is_empty() {
                ui.label("Nothing yet.").muted();
                return;
            }
            let area = ui.child_id("log");
            ui.scroll_area()
                .id(area)
                .name("Launcher log")
                .height(Grow(1))
                .show(|ui| {
                    for (index, line) in log.iter().enumerate() {
                        ui.push_id(index, |ui| {
                            ui.label(line.clone())
                                .wrap(false)
                                .font_size(12)
                                .style(|style| style.font_family(FontFamily::Monospace));
                        });
                    }
                });
            if log.len() != seen {
                ui.scroll_to(area, ScrollTarget::Bottom);
            }
        });
    log.len()
}
