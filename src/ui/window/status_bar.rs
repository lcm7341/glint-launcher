use weft::prelude::{Align, Fixed, Grow, Insets, Ui};

use super::Window;
use crate::app::Action;

pub(super) fn show(ui: &mut Ui, window: &mut Window) {
    ui.row()
        .width(Grow(1))
        .gap(8)
        .align(Align::Center)
        .padding(Insets::symmetric(3.0, 10.0))
        .style(|style| style.min_height(30.0))
        .show(|ui| {
            if let Some(download) = window.app.download() {
                ui.row().gap(8).align(Align::Center).show(|ui| {
                    ui.label(format!("Downloading GD {}", download.version()))
                        .wrap(false)
                        .font_size(12);
                    ui.progress(download.fraction())
                        .style(|style| style.width(Fixed(160.0)).height(Fixed(6.0)));
                    ui.label(download.detail())
                        .muted()
                        .font_size(12)
                        .wrap(false)
                        .style(|style| style.width(Fixed(220.0)).clip(true));
                    if ui.button("Cancel download").height(22).clicked() {
                        window.actions.push(Action::CancelDownload);
                    }
                });
            } else {
                ui.label(summary(window)).muted().font_size(12).wrap(false);
            }
            ui.spacer();
            match window.app.steam_account() {
                Some(account) => {
                    ui.label(format!("Steam: {account}"))
                        .muted()
                        .font_size(12)
                        .wrap(false);
                    if ui
                        .button("Sign out")
                        .muted()
                        .height(22)
                        .tooltip("Sign out")
                        .clicked()
                    {
                        window.actions.push(Action::SignOut);
                    }
                }
                None => {
                    ui.label("Steam: not signed in")
                        .muted()
                        .font_size(12)
                        .wrap(false)
                        .tooltip("You'll sign in on your first download");
                }
            }
        });
}

fn summary(window: &Window) -> String {
    let mut summary = match window.instances().count() {
        1 => "1 instance".to_owned(),
        count => format!("{count} instances"),
    };
    if let Some(playing) = window.app.playing() {
        summary.push_str(&format!(" · Playing {playing}"));
    }
    summary
}
