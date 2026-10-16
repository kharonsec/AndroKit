use super::{Action, AppState};
use crate::state::PendingAction;
use egui::{Color32, RichText};

pub fn render(
    ui: &mut egui::Ui,
    state: &mut AppState,
    action: &mut Option<Action>,
    disable_actions: bool,
) {
    let has_adb = state.device_mode == "adb"
        || state.device_mode == "recovery"
        || state.device_mode == "sideload";
    let has_fastboot = state.device_mode == "fastboot";

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Backup").strong());
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Partitions:");
                ui.add(
                    egui::TextEdit::singleline(&mut state.backup_partitions)
                        .hint_text("boot,recovery,dtbo")
                        .desired_width(260.0),
                );
            });
            ui.checkbox(&mut state.encrypt_backup, "Encrypt backup with AES-256-GCM");
            if state.encrypt_backup {
                ui.horizontal(|ui| {
                    ui.label("Passphrase:");
                    ui.add(
                        egui::TextEdit::singleline(&mut state.backup_passphrase)
                            .password(true)
                            .desired_width(200.0),
                    );
                });
            }
            ui.add_space(4.0);
            ui.add_enabled_ui(has_adb && !disable_actions, |ui| {
                if ui.button("Backup Partitions").clicked() {
                    *action = Some(Action::BackupPartitions);
                }
                if ui.button("Full NAND Backup").clicked() {
                    *action = Some(Action::FullNandBackup);
                }
            });
            if !has_adb {
                ui.label(
                    RichText::new("Connect a device in ADB/recovery mode.")
                        .small()
                        .color(Color32::GRAY),
                );
            }
        });

        ui.add_space(10.0);
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Restore").strong());
            });
            ui.separator();
            ui.label(
                RichText::new("Restores .img / .img.enc files from a backup folder via fastboot.")
                    .small(),
            );
            ui.add_enabled_ui(has_fastboot && !disable_actions, |ui| {
                if ui.button("Restore from Backup Folder").clicked() {
                    *action = Some(Action::RestoreBackup);
                }
            });
            if !has_fastboot {
                ui.label(
                    RichText::new("Restore requires fastboot mode.")
                        .small()
                        .color(Color32::GRAY),
                );
            }
        });

        ui.add_space(10.0);
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Forensic").strong());
            });
            ui.separator();
            ui.label(
                RichText::new(
                    "Read-only pull of boot/recovery/dtbo/vbmeta with a SHA-256 manifest.",
                )
                .small(),
            );
            ui.add_enabled_ui(has_adb && !disable_actions, |ui| {
                if ui.button("Forensic Dump (Read-Only)").clicked() {
                    *action = Some(Action::ForensicDump);
                }
            });
        });

        ui.add_space(10.0);
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("Device Wipe")
                        .color(Color32::from_rgb(200, 50, 50))
                        .strong(),
                );
            });
            ui.separator();
            ui.add_enabled_ui((has_fastboot || has_adb) && !disable_actions, |ui| {
                if ui.button("Quick Wipe (userdata)").clicked() {
                    *action = Some(Action::Confirm(PendingAction::QuickWipe));
                }
                if ui.button("Secure Wipe (overwrite userdata)").clicked() {
                    *action = Some(Action::Confirm(PendingAction::SecureWipe));
                }
            });
        });
    });
}
