use super::{Action, AppState};
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

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("ADB File Transfer").strong());
            });
            ui.separator();
            ui.add_enabled_ui(has_adb && !disable_actions, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Push to:");
                    ui.add(
                        egui::TextEdit::singleline(&mut state.push_remote_path)
                            .hint_text("/sdcard/")
                            .desired_width(240.0),
                    );
                    if ui.button("Push File...").clicked() {
                        *action = Some(Action::PushFile);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Pull from:");
                    ui.add(
                        egui::TextEdit::singleline(&mut state.pull_remote_path)
                            .hint_text("/sdcard/file")
                            .desired_width(240.0),
                    );
                    if ui.button("Pull to Folder...").clicked() {
                        *action = Some(Action::PullFile);
                    }
                });
            });
            if !has_adb {
                ui.label(
                    RichText::new("Connect a device in ADB mode to transfer files.")
                        .small()
                        .color(Color32::GRAY),
                );
            }
        });

        ui.add_space(10.0);
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("MTP Browser").strong());
            });
            ui.separator();
            ui.label(
                RichText::new(
                    "MTP browsing is not implemented. Use ADB file transfer above, or your \
                     desktop file manager's MTP support.",
                )
                .small()
                .color(Color32::GRAY),
            );
        });
    });
}
