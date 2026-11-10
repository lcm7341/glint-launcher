use weft::prelude::{Align, Fixed, Grow, Insets, Ui};

use super::Window;
use crate::ui::dialogs::Dialog;
use crate::ui::play_button::play;
use crate::ui::theme::{HAIRLINE, RAISED};

pub(super) fn show(ui: &mut Ui, window: &mut Window) {
    ui.row()
        .width(Grow(1))
        .gap(4)
        .align(Align::Center)
        .padding(Insets::symmetric(6.0, 8.0))
        .show(|ui| {
            let play = play(&window.app, window.chosen_instance());
            let button = ui
                .button(play.label)
                .primary()
                .enabled(play.click.is_some())
                .width(Fixed(112.0))
                .show();
            if !play.hint.is_empty() {
                ui.tooltip_for(&button, play.hint);
            }
            if button.clicked()
                && let Some(click) = play.click
            {
                window.click(click);
            }

            divider(ui);
            if ui
                .button("New…")
                .muted()
                .name("New instance")
                .tooltip("New instance  Ctrl+N")
                .clicked()
            {
                window.dialog = Some(Dialog::new_instance(&window.app));
            }
            if ui
                .button("Add existing…")
                .muted()
                .name("Add existing")
                .tooltip("Add an existing GD folder")
                .clicked()
            {
                window.app.refresh_found();
                window.dialog = Some(Dialog::AddExisting(Default::default()));
            }
            ui.spacer();
            ui.text_input(&mut window.filter)
                .id(super::filter_id())
                .name("Search instances")
                .placeholder("Filter  Ctrl+F")
                .width(Fixed(220.0));
            divider(ui);
            let showing_log = window.show_log;
            let log = ui
                .button("Log")
                .muted()
                .name("App log")
                .tooltip("Log")
                .style(|style| if showing_log { style.bg(RAISED) } else { style })
                .show();
            if log.clicked() && ui.once_per_frame(ui.child_id("log switch")) {
                window.show_log = !window.show_log;
            }
        });
}

fn divider(ui: &mut Ui) {
    ui.canvas().size(9.0, 18.0).show(|canvas, rect, _| {
        let middle = (rect.width() / 2.0).floor() + 0.5;
        canvas.fill_rect(
            weft::core::Rect::from_min_size(
                weft::prelude::vec2(middle - 0.5, 0.0),
                weft::prelude::vec2(1.0, rect.height()),
            ),
            HAIRLINE,
        );
    });
}
