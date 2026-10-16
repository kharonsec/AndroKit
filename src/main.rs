#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)]

mod advanced;
mod analyzer;
mod app;
mod config;
mod executor;
mod flash;
mod legal;
mod network;
mod platform;
mod protocols;
mod safety;
mod state;
mod ui;

use app::{Action, AndroKitApp};
use eframe::egui::{self, Color32, RichText};
use executor::ProcessOutput;
use rfd::FileDialog;
use state::PendingAction;

impl eframe::App for AndroKitApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::light());
        self.check_device_state();

        // Acquire the UI lock for this frame. If a background task briefly
        // holds it, skip rendering this frame and try again rather than panic.
        let mut st = match self.state.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                ctx.request_repaint();
                return;
            }
        };

        // Drain executor output into the log console (lock already held).
        while let Ok(event) = self.rx.try_recv() {
            match event {
                ProcessOutput::Stdout(line) => {
                    st.push_log(format!("[STDOUT] {}", line), Color32::from_rgb(90, 90, 90));
                    if st.current_command_id == "detect_adb_props" {
                        if line.contains("ro.product.manufacturer") {
                            if let Some(val) = line.split("]: [").nth(1) {
                                st.device_manufacturer = val.trim_end_matches(']').to_string();
                            }
                        }
                        if line.contains("ro.product.model") {
                            if let Some(val) = line.split("]: [").nth(1) {
                                st.device_model = val.trim_end_matches(']').to_string();
                            }
                        }
                    }
                }
                ProcessOutput::Stderr(line) => {
                    if st.current_command_id == "detect_fastboot_vars"
                        && line.starts_with("(bootloader) product:")
                    {
                        if let Some(val) = line.split(':').nth(1) {
                            st.device_model = format!("{} (codename)", val.trim());
                        }
                    }
                    st.push_log(format!("[STDERR] {}", line), Color32::from_rgb(200, 50, 50));
                }
                ProcessOutput::Error(e) => {
                    st.push_log(format!("[ERROR] {}", e), Color32::RED);
                    st.status_bar = "Error occurred.".into();
                }
                ProcessOutput::Finished(success, id) => {
                    st.is_command_running = false;
                    st.current_command_id.clear();
                    st.cancel_tx = None;
                    if success {
                        st.push_log(
                            "[SUCCESS] Command completed successfully.".into(),
                            Color32::from_rgb(40, 140, 40),
                        );
                        st.status_bar = format!("Command '{}' successful.", id);
                    } else {
                        st.push_log("[FAILED] Command failed.".into(), Color32::DARK_RED);
                        st.status_bar = format!("Command '{}' failed.", id);
                    }
                }
            }
        }

        let mut action = Option::<Action>::None;

        if st.is_amd
            && !st.has_quirk
            && st.device_manufacturer.to_lowercase().contains("xiaomi")
            && !st.quirk_prompt_dismissed
        {
            st.show_quirk_prompt = true;
        }

        let is_running = st.is_command_running;
        let mode = st.device_mode.clone();

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("AndroKit").size(20.0).strong());
                    ui.label(RichText::new("Connected Device Details").color(Color32::GRAY));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mode_color = match mode.as_str() {
                        "adb" => Color32::from_rgb(50, 150, 50),
                        "fastboot" => Color32::from_rgb(200, 100, 0),
                        "none" => Color32::GRAY,
                        _ => Color32::from_rgb(0, 100, 200),
                    };
                    ui.label(
                        RichText::new(mode.to_uppercase())
                            .strong()
                            .size(24.0)
                            .color(mode_color),
                    );
                    ui.label("MODE:");
                });
            });
            ui.add_space(8.0);
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&st.status_bar).small());
                if is_running {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(egui::Spinner::new().size(14.0));
                        if ui.button("Cancel").clicked() {
                            if let Some(tx) = st.cancel_tx.take() {
                                let _ = tx.try_send(());
                            }
                        }
                    });
                }
            });
        });

        egui::TopBottomPanel::top("tabs")
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (i, tab) in ui::TABS.iter().enumerate() {
                        if ui
                            .selectable_label(st.selected_tab == i, tab.name)
                            .clicked()
                        {
                            st.selected_tab = i;
                        }
                    }
                });
            });

        egui::TopBottomPanel::bottom("log_console")
            .resizable(true)
            .default_height(190.0)
            .min_height(90.0)
            .show(ctx, |ui| {
                ui::render_log_console(ui, &mut st, &mut action);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui::render_tab_content(ui, st.selected_tab, &mut st, &mut action, is_running);
        });

        if st.show_quirk_prompt {
            egui::Window::new("AMD Xiaomi Fix Required")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    ui.label(
                        "Xiaomi devices often disconnect in fastboot on AMD. Apply USB quirk fix?",
                    );
                    ui.horizontal(|ui| {
                        if ui.button("Apply Fix").clicked() {
                            action = Some(Action::ApplyQuirk);
                            st.show_quirk_prompt = false;
                        }
                        if ui.button("Dismiss").clicked() {
                            st.show_quirk_prompt = false;
                            st.quirk_prompt_dismissed = true;
                        }
                    });
                });
        }

        // Confirmation modal for destructive actions.
        if let Some(pending) = st.pending_confirm.clone() {
            egui::Window::new(pending.title())
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    ui.set_max_width(420.0);
                    ui.label(
                        RichText::new(pending.description()).color(Color32::from_rgb(180, 40, 40)),
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui
                            .button(RichText::new("Proceed").color(Color32::from_rgb(180, 40, 40)))
                            .clicked()
                        {
                            action = Some(match pending {
                                PendingAction::QuickWipe => Action::QuickWipe,
                                PendingAction::SecureWipe => Action::SecureWipe,
                                PendingAction::LockBootloader => Action::LockBootloader,
                                PendingAction::UnlockBootloader => Action::UnlockBootloader,
                            });
                            st.pending_confirm = None;
                        }
                        if ui.button("Cancel").clicked() {
                            st.pending_confirm = None;
                        }
                    });
                });
        }

        // Release the UI lock before handling actions (handlers re-lock).
        drop(st);
        self.handle_action(action);
    }
}

impl AndroKitApp {
    fn handle_action(&mut self, action: Option<Action>) {
        let Some(a) = action else {
            return;
        };
        if a == Action::None {
            return;
        }

        let (config, mode) = {
            let Ok(st) = self.state.try_lock() else {
                return;
            };
            (
                self.config
                    .get_config(&st.device_manufacturer, &st.device_model),
                st.device_mode.clone(),
            )
        };

        match a {
            Action::None => {}
            Action::ToggleTab(_) => {}
            Action::ApplyQuirk => self.apply_quirk(),
            Action::DetectDetails => self.run_detect_details(),
            Action::Confirm(pending) => {
                if let Ok(mut st) = self.state.try_lock() {
                    st.pending_confirm = Some(pending);
                }
            }
            Action::UnlockBootloader => {
                if let Some(cmd) = config.unlock_command {
                    self.run_cmd("unlock", &cmd, config.unlock_warning.as_deref());
                }
            }
            Action::LockBootloader => {
                if let Some(cmd) = config.lock_command {
                    self.run_cmd("lock", &cmd, config.lock_warning.as_deref());
                }
            }
            Action::FlashRecovery => {
                if let Some(path) = FileDialog::new().add_filter("image", &["img"]).pick_file() {
                    self.blocklist_warn(&path);
                    let partition = config
                        .recovery_partition
                        .unwrap_or_else(|| "recovery".into());
                    let args: Vec<String> = vec![
                        "flash".into(),
                        partition,
                        path.to_string_lossy().into_owned(),
                    ];
                    self.start_process("flash_recovery", "fastboot".into(), args);
                }
            }
            Action::FlashBoot => {
                if let Some(path) = FileDialog::new().add_filter("image", &["img"]).pick_file() {
                    self.blocklist_warn(&path);
                    let magisk = self
                        .state
                        .try_lock()
                        .map(|s| s.magisk_patch)
                        .unwrap_or(false);
                    let flash_path = if magisk {
                        match crate::flash::magisk::patch_boot_image(&path) {
                            Ok(p) => {
                                self.append_log(
                                    &format!("[*] Magisk-patched image: {}", p.display()),
                                    Color32::LIGHT_BLUE,
                                );
                                p
                            }
                            Err(e) => {
                                self.append_log(
                                    &format!("[ERROR] Magisk patch failed: {}", e),
                                    Color32::RED,
                                );
                                return;
                            }
                        }
                    } else {
                        path
                    };
                    let partition = config.boot_partition.unwrap_or_else(|| "boot".into());
                    let args: Vec<String> = vec![
                        "flash".into(),
                        partition,
                        flash_path.to_string_lossy().into_owned(),
                    ];
                    self.start_process("flash_boot", "fastboot".into(), args);
                }
            }
            Action::RebootSystem => {
                let cmd = if mode == "fastboot" {
                    config
                        .fastboot_reboot_command
                        .unwrap_or_else(|| "fastboot reboot".into())
                } else {
                    config
                        .adb_reboot_system_command
                        .unwrap_or_else(|| "adb reboot".into())
                };
                self.run_cmd("reboot_system", &cmd, None);
            }
            Action::RebootRecovery => {
                let cmd = config
                    .adb_reboot_recovery_command
                    .unwrap_or_else(|| "adb reboot recovery".into());
                self.run_cmd("reboot_recovery", &cmd, None);
            }
            Action::RebootBootloader => {
                let cmd = config
                    .adb_reboot_bootloader_command
                    .unwrap_or_else(|| "adb reboot bootloader".into());
                self.run_cmd("reboot_bootloader", &cmd, None);
            }
            Action::Sideload => {
                if let Some(path) = FileDialog::new().add_filter("zip", &["zip"]).pick_file() {
                    self.blocklist_warn(&path);
                    let args: Vec<String> =
                        vec!["sideload".into(), path.to_string_lossy().into_owned()];
                    self.start_process("sideload", "adb".into(), args);
                }
            }
            Action::PushFile => {
                if let Some(path) = FileDialog::new().pick_file() {
                    self.blocklist_warn(&path);
                    self.push_file(path);
                }
            }
            Action::PullFile => {
                if let Some(dir) = FileDialog::new().pick_folder() {
                    self.pull_file(dir);
                }
            }
            Action::BackupPartitions => self.backup_partitions(false),
            Action::FullNandBackup => self.backup_partitions(true),
            Action::RestoreBackup => {
                if let Some(dir) = FileDialog::new().pick_folder() {
                    self.restore_backup(dir);
                }
            }
            Action::ForensicDump => self.forensic_dump(),
            Action::QuickWipe => {
                let serial = self
                    .state
                    .try_lock()
                    .map(|s| s.device_serial.clone())
                    .unwrap_or_default();
                crate::safety::audit::log_action("quick_wipe", &serial);
                self.start_process(
                    "quick_wipe",
                    "fastboot".into(),
                    vec!["erase".into(), "userdata".into()],
                );
            }
            Action::SecureWipe => {
                let serial = self
                    .state
                    .try_lock()
                    .map(|s| s.device_serial.clone())
                    .unwrap_or_default();
                crate::safety::audit::log_action("secure_wipe", &serial);
                self.start_process(
                    "secure_wipe",
                    "adb".into(),
                    vec![
                        "shell".into(),
                        "dd".into(),
                        "if=/dev/urandom".into(),
                        "of=/dev/block/by-name/userdata".into(),
                        "bs=1M".into(),
                    ],
                );
            }
            Action::StartLogcat => {
                let logcat_state = self.logcat_state.clone();
                let handle = self.rt.handle().clone();
                let tx = self.executor.new_tx();
                self.rt.spawn(async move {
                    let mut ls = logcat_state.lock().await;
                    ls.start(&handle, tx);
                });
            }
            Action::StopLogcat => {
                let logcat_state = self.logcat_state.clone();
                self.rt.spawn(async move {
                    let mut ls = logcat_state.lock().await;
                    ls.stop();
                });
            }
            Action::CaptureDmesg => {
                let dmesg_state = self.dmesg_state.clone();
                let handle = self.rt.handle().clone();
                let tx = self.executor.new_tx();
                self.rt.spawn(async move {
                    let mut ds = dmesg_state.lock().await;
                    ds.capture(&handle, tx);
                });
            }
            Action::RunCustomCommand => self.run_custom_command(),
            Action::WifiConnect => self.wifi_connect(),
            Action::WifiDisconnect => self.wifi_disconnect(),
            Action::PickBootImg => {
                if let Some(path) = FileDialog::new().add_filter("image", &["img"]).pick_file() {
                    if let Ok(mut st) = self.state.try_lock() {
                        st.boot_img_path = Some(path);
                        st.boot_info = None;
                    }
                }
            }
            Action::AnalyzeBoot => self.analyze_boot(),
            Action::PatchBootMagisk => self.patch_boot_magisk(),
            Action::PickDtbo => {
                if let Some(path) = FileDialog::new()
                    .add_filter("dtbo", &["img", "dtbo"])
                    .pick_file()
                {
                    if let Ok(mut st) = self.state.try_lock() {
                        st.dtbo_path = Some(path);
                        st.dtbo_info = None;
                    }
                }
            }
            Action::ViewDtbo => self.view_dtbo(),
            Action::ReadPartitionHex => self.read_partition_hex(),
            Action::ExportProfile => {
                if let Some(path) = FileDialog::new()
                    .add_filter("yaml", &["yaml"])
                    .set_file_name("profile.yaml")
                    .save_file()
                {
                    let profile = crate::config::FlashProfile {
                        name: "custom".into(),
                        device: None,
                        operations: vec![],
                    };
                    match crate::config::save_profile(&profile, &path.to_string_lossy()) {
                        Ok(()) => self.append_log(
                            &format!("[*] Profile exported to {}", path.display()),
                            Color32::LIGHT_BLUE,
                        ),
                        Err(e) => {
                            self.append_log(&format!("[ERROR] Export profile: {}", e), Color32::RED)
                        }
                    }
                }
            }
            Action::ImportProfile => {
                if let Some(path) = FileDialog::new().add_filter("yaml", &["yaml"]).pick_file() {
                    match crate::config::load_profile(&path.to_string_lossy()) {
                        Ok(p) => self.append_log(
                            &format!(
                                "[*] Imported profile '{}' with {} operation(s).",
                                p.name,
                                p.operations.len()
                            ),
                            Color32::LIGHT_BLUE,
                        ),
                        Err(e) => {
                            self.append_log(&format!("[ERROR] Import profile: {}", e), Color32::RED)
                        }
                    }
                }
            }
            Action::ClearLogs => {
                if let Ok(mut st) = self.state.try_lock() {
                    st.logs.clear();
                }
            }
            Action::SaveSettings => self.save_settings(),
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 780.0])
            .with_min_inner_size([760.0, 560.0])
            .with_title("AndroKit"),
        ..Default::default()
    };
    eframe::run_native(
        "AndroKit",
        options,
        Box::new(|cc| Box::new(AndroKitApp::new(cc))),
    )
}
