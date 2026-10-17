use crate::protocols::{DeviceInfo, ProtocolId};

/// Maximum number of log lines retained in memory before the oldest are
/// dropped. Keeps long-running logcat sessions from growing unbounded.
pub const MAX_LOG_LINES: usize = 5000;

pub struct AppState {
    pub device_mode: String,
    pub device_serial: String,
    pub device_manufacturer: String,
    pub device_model: String,
    pub active_protocol: Option<ProtocolId>,
    pub device_info: Option<DeviceInfo>,

    pub is_command_running: bool,
    pub current_command_id: String,
    pub cancel_tx: Option<tokio::sync::mpsc::Sender<()>>,

    pub logs: Vec<(String, egui::Color32)>,
    pub status_bar: String,

    pub is_amd: bool,
    pub has_quirk: bool,
    pub show_quirk_prompt: bool,
    pub quirk_prompt_dismissed: bool,

    pub selected_tab: usize,

    // --- Log console ---
    pub log_filter: String,
    pub log_autoscroll: bool,

    // --- Flash options ---
    pub magisk_patch: bool,

    // --- Backup options ---
    pub encrypt_backup: bool,
    pub backup_passphrase: String,
    pub backup_partitions: String,

    // --- File transfer ---
    pub push_remote_path: String,
    pub pull_remote_path: String,

    // --- Protocols / custom command ---
    pub custom_command: String,

    // --- Advanced analyzers ---
    pub boot_img_path: Option<std::path::PathBuf>,
    pub boot_info: Option<String>,
    pub dtbo_path: Option<std::path::PathBuf>,
    pub dtbo_info: Option<String>,
    pub hex_path: String,
    pub hex_result: Option<String>,

    // --- Settings (persisted) ---
    pub wifi_ip: String,
    pub wifi_port: String,
    pub wifi_status: String,
    pub compliance_mode: bool,

    // --- Pending destructive action awaiting confirmation ---
    pub pending_confirm: Option<PendingAction>,
}

/// A destructive action held until the user explicitly confirms it in a modal.
#[derive(Clone, PartialEq)]
pub enum PendingAction {
    QuickWipe,
    SecureWipe,
    LockBootloader,
    UnlockBootloader,
}

impl PendingAction {
    pub fn title(&self) -> &'static str {
        match self {
            PendingAction::QuickWipe => "Confirm Quick Wipe",
            PendingAction::SecureWipe => "Confirm Secure Wipe",
            PendingAction::LockBootloader => "Confirm Lock Bootloader",
            PendingAction::UnlockBootloader => "Confirm Unlock Bootloader",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            PendingAction::QuickWipe => {
                "This will erase userdata on the connected device. All user data will be lost. This cannot be undone."
            }
            PendingAction::SecureWipe => {
                "This will overwrite userdata with random data (secure erase). All user data will be permanently destroyed. This cannot be undone and may take a long time."
            }
            PendingAction::LockBootloader => {
                "Locking the bootloader will factory-reset the device and may hard-brick it if the installed software is not signed. Ensure the device boots stock firmware first."
            }
            PendingAction::UnlockBootloader => {
                "Unlocking the bootloader will factory-reset the device (erasing all data) and may void your warranty."
            }
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            device_mode: "none".into(),
            device_serial: "N/A".into(),
            device_manufacturer: "Unknown".into(),
            device_model: "Unknown".into(),
            active_protocol: None,
            device_info: None,
            is_command_running: false,
            current_command_id: String::new(),
            cancel_tx: None,
            logs: vec![],
            status_bar: "Ready.".into(),
            is_amd: false,
            has_quirk: false,
            show_quirk_prompt: false,
            quirk_prompt_dismissed: false,
            selected_tab: 0,
            log_filter: String::new(),
            log_autoscroll: true,
            magisk_patch: false,
            encrypt_backup: false,
            backup_passphrase: String::new(),
            backup_partitions: "boot,recovery,dtbo,vbmeta".into(),
            push_remote_path: "/sdcard/".into(),
            pull_remote_path: "/sdcard/".into(),
            custom_command: String::new(),
            boot_img_path: None,
            boot_info: None,
            dtbo_path: None,
            dtbo_info: None,
            hex_path: "/dev/block/by-name/boot".into(),
            hex_result: None,
            wifi_ip: "192.168.1.".into(),
            wifi_port: "5555".into(),
            wifi_status: "Not connected".into(),
            compliance_mode: false,
            pending_confirm: None,
        }
    }
}

impl AppState {
    /// Push a log line, trimming the oldest entries past the retention cap.
    pub fn push_log(&mut self, text: String, color: egui::Color32) {
        self.logs.push((text, color));
        if self.logs.len() > MAX_LOG_LINES {
            let overflow = self.logs.len() - MAX_LOG_LINES;
            self.logs.drain(0..overflow);
        }
    }
}
