use serde::Serialize;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Serialize)]
pub struct DistroInfo {
    pub id: String,
    pub id_like: String,
    pub name: String,
    pub version: String,
    pub package_manager: String,
}

impl DistroInfo {
    pub fn detect() -> Self {
        let fields = parse_os_release().unwrap_or_default();

        let id = fields.get("ID").cloned().unwrap_or_default();
        let id_like = fields.get("ID_LIKE").cloned().unwrap_or_default();
        let name = fields
            .get("PRETTY_NAME")
            .cloned()
            .unwrap_or_else(|| "Unknown Linux".into());
        let version = fields.get("VERSION_ID").cloned().unwrap_or_default();
        let package_manager = detect_package_manager(&id, &id_like);

        DistroInfo {
            id,
            id_like,
            name,
            version,
            package_manager,
        }
    }
}

fn parse_os_release() -> Option<HashMap<String, String>> {
    let content = fs::read_to_string("/etc/os-release")
        .or_else(|_| fs::read_to_string("/usr/lib/os-release"))
        .ok()?;

    let mut map = HashMap::new();
    for line in content.lines() {
        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim_matches('"').to_string();
            map.insert(key.to_string(), value);
        }
    }
    Some(map)
}

fn detect_package_manager(id: &str, id_like: &str) -> String {
    let combined = format!("{} {}", id, id_like);
    if combined.contains("arch") {
        "pacman".into()
    } else if combined.contains("debian") || combined.contains("ubuntu") {
        "apt".into()
    } else if combined.contains("fedora") || combined.contains("rhel") || combined.contains("centos") {
        "dnf".into()
    } else if combined.contains("opensuse") || combined.contains("suse") {
        "zypper".into()
    } else if combined.contains("void") {
        "xbps".into()
    } else if combined.contains("alpine") {
        "apk".into()
    } else {
        "unknown".into()
    }
}
