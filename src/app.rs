use crate::advanced::dmesg::DmesgState;
use crate::advanced::logcat::LogcatState;
use crate::config::FullConfig;
use crate::executor::{Executor, ProcessOutput};
use crate::state::AppState;
use crossbeam_channel::{unbounded, Receiver};
use eframe::egui::Color32;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(PartialEq)]
pub enum Action {
    None,
    ApplyQuirk,
    DetectDetails,
    UnlockBootloader,
    LockBootloader,
    FlashRecovery,
    FlashBoot,
    RebootSystem,
    RebootRecovery,
    RebootBootloader,
    Sideload,
    PushFile,
    PullFile,
    BackupPartitions,
    RestoreBackup,
    FullNandBackup,
    ForensicDump,
    QuickWipe,
    SecureWipe,
    StartLogcat,
    StopLogcat,
    CaptureDmesg,
    RunCustomCommand,
    WifiConnect,
    WifiDisconnect,
    PickBootImg,
    AnalyzeBoot,
    PatchBootMagisk,
    PickDtbo,
    ViewDtbo,
    ReadPartitionHex,
    ExportProfile,
    ImportProfile,
    ClearLogs,
    SaveSettings,
    /// Request confirmation before running a destructive action.
    Confirm(crate::state::PendingAction),
    ToggleTab(usize),
}

pub struct AndroKitApp {
    pub state: Arc<Mutex<AppState>>,
    pub config: FullConfig,
    pub rx: Receiver<ProcessOutput>,
    pub rt: tokio::runtime::Runtime,
    pub executor: Executor,
    last_checked_time: std::time::Instant,
    pub logcat_state: Arc<Mutex<LogcatState>>,
    pub dmesg_state: Arc<Mutex<DmesgState>>,
}

impl AndroKitApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = unbounded();
        let rt = tokio::runtime::Runtime::new().expect("Unable to create Runtime");
        let executor = Executor::new(tx.clone(), rt.handle().clone());
        let config = FullConfig::load("devices.yaml");

        // Apply persisted user settings to the initial state.
        let settings = crate::config::UserSettings::load();
        let initial = AppState {
            wifi_ip: settings.wifi_ip,
            wifi_port: settings.wifi_port,
            compliance_mode: settings.compliance_mode,
            backup_partitions: settings.backup_partitions,
            encrypt_backup: settings.encrypt_backup,
            ..AppState::default()
        };

        let app = Self {
            state: Arc::new(Mutex::new(initial)),
            config,
            rx,
            rt,
            executor,
            last_checked_time: std::time::Instant::now(),
            logcat_state: Arc::new(Mutex::new(LogcatState::default())),
            dmesg_state: Arc::new(Mutex::new(DmesgState::default())),
        };

        app.check_system_info();
        app
    }

    /// Persist the current user-facing settings to disk.
    pub fn save_settings(&self) {
        let settings = {
            let Ok(st) = self.state.try_lock() else {
                return;
            };
            crate::config::UserSettings {
                wifi_ip: st.wifi_ip.clone(),
                wifi_port: st.wifi_port.clone(),
                compliance_mode: st.compliance_mode,
                backup_partitions: st.backup_partitions.clone(),
                encrypt_backup: st.encrypt_backup,
            }
        };
        match settings.save() {
            Ok(()) => self.set_status("Settings saved."),
            Err(e) => self.append_log(&format!("[ERROR] Save settings: {}", e), Color32::RED),
        }
    }

    fn check_system_info(&self) {
        let state_clone = self.state.clone();
        self.rt.spawn(async move {
            let is_amd = std::fs::read_to_string("/proc/cpuinfo")
                .map(|s| s.contains("AuthenticAMD"))
                .unwrap_or(false);
            let has_quirk = std::fs::read_to_string("/sys/module/usbcore/parameters/quirks")
                .map(|s| s.contains("18d1:d00d:k"))
                .unwrap_or(false);
            let mut st = state_clone.lock().await;
            st.is_amd = is_amd;
            st.has_quirk = has_quirk;
        });
    }

    pub fn apply_quirk(&self) {
        let tx = self.executor.new_tx();
        self.rt.spawn(async move {
            let _ = tx.send(ProcessOutput::Stdout(
                "Applying temporary USB quirk...".into(),
            ));
            let output = std::process::Command::new("sh")
                .arg("-c")
                .arg("echo '18d1:d00d:k' | sudo tee /sys/module/usbcore/parameters/quirks")
                .output();
            match output {
                Ok(out) if out.status.success() => {
                    let _ = tx.send(ProcessOutput::Stdout("Quirk applied successfully!".into()));
                }
                _ => {
                    let _ = tx.send(ProcessOutput::Error(
                        "Failed to apply quirk. Root permissions may be required.".into(),
                    ));
                }
            }
        });
    }

    pub fn append_log(&self, text: &str, color: Color32) {
        if let Ok(mut state) = self.state.try_lock() {
            state.logs.push((text.to_string(), color));
        }
    }

    pub fn set_status(&self, text: &str) {
        if let Ok(mut state) = self.state.try_lock() {
            state.status_bar = text.to_string();
        }
    }

    pub fn run_cmd(&self, id: &str, command_str: &str, warning: Option<&str>) {
        if command_str.is_empty() {
            self.append_log(&format!("[ERROR] Command '{}' is empty!", id), Color32::RED);
            return;
        }
        if let Some(msg) = warning {
            self.append_log(&format!("[WARNING] {}", msg), Color32::YELLOW);
        }
        let parts: Vec<&str> = command_str.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }
        self.start_process(
            id,
            parts[0].into(),
            parts[1..].iter().map(|s| s.to_string()).collect(),
        );
    }

    pub fn start_process(&self, id: &str, program: String, args: Vec<String>) {
        if let Ok(mut state) = self.state.try_lock() {
            if state.is_command_running {
                state.logs.push((
                    "[WARNING] A command is already running.".into(),
                    Color32::YELLOW,
                ));
                return;
            }
            state.is_command_running = true;
            state.current_command_id = id.to_string();
            let (cancel_tx, cancel_rx) = tokio::sync::mpsc::channel(1);
            state.cancel_tx = Some(cancel_tx);
            self.append_log(
                &format!("[*] Starting: {} {:?}", program, args),
                Color32::LIGHT_BLUE,
            );
            self.executor
                .run_command(id.to_string(), program, args, cancel_rx);
        }
    }

    pub fn check_device_state(&mut self) {
        if self.last_checked_time.elapsed().as_secs() < 3 {
            return;
        }
        self.last_checked_time = std::time::Instant::now();
        {
            if let Ok(state) = self.state.try_lock() {
                if state.is_command_running {
                    return;
                }
            }
        }
        let state_clone = self.state.clone();
        self.rt.spawn(async move {
            let mut detected_mode = "none".to_string();
            let mut detected_serial = "N/A".to_string();
            if let Ok((success, stdout, _)) =
                crate::executor::Executor::run_command_sync("adb", &["devices"]).await
            {
                if success {
                    for line in stdout.lines().skip(1) {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() == 2 {
                            let status = parts[1];
                            match status {
                                "device" => {
                                    detected_mode = "adb".into();
                                    detected_serial = parts[0].into();
                                    break;
                                }
                                "recovery" => {
                                    detected_mode = "recovery".into();
                                    detected_serial = parts[0].into();
                                    break;
                                }
                                "sideload" => {
                                    detected_mode = "sideload".into();
                                    detected_serial = parts[0].into();
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            if detected_mode == "none" {
                if let Ok((success, stdout, _)) =
                    crate::executor::Executor::run_command_sync("fastboot", &["devices"]).await
                {
                    if success {
                        for line in stdout.lines() {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 2 && parts[1] == "fastboot" {
                                detected_mode = "fastboot".into();
                                detected_serial = parts[0].into();
                                break;
                            }
                        }
                    }
                }
            }
            let mut st = state_clone.lock().await;
            st.device_mode = detected_mode;
            st.device_serial = detected_serial;
            if st.device_mode == "none" {
                st.device_manufacturer = "Unknown".into();
                st.device_model = "Unknown".into();
            }
        });
    }

    pub fn run_detect_details(&self) {
        let mode = {
            let Ok(st) = self.state.try_lock() else {
                return;
            };
            st.device_mode.clone()
        };
        if mode == "adb" || mode == "recovery" || mode == "sideload" {
            self.set_status("Getting ADB properties...");
            self.start_process(
                "detect_adb_props",
                "adb".into(),
                vec!["shell".into(), "getprop".into()],
            );
        } else if mode == "fastboot" {
            self.set_status("Getting Fastboot variables...");
            self.start_process(
                "detect_fastboot_vars",
                "fastboot".into(),
                vec!["getvar".into(), "all".into()],
            );
        }
    }

    /// Run an arbitrary adb/fastboot command typed by the user. Only the two
    /// known front-end binaries are permitted as the program.
    pub fn run_custom_command(&self) {
        let cmd = {
            let Ok(st) = self.state.try_lock() else {
                return;
            };
            st.custom_command.trim().to_string()
        };
        if cmd.is_empty() {
            self.append_log("[WARNING] Custom command is empty.", Color32::YELLOW);
            return;
        }
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let program = parts[0];
        if program != "adb" && program != "fastboot" {
            self.append_log(
                "[ERROR] Custom commands must start with 'adb' or 'fastboot'.",
                Color32::RED,
            );
            return;
        }
        self.run_cmd("custom", &cmd, None);
    }

    pub fn wifi_connect(&self) {
        let (ip, port) = {
            let Ok(st) = self.state.try_lock() else {
                return;
            };
            (st.wifi_ip.clone(), st.wifi_port.clone())
        };
        let Ok(port_num) = port.parse::<u16>() else {
            self.append_log("[ERROR] Invalid Wi-Fi ADB port.", Color32::RED);
            return;
        };
        let state = self.state.clone();
        let tx = self.executor.new_tx();
        self.rt.spawn(async move {
            let _ = tx.send(ProcessOutput::Stdout(format!(
                "Connecting to {}:{}...",
                ip, port_num
            )));
            match crate::network::wifi_adb::connect(&ip, port_num).await {
                Ok(out) => {
                    let _ = tx.send(ProcessOutput::Stdout(out.trim().to_string()));
                    if let Ok(mut st) = state.try_lock() {
                        st.wifi_status = format!("Connected to {}:{}", ip, port_num);
                    }
                }
                Err(e) => {
                    let _ = tx.send(ProcessOutput::Error(format!("Wi-Fi ADB: {}", e.trim())));
                    if let Ok(mut st) = state.try_lock() {
                        st.wifi_status = "Connection failed".into();
                    }
                }
            }
        });
    }

    pub fn wifi_disconnect(&self) {
        let (ip, port) = {
            let Ok(st) = self.state.try_lock() else {
                return;
            };
            (st.wifi_ip.clone(), st.wifi_port.clone())
        };
        let Ok(port_num) = port.parse::<u16>() else {
            self.append_log("[ERROR] Invalid Wi-Fi ADB port.", Color32::RED);
            return;
        };
        let state = self.state.clone();
        let tx = self.executor.new_tx();
        self.rt.spawn(async move {
            match crate::network::wifi_adb::disconnect(&ip, port_num).await {
                Ok(out) => {
                    let _ = tx.send(ProcessOutput::Stdout(out.trim().to_string()));
                }
                Err(e) => {
                    let _ = tx.send(ProcessOutput::Error(format!("Wi-Fi ADB: {}", e.trim())));
                }
            }
            if let Ok(mut st) = state.try_lock() {
                st.wifi_status = "Not connected".into();
            }
        });
    }

    /// Parse the selected boot image header and store a human-readable summary.
    pub fn analyze_boot(&self) {
        let path = {
            let Ok(st) = self.state.try_lock() else {
                return;
            };
            st.boot_img_path.clone()
        };
        let Some(path) = path else {
            self.append_log("[WARNING] No boot image selected.", Color32::YELLOW);
            return;
        };
        let result = match std::fs::read(&path) {
            Ok(data) => match crate::analyzer::boot::parse_boot_header(&data) {
                Ok(info) => Ok(format!(
                    "Magic: ANDROID!\n\
                     Kernel:  size={} bytes  addr=0x{:08x}\n\
                     Ramdisk: size={} bytes  addr=0x{:08x}\n\
                     Second:  size={} bytes  addr=0x{:08x}\n\
                     Tags addr: 0x{:08x}\n\
                     Page size: {} bytes\n\
                     Has DTB: {}\n\
                     Cmdline: {}",
                    info.kernel_size,
                    info.kernel_addr,
                    info.ramdisk_size,
                    info.ramdisk_addr,
                    info.second_size,
                    info.second_addr,
                    info.tags_addr,
                    info.page_size,
                    info.has_dtb,
                    info.cmdline,
                )),
                Err(e) => Err(e),
            },
            Err(e) => Err(format!("Read error: {}", e)),
        };
        if let Ok(mut st) = self.state.try_lock() {
            match result {
                Ok(text) => {
                    st.boot_info = Some(text);
                    st.push_log(
                        format!("[*] Analyzed boot image: {}", path.display()),
                        Color32::LIGHT_BLUE,
                    );
                }
                Err(e) => {
                    st.boot_info = Some(format!("Error: {}", e));
                    st.push_log(format!("[ERROR] Boot analyze: {}", e), Color32::RED);
                }
            }
        }
    }

    /// Patch the selected boot image with Magisk (requires magiskboot on PATH).
    pub fn patch_boot_magisk(&self) {
        let path = {
            let Ok(st) = self.state.try_lock() else {
                return;
            };
            st.boot_img_path.clone()
        };
        let Some(path) = path else {
            self.append_log(
                "[WARNING] Select a boot image before patching.",
                Color32::YELLOW,
            );
            return;
        };
        let tx = self.executor.new_tx();
        self.rt.spawn(async move {
            let _ = tx.send(ProcessOutput::Stdout(format!(
                "Patching {} with Magisk...",
                path.display()
            )));
            match crate::flash::magisk::patch_boot_image(&path) {
                Ok(out) => {
                    let _ = tx.send(ProcessOutput::Stdout(format!(
                        "Patched image written to {}",
                        out.display()
                    )));
                }
                Err(e) => {
                    let _ = tx.send(ProcessOutput::Error(format!("Magisk patch: {}", e)));
                }
            }
        });
    }

    /// Parse the selected DTBO header and store a summary.
    pub fn view_dtbo(&self) {
        let path = {
            let Ok(st) = self.state.try_lock() else {
                return;
            };
            st.dtbo_path.clone()
        };
        let Some(path) = path else {
            self.append_log("[WARNING] No DTBO file selected.", Color32::YELLOW);
            return;
        };
        let result = match std::fs::read(&path) {
            Ok(data) => crate::analyzer::dtbo::parse_dtbo_header(&data).map(|info| {
                format!(
                    "Magic: 0x{:08x}\n\
                     Total size: {} bytes\n\
                     Header size: {} bytes\n\
                     DT entry size: {} bytes\n\
                     DT entry count: {}",
                    info.magic,
                    info.total_size,
                    info.header_size,
                    info.dt_entry_size,
                    info.dt_entry_count,
                )
            }),
            Err(e) => Err(format!("Read error: {}", e)),
        };
        if let Ok(mut st) = self.state.try_lock() {
            match result {
                Ok(text) => st.dtbo_info = Some(text),
                Err(e) => st.dtbo_info = Some(format!("Error: {}", e)),
            }
        }
    }

    /// Read the first 256 bytes of a partition on the device and render a hex
    /// dump into the viewer. Read-only.
    pub fn read_partition_hex(&self) {
        let path = {
            let Ok(st) = self.state.try_lock() else {
                return;
            };
            st.hex_path.trim().to_string()
        };
        if path.is_empty() {
            self.append_log("[WARNING] Enter a partition path.", Color32::YELLOW);
            return;
        }
        let state = self.state.clone();
        let tx = self.executor.new_tx();
        self.rt.spawn(async move {
            let count = "256";
            let script = format!("dd if={} bs=1 count={} 2>/dev/null | xxd", path, count);
            match crate::executor::Executor::run_command_sync(
                "adb",
                &["shell", "su", "-c", &script],
            )
            .await
            {
                Ok((true, stdout, _)) if !stdout.trim().is_empty() => {
                    if let Ok(mut st) = state.try_lock() {
                        st.hex_result = Some(stdout);
                    }
                }
                Ok((_, _, stderr)) => {
                    let msg = if stderr.trim().is_empty() {
                        "No output (device may require root, or path is invalid).".to_string()
                    } else {
                        stderr.trim().to_string()
                    };
                    if let Ok(mut st) = state.try_lock() {
                        st.hex_result = Some(format!("Error: {}", msg));
                    }
                    let _ = tx.send(ProcessOutput::Error(format!("Hex read: {}", msg)));
                }
                Err(e) => {
                    if let Ok(mut st) = state.try_lock() {
                        st.hex_result = Some(format!("Error: {}", e));
                    }
                    let _ = tx.send(ProcessOutput::Error(format!("Hex read: {}", e)));
                }
            }
        });
    }

    /// Back up a set of partitions (comma-separated in state) to the backups
    /// directory, optionally encrypting each image with AES-256-GCM.
    pub fn backup_partitions(&self, full: bool) {
        let (serial, list, encrypt, passphrase) = {
            let Ok(st) = self.state.try_lock() else {
                return;
            };
            (
                st.device_serial.clone(),
                st.backup_partitions.clone(),
                st.encrypt_backup,
                st.backup_passphrase.clone(),
            )
        };
        if serial == "N/A" {
            self.append_log("[ERROR] No device connected.", Color32::RED);
            return;
        }
        if encrypt && passphrase.is_empty() {
            self.append_log(
                "[ERROR] Encryption enabled but passphrase is empty.",
                Color32::RED,
            );
            return;
        }
        let partitions: Vec<String> = if full {
            vec!["userdata".into(), "super".into(), "system".into()]
        } else {
            list.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        };
        if partitions.is_empty() {
            self.append_log("[WARNING] No partitions specified.", Color32::YELLOW);
            return;
        }
        crate::safety::audit::log_action(
            if full {
                "full_nand_backup"
            } else {
                "backup_partitions"
            },
            &serial,
        );
        let state = self.state.clone();
        let tx = self.executor.new_tx();
        self.set_running(true, "backup");
        self.rt.spawn(async move {
            let part_refs: Vec<&str> = partitions.iter().map(|s| s.as_str()).collect();
            let result = crate::safety::backup::backup_partitions(&serial, &part_refs).await;
            match result {
                Ok(dir) => {
                    let _ = tx.send(ProcessOutput::Stdout(format!(
                        "Backup complete: {}",
                        dir.display()
                    )));
                    if encrypt {
                        match encrypt_dir_images(&dir, &passphrase) {
                            Ok(n) => {
                                let _ = tx.send(ProcessOutput::Stdout(format!(
                                    "Encrypted {} image(s) with AES-256-GCM.",
                                    n
                                )));
                            }
                            Err(e) => {
                                let _ = tx.send(ProcessOutput::Error(format!("Encryption: {}", e)));
                            }
                        }
                    }
                    let _ = tx.send(ProcessOutput::Finished(true, "backup".into()));
                }
                Err(e) => {
                    let _ = tx.send(ProcessOutput::Error(format!("Backup: {}", e)));
                    let _ = tx.send(ProcessOutput::Finished(false, "backup".into()));
                }
            }
            if let Ok(mut st) = state.try_lock() {
                st.is_command_running = false;
                st.current_command_id.clear();
            }
        });
    }

    /// Restore `.img` files from a user-selected backup directory, decrypting
    /// `.enc` files first when a passphrase is set.
    pub fn restore_backup(&self, dir: std::path::PathBuf) {
        let (serial, passphrase) = {
            let Ok(st) = self.state.try_lock() else {
                return;
            };
            (st.device_serial.clone(), st.backup_passphrase.clone())
        };
        crate::safety::audit::log_action("restore_backup", &serial);
        let tx = self.executor.new_tx();
        self.set_running(true, "restore");
        let state = self.state.clone();
        self.rt.spawn(async move {
            match restore_from_dir(&dir, &passphrase, &tx).await {
                Ok(n) => {
                    let _ = tx.send(ProcessOutput::Stdout(format!(
                        "Restored {} partition(s).",
                        n
                    )));
                    let _ = tx.send(ProcessOutput::Finished(true, "restore".into()));
                }
                Err(e) => {
                    let _ = tx.send(ProcessOutput::Error(format!("Restore: {}", e)));
                    let _ = tx.send(ProcessOutput::Finished(false, "restore".into()));
                }
            }
            if let Ok(mut st) = state.try_lock() {
                st.is_command_running = false;
                st.current_command_id.clear();
            }
        });
    }

    /// Read-only forensic pull of key partitions plus a SHA-256 manifest.
    pub fn forensic_dump(&self) {
        let serial = {
            let Ok(st) = self.state.try_lock() else {
                return;
            };
            st.device_serial.clone()
        };
        if serial == "N/A" {
            self.append_log("[ERROR] No device connected.", Color32::RED);
            return;
        }
        crate::safety::audit::log_action("forensic_dump", &serial);
        let tx = self.executor.new_tx();
        self.set_running(true, "forensic");
        let state = self.state.clone();
        self.rt.spawn(async move {
            let _ = tx.send(ProcessOutput::Stdout(
                "Starting read-only forensic dump...".into(),
            ));
            let parts = ["boot", "recovery", "dtbo", "vbmeta"];
            let result = crate::safety::backup::backup_partitions(&serial, &parts).await;
            match result {
                Ok(dir) => {
                    // Write a SHA-256 manifest for evidentiary integrity.
                    let mut manifest = String::new();
                    if let Ok(entries) = std::fs::read_dir(&dir) {
                        for e in entries.flatten() {
                            let p = e.path();
                            if p.extension().is_some_and(|x| x == "img") {
                                if let Ok(sum) = crate::safety::checksum::checksum_file(&p) {
                                    manifest.push_str(&format!(
                                        "{}  {}\n",
                                        sum,
                                        p.file_name().unwrap_or_default().to_string_lossy()
                                    ));
                                }
                            }
                        }
                    }
                    let _ = std::fs::write(dir.join("SHA256SUMS.txt"), &manifest);
                    let _ = tx.send(ProcessOutput::Stdout(format!(
                        "Forensic dump complete: {}\nSHA-256 manifest written.",
                        dir.display()
                    )));
                    let _ = tx.send(ProcessOutput::Finished(true, "forensic".into()));
                }
                Err(e) => {
                    let _ = tx.send(ProcessOutput::Error(format!("Forensic dump: {}", e)));
                    let _ = tx.send(ProcessOutput::Finished(false, "forensic".into()));
                }
            }
            if let Ok(mut st) = state.try_lock() {
                st.is_command_running = false;
                st.current_command_id.clear();
            }
        });
    }

    pub fn push_file(&self, local: std::path::PathBuf) {
        let remote = {
            let Ok(st) = self.state.try_lock() else {
                return;
            };
            st.push_remote_path.clone()
        };
        let args = vec!["push".into(), local.to_string_lossy().into_owned(), remote];
        self.start_process("push", "adb".into(), args);
    }

    pub fn pull_file(&self, local_dir: std::path::PathBuf) {
        let remote = {
            let Ok(st) = self.state.try_lock() else {
                return;
            };
            st.pull_remote_path.clone()
        };
        let args = vec![
            "pull".into(),
            remote,
            local_dir.to_string_lossy().into_owned(),
        ];
        self.start_process("pull", "adb".into(), args);
    }

    /// Check a selected file name against the malware blocklist and emit a
    /// prominent warning to the console if it matches a known-bad pattern.
    /// Advisory only — it never blocks the operation.
    pub fn blocklist_warn(&self, path: &std::path::Path) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let list = crate::legal::blocklist::load_blocklist("blocklist.yaml");
        if let Some(warning) = crate::legal::blocklist::check_filename(&name, &list) {
            self.append_log(
                &format!("[WARNING] Blocklist match for '{}': {}", name, warning),
                Color32::from_rgb(220, 120, 0),
            );
        }
    }

    /// Mark a long-running (non-`Executor`) task as running so the UI disables
    /// conflicting buttons and shows the status bar.
    fn set_running(&self, running: bool, id: &str) {
        if let Ok(mut st) = self.state.try_lock() {
            st.is_command_running = running;
            st.current_command_id = if running {
                id.to_string()
            } else {
                String::new()
            };
        }
    }
}

/// Encrypt every `*.img` in `dir` into `*.img.enc`, removing the plaintext.
fn encrypt_dir_images(dir: &std::path::Path, passphrase: &str) -> Result<usize, String> {
    let mut count = 0;
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read_dir: {}", e))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "img") {
            let data = std::fs::read(&path).map_err(|e| format!("read: {}", e))?;
            let enc = crate::safety::encryption::encrypt_bytes(&data, passphrase)?;
            let out = path.with_extension("img.enc");
            std::fs::write(&out, enc).map_err(|e| format!("write: {}", e))?;
            std::fs::remove_file(&path).ok();
            count += 1;
        }
    }
    Ok(count)
}

/// Flash each image from a backup directory back to its partition. `.img.enc`
/// files are decrypted to a temp file first.
async fn restore_from_dir(
    dir: &std::path::Path,
    passphrase: &str,
    tx: &crossbeam_channel::Sender<ProcessOutput>,
) -> Result<usize, String> {
    let mut count = 0;
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read_dir: {}", e))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let (partition, flash_path): (String, std::path::PathBuf) = if name.ends_with(".img.enc") {
            if passphrase.is_empty() {
                return Err("Encrypted backup found but no passphrase set.".into());
            }
            let data = std::fs::read(&path).map_err(|e| format!("read: {}", e))?;
            let dec = crate::safety::encryption::decrypt_bytes(&data, passphrase)?;
            let part = name.trim_end_matches(".img.enc").to_string();
            let tmp = std::env::temp_dir().join(format!("androkit-{}.img", part));
            std::fs::write(&tmp, dec).map_err(|e| format!("write tmp: {}", e))?;
            (part, tmp)
        } else if name.ends_with(".img") {
            (name.trim_end_matches(".img").to_string(), path.clone())
        } else {
            continue;
        };
        let _ = tx.send(ProcessOutput::Stdout(format!(
            "Flashing {} -> {}",
            flash_path.display(),
            partition
        )));
        let (ok, _, stderr) = crate::executor::Executor::run_command_sync(
            "fastboot",
            &["flash", &partition, &flash_path.to_string_lossy()],
        )
        .await?;
        if !ok {
            return Err(format!("flash {} failed: {}", partition, stderr.trim()));
        }
        count += 1;
    }
    if count == 0 {
        return Err("No .img or .img.enc files found in directory.".into());
    }
    Ok(count)
}
