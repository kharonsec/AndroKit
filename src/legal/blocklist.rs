use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct BlocklistEntry {
    pub pattern: String,
    pub warning: String,
}

#[derive(Debug, Deserialize)]
struct BlocklistFile {
    pub malware: Vec<BlocklistEntry>,
}

pub fn load_blocklist(path: &str) -> Vec<BlocklistEntry> {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    serde_yaml::from_str::<BlocklistFile>(&content)
        .map(|f| f.malware)
        .unwrap_or_default()
}

pub fn check_filename(filename: &str, blocklist: &[BlocklistEntry]) -> Option<String> {
    let lower = filename.to_lowercase();
    for entry in blocklist {
        let pattern = entry.pattern.trim_matches('*').to_lowercase();
        if pattern.is_empty() {
            continue;
        }
        if lower.contains(&pattern) {
            return Some(entry.warning.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries() -> Vec<BlocklistEntry> {
        vec![
            BlocklistEntry {
                pattern: "*frp_bypass*".into(),
                warning: "Known FRP bypass tool".into(),
            },
            BlocklistEntry {
                pattern: "malware.apk".into(),
                warning: "Flagged malware".into(),
            },
        ]
    }

    #[test]
    fn matches_wildcard_pattern() {
        let w = check_filename("super_frp_bypass_v2.zip", &entries());
        assert_eq!(w.as_deref(), Some("Known FRP bypass tool"));
    }

    #[test]
    fn match_is_case_insensitive() {
        assert!(check_filename("MALWARE.APK", &entries()).is_some());
    }

    #[test]
    fn clean_filename_passes() {
        assert!(check_filename("twrp-3.7.0.img", &entries()).is_none());
    }
}
