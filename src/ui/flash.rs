use super::{Action, AppState};
use crate::config::load_profile;
use egui::{Color32, RichText};

pub fn render(
    ui: &mut egui::Ui,
    state: &mut AppState,
    action: &mut Option<Action>,
    disable_actions: bool,
) {
    let is_fastboot = state.device_mode == "fastboot";
    let is_adb = state.device_mode == "adb"
        || state.device_mode == "recovery"
        || state.device_mode == "sideload";
    let is_sideload = state.device_mode == "sideload";

    egui::SidePanel::left("flash_profiles")
        .resizable(true)
        .default_width(190.0)
        .width_range(150.0..=320.0)
        .show_inside(ui, |ui| {
            render_profiles(ui, state, action);
        });

    egui::CentralPanel::default().show_inside(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            render_fastboot_ops(ui, state, is_fastboot, disable_actions, action);
            ui.add_space(10.0);
            render_adb_ops(ui, is_adb, is_sideload, disable_actions, action);
            ui.add_space(10.0);
            render_batch_panel(ui, state, is_fastboot || is_adb, disable_actions);
        });
    });
}

fn render_profiles(ui: &mut egui::Ui, state: &mut AppState, action: &mut Option<Action>) {
    ui.vertical_centered(|ui| {
        ui.label(RichText::new("Flash Profiles").strong());
    });
    ui.separator();

    let profiles_dir = dirs_profiles();
    let mut loaded_msg: Option<(String, Color32)> = None;
    if let Ok(entries) = std::fs::read_dir(&profiles_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "yaml") {
                let name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if ui.button(&name).clicked() {
                    match load_profile(&path.to_string_lossy()) {
                        Ok(p) => {
                            loaded_msg = Some((
                                format!(
                                    "[*] Loaded profile '{}' ({} operation(s)).",
                                    p.name,
                                    p.operations.len()
                                ),
                                Color32::LIGHT_BLUE,
                            ));
                        }
                        Err(e) => {
                            loaded_msg =
                                Some((format!("[ERROR] Load profile: {}", e), Color32::RED));
                        }
                    }
                }
            }
        }
    }
    if let Some((msg, color)) = loaded_msg {
        state.push_log(msg, color);
    }

    ui.add_space(8.0);
    if ui.button("Export Profile").clicked() {
        *action = Some(Action::ExportProfile);
    }
    if ui.button("Import Profile").clicked() {
        *action = Some(Action::ImportProfile);
    }
    ui.add_space(6.0);
    ui.label(RichText::new(format!("Dir: {}", profiles_dir.display())).small());
}

fn dirs_profiles() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let dir = std::path::PathBuf::from(home)
        .join(".androidtool")
        .join("profiles");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn render_fastboot_ops(
    ui: &mut egui::Ui,
    state: &mut AppState,
    is_fastboot: bool,
    disable: bool,
    action: &mut Option<Action>,
) {
    ui.group(|ui| {
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("Fastboot Operations").strong());
        });
        ui.separator();
        ui.add_enabled_ui(is_fastboot && !disable, |ui| {
            if ui.button("Unlock Bootloader").clicked() {
                *action = Some(Action::Confirm(
                    crate::state::PendingAction::UnlockBootloader,
                ));
            }
            if ui.button("Lock Bootloader").clicked() {
                *action = Some(Action::Confirm(crate::state::PendingAction::LockBootloader));
            }
            if ui.button("Flash Recovery Image").clicked() {
                *action = Some(Action::FlashRecovery);
            }
            ui.horizontal(|ui| {
                if ui.button("Flash Boot Image").clicked() {
                    *action = Some(Action::FlashBoot);
                }
                ui.checkbox(&mut state.magisk_patch, "Patch with Magisk");
            });
        });
    });
}

fn render_adb_ops(
    ui: &mut egui::Ui,
    is_adb: bool,
    is_sideload: bool,
    disable: bool,
    action: &mut Option<Action>,
) {
    ui.group(|ui| {
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("ADB Operations").strong());
        });
        ui.separator();
        ui.add_enabled_ui(is_adb && !disable, |ui| {
            if ui.button("Reboot System").clicked() {
                *action = Some(Action::RebootSystem);
            }
            if ui.button("Reboot Recovery").clicked() {
                *action = Some(Action::RebootRecovery);
            }
            if ui.button("Reboot Bootloader").clicked() {
                *action = Some(Action::RebootBootloader);
            }
        });
        ui.add_enabled_ui(is_sideload && !disable, |ui| {
            if ui.button("ADB Sideload ZIP").clicked() {
                *action = Some(Action::Sideload);
            }
        });
    });
}

fn render_batch_panel(ui: &mut egui::Ui, state: &AppState, has_device: bool, disable: bool) {
    ui.collapsing("Batch Operations", |ui| {
        ui.add_enabled_ui(has_device && !disable, |ui| {
            ui.label("Connected device:");
            if state.device_serial == "N/A" {
                ui.label("  (none)");
            } else {
                ui.label(format!("  {} ({})", state.device_serial, state.device_mode));
            }
        });
    });
}
