pub mod advanced;
pub mod backup;
pub mod dashboard;
pub mod files;
pub mod flash;
pub mod protocols;
pub mod settings;

use super::state::AppState;
use super::Action;

pub struct Tab {
    pub name: &'static str,
    pub id: &'static str,
}

pub const TABS: &[Tab] = &[
    Tab {
        name: "Dashboard",
        id: "dashboard",
    },
    Tab {
        name: "Flash",
        id: "flash",
    },
    Tab {
        name: "Backup & Restore",
        id: "backup",
    },
    Tab {
        name: "Protocols",
        id: "protocols",
    },
    Tab {
        name: "Files",
        id: "files",
    },
    Tab {
        name: "Advanced",
        id: "advanced",
    },
    Tab {
        name: "Settings",
        id: "settings",
    },
];

pub fn render_tab_content(
    ui: &mut egui::Ui,
    tab_index: usize,
    state: &mut AppState,
    action: &mut Option<Action>,
    disable_actions: bool,
) {
    match tab_index {
        0 => dashboard::render(ui, state, action, disable_actions),
        1 => flash::render(ui, state, action, disable_actions),
        2 => backup::render(ui, state, action, disable_actions),
        3 => protocols::render(ui, state, action, disable_actions),
        4 => files::render(ui, state, action, disable_actions),
        5 => advanced::render(ui, state, action, disable_actions),
        6 => settings::render(ui, state, action, disable_actions),
        _ => {}
    }
}

/// Render the bottom log console: filter box, autoscroll/clear controls, and
/// the scrollable, color-coded output of commands and streams.
pub fn render_log_console(ui: &mut egui::Ui, state: &mut AppState, action: &mut Option<Action>) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Console").strong());
        ui.separator();
        ui.label("Filter:");
        ui.add(
            egui::TextEdit::singleline(&mut state.log_filter)
                .hint_text("substring")
                .desired_width(160.0),
        );
        ui.checkbox(&mut state.log_autoscroll, "Autoscroll");
        if ui.button("Clear").clicked() {
            *action = Some(Action::ClearLogs);
        }
        if ui.button("Copy").clicked() {
            let joined = state
                .logs
                .iter()
                .map(|(t, _)| t.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            ui.ctx().copy_text(joined);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(format!("{} lines", state.logs.len())).small());
        });
    });
    ui.separator();

    let filter = state.log_filter.to_lowercase();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(state.log_autoscroll)
        .show(ui, |ui| {
            ui.style_mut().wrap = Some(false);
            for (line, color) in &state.logs {
                if filter.is_empty() || line.to_lowercase().contains(&filter) {
                    ui.label(egui::RichText::new(line).monospace().color(*color));
                }
            }
        });
}
