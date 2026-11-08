use weft::prelude::{Align, FontWeight, Grow, Insets, Ui};
use weft::text::FontFamily;
use weft::ui::Border;

use crate::glint_theme::{HAIRLINE, MUTED, TEXT, WELL};

#[derive(Clone, PartialEq, Eq)]
enum Block {
    Heading(u8, String),
    Paragraph(String),
    Item(String, String),
    Quote(String),
    Code(String),
    Rule,
}

pub(crate) fn show_markdown(ui: &mut Ui, markdown: &str) {
    ui.column().gap(8).width(Grow(1)).show(|ui| {
        for (index, block) in blocks(markdown).into_iter().enumerate() {
            ui.push_id(index, |ui| show_block(ui, block));
        }
    });
}

fn show_block(ui: &mut Ui, block: Block) {
    match block {
        Block::Heading(level, text) => {
            let size = match level {
                1 => 17,
                2 => 15,
                _ => 14,
            };
            ui.label(text)
                .font_size(size)
                .style(|style| style.weight(FontWeight::SEMI_BOLD).color(TEXT));
        }
        Block::Paragraph(text) => {
            ui.label(text).style(|style| style.color(TEXT));
        }
        Block::Item(marker, text) => {
            ui.row()
                .gap(8)
                .width(Grow(1))
                .align(Align::Start)
                .show(|ui| {
                    ui.label(marker).style(|style| style.color(MUTED));
                    ui.label(text)
                        .style(|style| style.color(TEXT).width(Grow(1)));
                });
        }
        Block::Quote(text) => {
            ui.label(text).style(|style| {
                style
                    .color(MUTED)
                    .padding(Insets::new(0.0, 0.0, 0.0, 10.0))
                    .border(Border::new(Insets::new(0.0, 0.0, 0.0, 2.0), HAIRLINE))
            });
        }
        Block::Code(code) => {
            ui.column()
                .width(Grow(1))
                .padding(Insets::all(10.0))
                .style(|style| style.bg(WELL).radius(4).clip(true))
                .show(|ui| {
                    ui.label(code)
                        .font_size(12)
                        .wrap(false)
                        .style(|style| style.font_family(FontFamily::Monospace).color(TEXT));
                });
        }
        Block::Rule => {
            ui.separator();
        }
    }
}

fn blocks(markdown: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut lines = markdown.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("```") {
            let mut code = Vec::new();
            for line in lines.by_ref() {
                if line.trim().starts_with("```") {
                    break;
                }
                code.push(line);
            }
            blocks.push(Block::Code(code.join("\n")));
            continue;
        }
        if trimmed
            .chars()
            .all(|letter| matches!(letter, '-' | '*' | '_'))
            && trimmed.len() >= 3
        {
            blocks.push(Block::Rule);
            continue;
        }
        if let Some((level, text)) = heading(trimmed) {
            blocks.push(Block::Heading(level, inline(text)));
            continue;
        }
        if let Some(text) = trimmed.strip_prefix('>') {
            blocks.push(Block::Quote(inline(text.trim())));
            continue;
        }
        if let Some((marker, text)) = list_item(trimmed) {
            let mut text = text.to_owned();
            while let Some(next) = lines.peek() {
                let next_trimmed = next.trim();
                if next_trimmed.is_empty()
                    || !next.starts_with(' ')
                    || list_item(next_trimmed).is_some()
                {
                    break;
                }
                text.push(' ');
                text.push_str(next_trimmed);
                lines.next();
            }
            blocks.push(Block::Item(marker, inline(&text)));
            continue;
        }
        let mut text = trimmed.to_owned();
        while let Some(next) = lines.peek() {
            let next = next.trim();
            if next.is_empty()
                || next.starts_with("```")
                || next.starts_with('#')
                || next.starts_with('>')
                || list_item(next).is_some()
            {
                break;
            }
            text.push(' ');
            text.push_str(next);
            lines.next();
        }
        let text = inline(&text);
        if !text.trim().is_empty() {
            blocks.push(Block::Paragraph(text));
        }
    }
    blocks
}

fn heading(line: &str) -> Option<(u8, &str)> {
    let level = line.chars().take_while(|&letter| letter == '#').count();
    let text = line[level..].strip_prefix(' ')?;
    (1..=6).contains(&level).then_some((level as u8, text))
}

fn list_item(line: &str) -> Option<(String, &str)> {
    for bullet in ["- ", "* ", "+ "] {
        if let Some(text) = line.strip_prefix(bullet) {
            return Some(("•".to_owned(), text));
        }
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    let text = line[digits..].strip_prefix(". ")?;
    (digits > 0).then(|| (format!("{}.", &line[..digits]), text))
}

fn inline(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(letter) = rest.chars().next() {
        if let Some(after) = rest.strip_prefix("![")
            && let Some(end) = link_end(after)
        {
            rest = &after[end..];
            continue;
        }
        if let Some(after) = rest.strip_prefix('[')
            && let Some(close) = after.find("](")
            && let Some(end) = link_end(after)
        {
            out.push_str(&after[..close]);
            rest = &after[end..];
            continue;
        }
        if letter == '<'
            && let Some(close) = rest.find('>')
            && rest[1..close]
                .chars()
                .all(|letter| letter.is_ascii_alphanumeric() || matches!(letter, '/' | '-'))
        {
            rest = &rest[close + 1..];
            continue;
        }
        if matches!(letter, '*' | '`') || (letter == '_' && rest.starts_with("__")) {
            rest = rest.trim_start_matches(letter);
            continue;
        }
        out.push(letter);
        rest = &rest[letter.len_utf8()..];
    }
    out
}

fn link_end(after_bracket: &str) -> Option<usize> {
    let close = after_bracket.find("](")?;
    let target = &after_bracket[close + 2..];
    Some(close + 2 + target.find(')')? + 1)
}
