use super::{Action, AppState};
use egui::{Color32, RichText};

pub fn render(
    ui: &mut egui::Ui,
    state: &mut AppState,
    action: &mut Option<Action>,
    _disable: bool,
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Preferences").strong());
            });
            ui.separator();
            ui.label("Auto-detect interval: 3s");
            ui.label(format!(
                "Backup location: {}",
                crate::safety::backup::backup_dir().display()
            ));
            if ui.button("Save Settings").clicked() {
                *action = Some(Action::SaveSettings);
            }
        });

        ui.add_space(10.0);
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Compliance").strong());
            });
            ui.separator();
            ui.checkbox(
                &mut state.compliance_mode,
                "Disable exploit features (compliance mode)",
            );
            ui.label(
                RichText::new(
                    "When enabled, the exploit/bypass database on the Protocols tab is hidden.",
                )
                .small()
                .color(Color32::GRAY),
            );
        });

        ui.add_space(10.0);
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Wi-Fi ADB").strong());
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("IP:");
                ui.add(egui::TextEdit::singleline(&mut state.wifi_ip).desired_width(140.0));
                ui.label("Port:");
                ui.add_sized(
                    [60.0, 20.0],
                    egui::TextEdit::singleline(&mut state.wifi_port),
                );
            });
            ui.horizontal(|ui| {
                if ui.button("Connect").clicked() {
                    *action = Some(Action::WifiConnect);
                }
                if ui.button("Disconnect").clicked() {
                    *action = Some(Action::WifiDisconnect);
                }
            });
            ui.label(format!("Status: {}", state.wifi_status));
        });

        ui.add_space(10.0);
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Docker").strong());
            });
            ui.separator();
            if ui.button("Generate Dockerfile").clicked() {
                let content = crate::network::wifi_adb::generate_dockerfile();
                match std::fs::write("Dockerfile", content) {
                    Ok(()) => state.push_log(
                        "[*] Wrote Dockerfile to current directory.".into(),
                        Color32::LIGHT_BLUE,
                    ),
                    Err(e) => {
                        state.push_log(format!("[ERROR] Write Dockerfile: {}", e), Color32::RED)
                    }
                }
            }
            if ui.button("Generate docker-compose.yml").clicked() {
                let content = crate::network::docker::generate_compose();
                match std::fs::write("docker-compose.yml", content) {
                    Ok(()) => state.push_log(
                        "[*] Wrote docker-compose.yml to current directory.".into(),
                        Color32::LIGHT_BLUE,
                    ),
                    Err(e) => state.push_log(
                        format!("[ERROR] Write docker-compose.yml: {}", e),
                        Color32::RED,
                    ),
                }
            }
        });

        ui.add_space(10.0);
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Audit").strong());
            });
            ui.separator();
            ui.collapsing("Action Log Entries", |ui| {
                let entries = crate::safety::audit::read_action_log();
                let verified = crate::safety::audit::verify_chain(&entries);
                ui.label(
                    RichText::new(if verified {
                        "Hash chain: verified"
                    } else {
                        "Hash chain: TAMPERED"
                    })
                    .color(if verified {
                        Color32::from_rgb(40, 140, 40)
                    } else {
                        Color32::RED
                    }),
                );
                for e in &entries {
                    ui.label(format!("[{}] {} on {}", e.timestamp, e.action, e.serial));
                }
                if entries.is_empty() {
                    ui.label("No actions logged yet.");
                }
            });
        });

        ui.add_space(10.0);
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("About").strong());
            });
            ui.separator();
            ui.label("AndroKit");
            ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
            ui.label("License: GPLv3");
        });
    });
}
