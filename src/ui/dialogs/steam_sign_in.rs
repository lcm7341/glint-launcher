use weft::core::Rect;
use weft::prelude::{Align, Color, Fixed, FontWeight, Grow, Justify, Ui, vec2};

use crate::app::{Action, App, Credentials, SteamPrompt};
use crate::depot::CodeKind;
use crate::instances::InstanceName;
use crate::ui::form::{DIALOG_WIDTH, field_label, show_form};
use crate::ui::theme::TEXT;
use crate::ui::window::Window;

pub(crate) struct SignInForm {
    instance: InstanceName,
    username: String,
    password: String,
}

impl SignInForm {
    pub(crate) fn new(instance: InstanceName) -> Self {
        Self {
            instance,
            username: String::new(),
            password: String::new(),
        }
    }
}

pub(crate) fn show_sign_in(
    ui: &mut Ui,
    window: &mut Window,
    open: &mut bool,
    form: &mut SignInForm,
) {
    ui.modal(open)
        .title("Sign in to Steam")
        .style(|style| style.width(Fixed(DIALOG_WIDTH)))
        .show(|ui| {
            let mut submitted = false;
            show_form(ui, |ui| {
                field_label(ui, "Username");
                submitted |= ui
                    .text_input(&mut form.username)
                    .name("Steam username")
                    .width(Grow(1))
                    .submitted();
                field_label(ui, "Password");
                submitted |= ui
                    .text_input(&mut form.password)
                    .password()
                    .name("Steam password")
                    .width(Grow(1))
                    .submitted();
            });
            ui.label("This is only sent to and saved through DepotDownloader.")
                .muted()
                .font_size(12);
            let username = form.username.trim();
            let ready = !username.is_empty() && !form.password.is_empty();
            ui.row().width(Grow(1)).gap(8).show(|ui| {
                if ui.button("Use a QR code").clicked() {
                    window.actions.push(Action::Download(form.instance.clone()));
                    ui.close_overlay();
                }
                ui.spacer();
                if ui.button("Cancel").clicked() {
                    ui.close_overlay();
                }
                let clicked = ui.button("Sign in").primary().enabled(ready).clicked();
                if ready && (clicked || submitted) {
                    window.actions.push(Action::SignInAndDownload {
                        instance: form.instance.clone(),
                        sign_in: Credentials {
                            username: username.to_owned(),
                            password: std::mem::take(&mut form.password),
                        },
                    });
                    ui.close_overlay();
                }
            });
        });
}

/// steam asks mid-download: an expired session, a steam guard code, or app approval
pub(crate) fn prompt_dialog(
    ui: &mut Ui,
    launcher: &App,
    answer: &mut String,
    actions: &mut Vec<Action>,
) {
    let approving = launcher
        .download()
        .is_some_and(|download| download.approving());
    let prompt = launcher.download().and_then(|download| download.prompt());
    if prompt.is_none() && !approving {
        answer.clear();
        return;
    }
    let mut is_open = true;
    ui.modal(&mut is_open)
        .title("Sign in to Steam")
        .dismissible(false)
        .style(|style| style.width(Fixed(DIALOG_WIDTH)))
        .show(|ui| {
            let Some(prompt) = &prompt else {
                ui.label("Approve the sign-in in the Steam app.");
                ui.row().width(Grow(1)).justify(Justify::End).show(|ui| {
                    if ui.button("Cancel").clicked() {
                        actions.push(Action::CancelDownload);
                    }
                });
                return;
            };
            let is_password = matches!(prompt, SteamPrompt::Password { .. });
            ui.label(prompt_text(prompt));
            if let SteamPrompt::Code { wrong: true, .. } = prompt {
                ui.label("That code didn't work, try the newest one.")
                    .muted()
                    .font_size(12);
            }
            let input = ui.text_input(answer).width(Grow(1));
            let submitted = if is_password {
                input.password().name("Steam password").submitted()
            } else {
                input.name("Steam Guard code").submitted()
            };
            let ready = !answer.trim().is_empty();
            ui.row().width(Grow(1)).gap(8).show(|ui| {
                if is_password && ui.button("Use a QR code").clicked() {
                    actions.push(Action::SignInWithQr);
                }
                ui.spacer();
                if ui.button("Cancel").clicked() {
                    actions.push(Action::CancelDownload);
                }
                let clicked = ui.button("Continue").primary().enabled(ready).clicked();
                if ready && (clicked || submitted) {
                    let text = std::mem::take(answer);
                    let text = if is_password {
                        text
                    } else {
                        text.trim().to_owned()
                    };
                    actions.push(Action::AnswerSteam(text));
                }
            });
        });
}

fn prompt_text(prompt: &SteamPrompt<'_>) -> String {
    match prompt {
        SteamPrompt::Password {
            account: Some(account),
        } => {
            format!("Steam signed you out. Enter the password for {account}.")
        }
        SteamPrompt::Password { account: None } => "Enter your Steam password.".to_owned(),
        SteamPrompt::Code {
            kind: CodeKind::App,
            ..
        } => "Enter the Steam Guard code from the Steam app.".to_owned(),
        SteamPrompt::Code {
            kind: CodeKind::Email(Some(email)),
            ..
        } => {
            format!("Enter the Steam Guard code Steam emailed to {email}.")
        }
        SteamPrompt::Code {
            kind: CodeKind::Email(None),
            ..
        } => "Enter the Steam Guard code Steam emailed you.".to_owned(),
    }
}

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
