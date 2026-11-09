use weft::prelude::{Grow, Insets, Ui};
use weft::text::FontFamily;

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
            let mut list = ui
                .virtual_list(log.len())
                .id(ui.child_id("log"))
                .row_height(18.0);
            // ScrollTarget::Bottom doesn't work in a virtual list
            if launcher.log_added() != seen {
                list = list.scroll_to(log.len() - 1);
            }
            list.name("Launcher log")
                .width(Grow(1))
                .height(Grow(1))
                .show(|ui, index| {
                    ui.label(log[index].clone())
                        .wrap(false)
                        .font_size(12)
                        .style(|style| style.font_family(FontFamily::Monospace));
                });
        });
    launcher.log_added()
}
