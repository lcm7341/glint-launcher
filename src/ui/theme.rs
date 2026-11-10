use weft::prelude::{BoxShadow, Color, Style, Theme, Transition, Variant, WidgetKind, vec2};
use weft::ui::Border;

pub(crate) fn glint_theme() -> Theme {
    let mut tokens = Theme::dark().tokens;
    tokens.colors.background = BACKGROUND;
    tokens.colors.surface = WELL;
    tokens.colors.surface_raised = RAISED;
    tokens.colors.text = TEXT;
    tokens.colors.text_muted = MUTED;
    tokens.colors.accent = ACCENT;
    tokens.colors.on_accent = ON_ACCENT;
    tokens.colors.border = HAIRLINE;
    tokens.colors.focus_ring = ACCENT;
    tokens.colors.danger = DANGER;
    tokens.colors.selection = ACCENT.with_alpha(0.2);
    tokens.radii.sm = 3.0;
    tokens.radii.md = 4.0;
    tokens.radii.lg = 6.0;
    tokens.control_height = 28.0;
    tokens.focus_ring_width = 1.0;
    let mut theme = Theme::from_tokens(tokens);
    restyle(&mut theme);
    theme
}

pub(crate) const BACKGROUND: Color = Color::hex(0x1E1E21);
pub(crate) const WELL: Color = Color::hex(0x141416);
pub(crate) const RAISED: Color = Color::hex(0x2A2A2E);
pub(crate) const HOVERED: Color = Color::hex(0x333338);
pub(crate) const PRESSED: Color = Color::hex(0x3B3B41);
pub(crate) const TEXT: Color = Color::hex(0xE4E4E7);
pub(crate) const MUTED: Color = Color::hex(0x95959D);
pub(crate) const ACCENT: Color = Color::hex(0xF5C04A);
pub(crate) const ON_ACCENT: Color = Color::hex(0x1C1500);
pub(crate) const HAIRLINE: Color = Color::hex(0x313136);
const OUTLINE: Color = Color::hex(0x3A3A40);
const DANGER: Color = Color::hex(0xFF6B7A);

const QUICK: Transition = Transition::millis(80);

fn restyle(theme: &mut Theme) {
    restyle_buttons(theme);
    restyle_fields(theme);
    restyle_overlays(theme);
    restyle_surfaces(theme);
}

fn restyle_buttons(theme: &mut Theme) {
    theme.restyle(
        WidgetKind::Button,
        Style::new()
            .bg(RAISED)
            .border(Border::new(1.0, OUTLINE))
            .radius(4)
            .padding(weft::prelude::Insets::symmetric(0.0, 12.0))
            .color(TEXT)
            .transition(QUICK)
            .hover(|state| state.bg(HOVERED))
            .active(|state| state.bg(PRESSED))
            .disabled(|state| state.bg(BACKGROUND).color(MUTED.with_alpha(0.5))),
    );
    theme.restyle_variant(
        WidgetKind::Button,
        Variant::PRIMARY,
        Style::new()
            .bg(ACCENT)
            .color(ON_ACCENT)
            .border(Border::new(1.0, ACCENT))
            .bold()
            .hover(|state| state.bg(Color::hex(0xF8CE6E)))
            .active(|state| state.bg(ACCENT))
            .disabled(|state| {
                state
                    .bg(RAISED)
                    .color(MUTED.with_alpha(0.5))
                    .border(Border::new(1.0, OUTLINE))
            }),
    );
    theme.restyle_variant(
        WidgetKind::Button,
        Variant::MUTED,
        Style::new()
            .bg(Color::TRANSPARENT)
            .border(Border::new(1.0, Color::TRANSPARENT))
            .hover(|state| state.bg(HOVERED).border(Border::new(1.0, OUTLINE)))
            .active(|state| state.bg(PRESSED))
            .disabled(|state| state.bg(Color::TRANSPARENT).color(MUTED.with_alpha(0.5))),
    );
    theme.restyle_variant(
        WidgetKind::Button,
        Variant::DANGER,
        Style::new()
            .bg(DANGER.with_alpha(0.14))
            .color(DANGER)
            .border(Border::new(1.0, DANGER.with_alpha(0.4)))
            .hover(|state| state.bg(DANGER.with_alpha(0.24)))
            .active(|state| state.bg(DANGER.with_alpha(0.3))),
    );
}

fn restyle_fields(theme: &mut Theme) {
    for kind in [WidgetKind::TextInput, WidgetKind::Combo] {
        theme.restyle(
            kind,
            Style::new()
                .bg(WELL)
                .border(Border::new(1.0, OUTLINE))
                .radius(4)
                .transition(QUICK)
                .focus(|state| state.border(Border::new(1.0, ACCENT))),
        );
    }

    theme.restyle(
        WidgetKind::Segmented,
        Style::new()
            .bg(WELL)
            .border(Border::new(1.0, OUTLINE))
            .radius(4),
    );
    theme.restyle(
        WidgetKind::SegmentIndicator,
        Style::new().bg(ACCENT).radius(3),
    );
}

fn restyle_overlays(theme: &mut Theme) {
    let float = |fill: Color, radius: f32| {
        Style::new()
            .bg(fill)
            .border(Border::new(1.0, OUTLINE))
            .radius(radius)
            .shadow(BoxShadow::new(
                vec2(0.0, 4.0),
                12.0,
                Color::BLACK.with_alpha(0.45),
            ))
    };
    theme.restyle(WidgetKind::Dialog, float(BACKGROUND, 6.0));
    for kind in [WidgetKind::Popup, WidgetKind::Toast] {
        theme.restyle(kind, float(RAISED, 4.0));
    }
    theme.restyle(WidgetKind::Tooltip, float(PRESSED, 3.0).color(TEXT));
    theme.restyle(
        WidgetKind::Scrim,
        Style::new().bg(Color::BLACK.with_alpha(0.45)),
    );
    for kind in [WidgetKind::MenuItem, WidgetKind::ComboItem] {
        theme.restyle(
            kind,
            Style::new()
                .radius(3)
                .transition(QUICK)
                .hover(|state| state.bg(ACCENT.with_alpha(0.2))),
        );
    }
}

fn restyle_surfaces(theme: &mut Theme) {
    for kind in [WidgetKind::Column, WidgetKind::Row] {
        theme.restyle_variant(
            kind,
            Variant::CARD,
            Style::new()
                .bg(WELL)
                .border(Border::new(1.0, HAIRLINE))
                .radius(4),
        );
    }
    theme.restyle(
        WidgetKind::Table,
        Style::new().bg(WELL).border(Border::none()).radius(0),
    );

    theme.restyle(WidgetKind::Separator, Style::new().bg(HAIRLINE));
    theme.restyle(
        WidgetKind::Splitter,
        Style::new()
            .bg(BACKGROUND)
            .border(Border::new(
                weft::prelude::Insets::new(1.0, 0.0, 1.0, 0.0),
                HAIRLINE,
            ))
            .hover(|state| state.bg(HOVERED))
            .active(|state| state.bg(PRESSED)),
    );
    theme.restyle(
        WidgetKind::ScrollThumb,
        Style::new().bg(Color::WHITE.with_alpha(0.16)).radius(3),
    );
    theme.restyle(
        WidgetKind::Progress,
        Style::new()
            .bg(WELL)
            .border(Border::new(1.0, OUTLINE))
            .radius(2),
    );
    theme.restyle(WidgetKind::ProgressFill, Style::new().bg(ACCENT).radius(2));
}

pub(crate) fn era_color(version: &crate::gd::GdVersion) -> Color {
    let at_least = |text: &str| *version >= text.parse().expect("a fixed version parses");
    if at_least("2.2") {
        Color::hex(0x6D5FE0)
    } else if at_least("2.1") {
        Color::hex(0x2F7FC8)
    } else if at_least("2.0") {
        Color::hex(0x2E9468)
    } else {
        Color::hex(0xC27A2C)
    }
}
