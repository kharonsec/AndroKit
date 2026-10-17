use super::{Action, AppState};
use egui::{Color32, RichText};

pub fn render(ui: &mut egui::Ui, state: &mut AppState, action: &mut Option<Action>, disable: bool) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.label(RichText::new("Active Protocol").strong());
        ui.label(format!("Mode: {}", state.device_mode.to_uppercase()));

        ui.add_space(10.0);
        ui.collapsing("Custom Command", |ui| {
            ui.label(
                RichText::new("Runs an adb or fastboot command; output goes to the console.")
                    .small(),
            );
            ui.horizontal(|ui| {
                let resp = ui.add_sized(
                    [300.0, 20.0],
                    egui::TextEdit::singleline(&mut state.custom_command)
                        .hint_text("adb shell getprop ro.build.fingerprint"),
                );
                let submit = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let clicked = ui.add_enabled(!disable, egui::Button::new("Run")).clicked();
                if clicked || submit {
                    *action = Some(Action::RunCustomCommand);
                }
            });
        });

        if !state.compliance_mode {
            ui.add_space(10.0);
            render_exploit_db(ui);
        } else {
            ui.add_space(10.0);
            ui.label(
                RichText::new("Exploit database hidden (compliance mode is enabled in Settings).")
                    .color(Color32::GRAY),
            );
        }
    });
}

fn render_exploit_db(ui: &mut egui::Ui) {
    ui.collapsing("Known Exploits & Bypasses", |ui| {
        let content = std::fs::read_to_string("exploits.yaml").unwrap_or_default();
        if let Ok(configs) = serde_yaml::from_str::<Vec<serde_yaml::Value>>(&content) {
            for entry in configs {
                let device = entry
                    .get("device_pattern")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let etype = entry.get("type").and_then(|v| v.as_str()).unwrap_or("?");
                let risk = entry.get("risk").and_then(|v| v.as_str()).unwrap_or("low");
                let desc = entry
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let risk_color = match risk {
                    "critical" => Color32::RED,
                    "high" => Color32::from_rgb(200, 100, 0),
                    "medium" => Color32::from_rgb(200, 150, 0),
                    _ => Color32::GRAY,
                };

                ui.collapsing(
                    RichText::new(format!("[{}] {} - {}", device, etype, risk)).color(risk_color),
                    |ui| {
                        ui.label(desc);
                        ui.add_space(4.0);
                        let mut agreed = false;
                        ui.checkbox(&mut agreed, "I understand this may void warranty");
                        if ui
                            .add_enabled(agreed, egui::Button::new("Activate"))
                            .clicked()
                        {}
                    },
                );
            }
        } else {
            ui.label("No exploit profiles loaded.");
        }
        ui.label("  (Load exploits.yaml with device profiles)");
    });
}
