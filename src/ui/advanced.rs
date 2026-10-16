use super::{Action, AppState};
use egui::{Color32, RichText};

pub fn render(
    ui: &mut egui::Ui,
    state: &mut AppState,
    action: &mut Option<Action>,
    _disable: bool,
) {
    let has_adb = state.device_mode == "adb" || state.device_mode == "recovery";

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(6.0);
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Live logcat").strong());
            });
            ui.separator();
            ui.add_enabled_ui(has_adb, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Start logcat").clicked() {
                        *action = Some(Action::StartLogcat);
                    }
                    if ui.button("Stop logcat").clicked() {
                        *action = Some(Action::StopLogcat);
                    }
                });
            });
            ui.label(RichText::new("Output appears in the console below.").small());
        });

        ui.add_space(10.0);
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("dmesg Viewer").strong());
            });
            ui.separator();
            ui.add_enabled_ui(has_adb, |ui| {
                if ui.button("Capture dmesg").clicked() {
                    *action = Some(Action::CaptureDmesg);
                }
            });
        });

        ui.add_space(10.0);
        ui.collapsing("Boot Image Analyzer", |ui| {
            ui.horizontal(|ui| {
                if ui.button("Pick boot.img").clicked() {
                    *action = Some(Action::PickBootImg);
                }
                if ui.button("Analyze").clicked() {
                    *action = Some(Action::AnalyzeBoot);
                }
                if ui.button("Patch with Magisk").clicked() {
                    *action = Some(Action::PatchBootMagisk);
                }
            });
            if let Some(path) = &state.boot_img_path {
                ui.label(RichText::new(format!("Selected: {}", path.display())).small());
            }
            if let Some(info) = &state.boot_info {
                ui.add_space(4.0);
                ui.label(RichText::new(info).monospace());
            }
        });

        ui.add_space(10.0);
        ui.collapsing("DTBO Info Viewer", |ui| {
            ui.horizontal(|ui| {
                if ui.button("Pick .dtbo / .img").clicked() {
                    *action = Some(Action::PickDtbo);
                }
                if ui.button("View Info").clicked() {
                    *action = Some(Action::ViewDtbo);
                }
            });
            if let Some(path) = &state.dtbo_path {
                ui.label(RichText::new(format!("Selected: {}", path.display())).small());
            }
            if let Some(info) = &state.dtbo_info {
                ui.add_space(4.0);
                ui.label(RichText::new(info).monospace());
            }
        });

        ui.add_space(10.0);
        ui.collapsing("Partition Hex Viewer (read-only)", |ui| {
            ui.horizontal(|ui| {
                ui.label("Path:");
                ui.add_sized(
                    [260.0, 20.0],
                    egui::TextEdit::singleline(&mut state.hex_path),
                );
                ui.add_enabled_ui(has_adb, |ui| {
                    if ui.button("Read 256B").clicked() {
                        *action = Some(Action::ReadPartitionHex);
                    }
                });
            });
            ui.label(
                RichText::new("Requires a rooted device (uses su + dd + xxd).")
                    .small()
                    .color(Color32::GRAY),
            );
            if let Some(result) = &state.hex_result {
                ui.add_space(4.0);
                ui.label(RichText::new(result).monospace().size(11.0));
            }
        });
    });
}
