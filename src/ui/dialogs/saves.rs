use weft::prelude::{Fixed, Grow, Justify, Ui};

use crate::app::{Action, ago};
use crate::instances::InstanceName;
use crate::ui::form::DIALOG_WIDTH;
use crate::ui::window::Window;

pub(super) fn show_backups(ui: &mut Ui, window: &mut Window, open: &mut bool, name: &InstanceName) {
    let backups = window.app.save_backups(name);
    let game_open = window.app.playing().is_some();
    ui.modal(open)
        .title(format!("Save backups of \"{name}\""))
        .style(|style| style.width(Fixed(DIALOG_WIDTH)))
        .show(|ui| {
            if backups.is_empty() {
                ui.label("No backups. Glint makes one each time the game opens.")
                    .muted();
            }
            for backup in &backups {
                ui.push_id(&backup.folder, |ui| {
                    ui.row().width(Grow(1)).gap(8).show(|ui| {
                        ui.label(format!("{} - GD {}", ago(backup.taken), backup.version))
                            .style(|style| style.width(Grow(1)));
                        let restore = ui
                            .button("Restore")
                            .height(24)
                            .enabled(!game_open)
                            .tooltip(if game_open {
                                "Close Geometry Dash first"
                            } else {
                                "Restores this save. Glint backs up the current one first"
                            })
                            .clicked();
                        if restore {
                            window.actions.push(Action::RestoreBackup {
                                instance: name.clone(),
                                backup: backup.folder.clone(),
                            });
                            ui.close_overlay();
                        }
                    });
                });
            }
            ui.row().justify(Justify::End).gap(8).show(|ui| {
                if ui.button("Done").primary().clicked() {
                    ui.close_overlay();
                }
            });
        });
}
