use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct DeviceConfig {
    pub unlock_command: Option<String>,
    pub lock_command: Option<String>,
    pub adb_reboot_bootloader_command: Option<String>,
    pub adb_reboot_recovery_command: Option<String>,
    pub adb_reboot_system_command: Option<String>,
    pub fastboot_reboot_command: Option<String>,
    pub recovery_partition: Option<String>,
    pub boot_partition: Option<String>,
    pub unlock_warning: Option<String>,
    pub lock_warning: Option<String>,
    #[allow(dead_code)]
    pub notes: Option<String>,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            unlock_command: Some("fastboot oem unlock".into()),
            lock_command: Some("fastboot oem lock".into()),
            adb_reboot_bootloader_command: Some("adb reboot bootloader".into()),
            adb_reboot_recovery_command: Some("adb reboot recovery".into()),
            adb_reboot_system_command: Some("adb reboot".into()),
            fastboot_reboot_command: Some("fastboot reboot".into()),
            recovery_partition: Some("recovery".into()),
            boot_partition: Some("boot".into()),
            unlock_warning: Some("Unlocking the bootloader will erase all data (factory reset) and may void your warranty! Proceed with caution.".into()),
            lock_warning: Some("Locking the bootloader will erase all data. Ensure you have backups before proceeding.".into()),
            notes: Some("Built-in default configuration.".into()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct FullConfig {
    pub default: Option<DeviceConfig>,
    #[serde(flatten)]
    pub devices: HashMap<String, DeviceConfig>,
}

impl FullConfig {
    pub fn load(path: &str) -> Self {
        let default_config = Self {
            default: Some(DeviceConfig::default()),
            devices: HashMap::new(),
        };

        let file_content = std::fs::read_to_string(path).unwrap_or_default();
        if file_content.is_empty() {
            return default_config;
        }

        match serde_yaml::from_str::<FullConfig>(&file_content) {
            Ok(mut config) => {
                if config.default.is_none() {
                    config.default = Some(DeviceConfig::default());
                }
                config
            }
            Err(e) => {
                eprintln!("Failed to parse config: {}", e);
                default_config
            }
        }
    }

    pub fn get_config(&self, manufacturer: &str, model: &str) -> DeviceConfig {
        let exact_key = format!("{} {}", manufacturer.to_lowercase(), model.to_lowercase());
        if let Some(config) = self.devices.get(&exact_key) {
            return config.clone();
        }

        let mfg_key = manufacturer.to_lowercase();
        if let Some(config) = self.devices.get(&mfg_key) {
            return config.clone();
        }

        self.default.clone().unwrap_or_default()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FlashProfile {
    pub name: String,
    pub device: Option<String>,
    pub operations: Vec<ProfileOp>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProfileOp {
    #[serde(rename = "type")]
    pub op_type: String,
    pub target: String,
    pub file: Option<String>,
}

pub fn save_profile(profile: &FlashProfile, path: &str) -> Result<(), String> {
    let yaml = serde_yaml::to_string(profile).map_err(|e| format!("Serialize: {}", e))?;
    std::fs::write(path, yaml).map_err(|e| format!("Write: {}", e))
}

pub fn load_profile(path: &str) -> Result<FlashProfile, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("Read: {}", e))?;
    serde_yaml::from_str(&content).map_err(|e| format!("Parse: {}", e))
}

/// User-facing preferences persisted between runs.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserSettings {
    pub wifi_ip: String,
    pub wifi_port: String,
    pub compliance_mode: bool,
    pub backup_partitions: String,
    pub encrypt_backup: bool,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            wifi_ip: "192.168.1.".into(),
            wifi_port: "5555".into(),
            compliance_mode: false,
            backup_partitions: "boot,recovery,dtbo,vbmeta".into(),
            encrypt_backup: false,
        }
    }
}

/// Returns `~/.androidtool/settings.json`, creating the parent directory.
pub fn settings_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let dir = std::path::PathBuf::from(home).join(".androidtool");
    std::fs::create_dir_all(&dir).ok();
    dir.join("settings.json")
}

impl UserSettings {
    pub fn load() -> Self {
        let content = std::fs::read_to_string(settings_path()).unwrap_or_default();
        if content.trim().is_empty() {
            return Self::default();
        }
        serde_json::from_str(&content).unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| format!("Serialize: {}", e))?;
        std::fs::write(settings_path(), json).map_err(|e| format!("Write: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_config_prefers_exact_then_mfg_then_default() {
        let mut devices = HashMap::new();
        devices.insert(
            "xiaomi".into(),
            DeviceConfig {
                unlock_command: Some("mfg-unlock".into()),
                ..DeviceConfig::default()
            },
        );
        devices.insert(
            "xiaomi mi 11".into(),
            DeviceConfig {
                unlock_command: Some("exact-unlock".into()),
                ..DeviceConfig::default()
            },
        );
        let cfg = FullConfig {
            default: Some(DeviceConfig::default()),
            devices,
        };
        assert_eq!(
            cfg.get_config("Xiaomi", "Mi 11").unlock_command.as_deref(),
            Some("exact-unlock")
        );
        assert_eq!(
            cfg.get_config("Xiaomi", "Unknown Model")
                .unlock_command
                .as_deref(),
            Some("mfg-unlock")
        );
        assert_eq!(
            cfg.get_config("Google", "Pixel").unlock_command.as_deref(),
            Some("fastboot oem unlock")
        );
    }

    #[test]
    fn profile_roundtrips_through_yaml() {
        let profile = FlashProfile {
            name: "test".into(),
            device: Some("pixel".into()),
            operations: vec![ProfileOp {
                op_type: "flash".into(),
                target: "boot".into(),
                file: Some("boot.img".into()),
            }],
        };
        let mut path = std::env::temp_dir();
        path.push(format!("androkit-profile-{}.yaml", std::process::id()));
        save_profile(&profile, &path.to_string_lossy()).unwrap();
        let loaded = load_profile(&path.to_string_lossy()).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(loaded.name, "test");
        assert_eq!(loaded.operations.len(), 1);
        assert_eq!(loaded.operations[0].target, "boot");
    }

    #[test]
    fn user_settings_serde_roundtrip() {
        let s = UserSettings {
            wifi_ip: "10.0.0.5".into(),
            wifi_port: "5556".into(),
            compliance_mode: true,
            backup_partitions: "boot".into(),
            encrypt_backup: true,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: UserSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.wifi_ip, "10.0.0.5");
        assert!(back.compliance_mode);
        assert!(back.encrypt_backup);
    }
}
