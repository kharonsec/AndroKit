use super::{Action, AppState};
use egui::{Color32, RichText};

pub fn render(
    ui: &mut egui::Ui,
    state: &mut AppState,
    action: &mut Option<Action>,
    disable_actions: bool,
) {
    ui.add_space(10.0);
    ui.group(|ui| {
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("Device Info").strong());
        });
        ui.separator();
        ui.label(RichText::new(format!("Manufacturer: {}", state.device_manufacturer)).strong());
        ui.label(RichText::new(format!("Model: {}", state.device_model)).strong());
        ui.label(RichText::new(format!("Serial: {}", state.device_serial)).strong());
        ui.label(format!("Mode: {}", state.device_mode.to_uppercase()));
    });

    ui.add_space(10.0);
    ui.group(|ui| {
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("Quick Actions").strong());
        });
        ui.separator();
        ui.add_enabled_ui(!disable_actions, |ui| {
            if ui.button("Detect Device Details").clicked() {
                *action = Some(Action::DetectDetails);
            }
        });
    });

    if state.is_amd {
        ui.add_space(10.0);
        ui.group(|ui| {
            ui.label(RichText::new("System (AMD Detected)").color(Color32::from_rgb(200, 50, 50)));
            if state.has_quirk {
                ui.label(RichText::new("USB Quirk Applied").color(Color32::from_rgb(50, 150, 50)));
            } else {
                ui.label(RichText::new("USB Quirk Missing").strong());
                if ui.button("Apply Fix (Temporary)").clicked() {
                    *action = Some(Action::ApplyQuirk);
                }
            }
        });
    }

    if state.device_manufacturer != "Unknown" {
        ui.add_space(10.0);
        let mfg = state.device_manufacturer.to_lowercase();
        let content = std::fs::read_to_string("warranty_warnings.yaml").unwrap_or_default();
        if let Ok(configs) =
            serde_yaml::from_str::<std::collections::HashMap<String, serde_yaml::Value>>(&content)
        {
            for (key, val) in &configs {
                if mfg.contains(&key.to_lowercase()) {
                    if let Some(warning) = val.get("warning").and_then(|v| v.as_str()) {
                        ui.group(|ui| {
                            ui.label(
                                RichText::new("Warranty Warning")
                                    .color(Color32::from_rgb(200, 100, 0))
                                    .strong(),
                            );
                            ui.label(warning);
                        });
                    }
                    break;
                }
            }
        }
    }
}
