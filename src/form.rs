use crate::gd::ParseError;
use weft::prelude::{Align, Fixed, Grow, Ui};

use crate::sentence::capitalized;

pub(crate) const DIALOG_WIDTH: f32 = 420.0;

const LABEL_WIDTH: f32 = 112.0;

pub(crate) fn show_form(ui: &mut Ui, fields: impl FnOnce(&mut Ui)) {
    ui.grid()
        .columns([Fixed(LABEL_WIDTH), Grow(1)])
        .column_gap(12)
        .row_gap(8)
        .show(fields);
}

pub(crate) fn field_label(ui: &mut Ui, text: &str) {
    ui.label(text)
        .style(|style| style.align_self(Align::Center));
}

pub(crate) fn empty_label(ui: &mut Ui) {
    ui.label(String::new());
}

pub(crate) fn show_field_hint(ui: &mut Ui, error: Option<&ParseError>) {
    let text = error.map_or_else(String::new, |error| {
        let message = error.message();
        let reason = message
            .split_once(": ")
            .map_or(message, |(_, reason)| reason);
        capitalized(reason)
    });
    ui.label(text)
        .muted()
        .font_size(12)
        .style(|style| style.min_height(18.0));
}
