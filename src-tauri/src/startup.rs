use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupEntry {
    pub id: String,
    pub name: String,
    pub comment: String,
    pub command: String,
    pub enabled: bool,
    pub source: String,       // "user", "system", "systemd"
    pub source_path: String,
    pub entry_type: String,   // "desktop", "systemd"
    pub icon: String,
}

/// Scan all startup entries from autostart dirs and systemd user services.
pub fn get_startup_entries() -> Vec<StartupEntry> {
    let mut entries = Vec::new();

    // User autostart: ~/.config/autostart/
    if let Some(config_dir) = dirs::config_dir() {
        let user_autostart = config_dir.join("autostart");
        scan_desktop_dir(&user_autostart, "user", &mut entries);
    }

    // System autostart: /etc/xdg/autostart/
    let system_autostart = Path::new("/etc/xdg/autostart");
    scan_desktop_dir(system_autostart, "system", &mut entries);

    // Systemd user services
    scan_systemd_user_services(&mut entries);

    // Deduplicate: if a user entry overrides a system one (same filename), keep user's
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut deduped: Vec<StartupEntry> = Vec::new();
    for entry in entries {
        if let Some(&existing_idx) = seen.get(&entry.id) {
            // User entries override system entries
            if entry.source == "user" && deduped[existing_idx].source == "system" {
                deduped[existing_idx] = entry;
            }
        } else {
            seen.insert(entry.id.clone(), deduped.len());
            deduped.push(entry);
        }
    }

    // Sort: enabled first, then alphabetically
    deduped.sort_by(|a, b| {
        b.enabled.cmp(&a.enabled).then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    deduped
}

/// Parse .desktop files from an autostart directory.
fn scan_desktop_dir(dir: &Path, source: &str, entries: &mut Vec<StartupEntry>) {
    let read = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return,
    };

    for entry in read.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
            continue;
        }

        if let Some(se) = parse_desktop_file(&path, source) {
            entries.push(se);
        }
    }
}

/// Parse a single .desktop autostart file.
fn parse_desktop_file(path: &Path, source: &str) -> Option<StartupEntry> {
    let content = fs::read_to_string(path).ok()?;

    let mut name = String::new();
    let mut comment = String::new();
    let mut exec = String::new();
    let mut hidden = false;
    let mut no_display = false;
    let mut icon = String::new();
    let mut in_desktop_entry = false;

    for line in content.lines() {
        let line = line.trim();
        if line == "[Desktop Entry]" {
            in_desktop_entry = true;
            continue;
        }
        if line.starts_with('[') {
            in_desktop_entry = false;
            continue;
        }
        if !in_desktop_entry {
            continue;
        }

        if let Some(val) = line.strip_prefix("Name=") {
            name = val.to_string();
        } else if let Some(val) = line.strip_prefix("Comment=") {
            comment = val.to_string();
        } else if let Some(val) = line.strip_prefix("Exec=") {
            exec = val.to_string();
        } else if let Some(val) = line.strip_prefix("Icon=") {
            icon = val.to_string();
        } else if let Some(val) = line.strip_prefix("Hidden=") {
            hidden = val.trim().eq_ignore_ascii_case("true");
        } else if let Some(val) = line.strip_prefix("NoDisplay=") {
            no_display = val.trim().eq_ignore_ascii_case("true");
        }
        // Also handle Name[xx]= style — skip, we only want the base Name
        // Also handle "X-GNOME-Autostart-enabled=false" as disabled
        if let Some(val) = line.strip_prefix("X-GNOME-Autostart-enabled=") {
            if val.trim().eq_ignore_ascii_case("false") {
                hidden = true;
            }
        }
    }

    if name.is_empty() && exec.is_empty() {
        return None;
    }

    let file_name = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    if name.is_empty() {
        name = file_name.clone();
    }

    // Skip entries that are just noise (e.g. NoDisplay with no real exec)
    if no_display && exec.is_empty() {
        return None;
    }

    Some(StartupEntry {
        id: file_name.clone(),
        name,
        comment: enrich_description(&comment, &file_name, &exec),
        command: exec,
        enabled: !hidden,
        source: source.to_string(),
        source_path: path.to_string_lossy().to_string(),
        entry_type: "desktop".to_string(),
        icon,
    })
}

/// Provide a human-readable description for well-known startup entries.
/// Falls back to generating a description from the command if unknown.
fn enrich_description(comment: &str, id: &str, command: &str) -> String {
    if !comment.is_empty() {
        return comment.to_string();
    }

    // Well-known desktop/system entries
    let desc = match id {
        // KDE Plasma
        "baloo_file" => "File search indexer — indexes file contents and metadata for quick search",
        "kglobalacceld" => "Handles global keyboard shortcuts across the desktop",
        "gmenudbusmenuproxy" => "Bridges GTK application menus to the KDE panel (global menu support)",
        "org.kde.plasma-fallback-session-restore" => "Restores your previous session's windows and state on login",
        "org.kde.plasmashell" => "The main Plasma desktop shell — taskbar, system tray, and widgets",
        "xembedsniproxy" => "Translates legacy system tray icons (XEmbed) to the modern StatusNotifier protocol",
        "pam_kwallet_init" => "Auto-unlocks KWallet password storage using your login credentials",
        "polkit-kde-authentication-agent-1" => "Shows password prompts when applications request elevated privileges",
        "powerdevil" => "Battery, display brightness, and CPU power management",
        "kaccess" => "Accessibility features — screen reader, sticky keys, and visual notifications",
        "plasma-kded" | "kded5" | "kded6" => "Background service daemon for KDE — handles hardware events and notifications",
        "org.kde.kscreen.osd" => "Shows on-screen display when monitors are connected or disconnected",
        "org.kde.kdeconnect.daemon" => "Connects your phone to your desktop for notifications, file sharing, and remote control",

        // GNOME
        "gnome-keyring-pkcs11" | "gnome-keyring-secrets" | "gnome-keyring-ssh" => "GNOME Keyring — securely stores passwords, keys, and certificates",
        "tracker-miner-fs-3" | "tracker-miner-fs" => "File search indexer — indexes files for GNOME search",
        "evolution-alarm-notify" => "Calendar alarm notifications from Evolution/GNOME Calendar",

        // Common system
        "at-spi-dbus-bus" => "Accessibility service bridge — enables screen readers and assistive technology",
        "xdg-user-dirs" => "Creates and updates standard user folders (Documents, Downloads, Music, etc.)",
        "spice-vdagent" => "Virtual machine guest agent — clipboard sharing and display resizing",
        "nm-applet" => "NetworkManager system tray applet — manages Wi-Fi and network connections",
        "blueman" => "Bluetooth manager — handles pairing and connecting Bluetooth devices",
        "pulseaudio" | "pipewire" | "pipewire-pulse" => "Audio server — manages sound input/output for all applications",
        "wireplumber" => "PipeWire session manager — handles audio/video routing and device management",

        // Common apps
        "portmaster-autostart" | "portmaster" => "Application firewall — monitors and controls per-app network access",
        "steam" => "Steam gaming client background service",
        "discord" => "Discord chat client — starts minimized to system tray",
        "slack" => "Slack workspace messenger",
        "dropbox" => "Dropbox cloud file sync service",
        "syncthing-gtk" | "syncthing" => "Syncthing — continuous peer-to-peer file synchronization",
        "flameshot" => "Screenshot tool with annotation support",
        "copyq" => "Clipboard manager — stores clipboard history",
        "redshift" | "redshift-gtk" => "Adjusts screen color temperature based on time of day (blue light filter)",
        "gammastep" => "Adjusts screen color temperature for eye comfort (Wayland blue light filter)",
        "albert" => "Application launcher and quick search",
        "ulauncher" => "Application launcher with extensions support",
        "megasync" => "MEGA cloud storage sync client",
        "insync" => "Google Drive and OneDrive sync client",
        "nextcloud" => "Nextcloud file sync and share client",
        "owncloud" => "ownCloud file sync client",
        "barrier" => "Share mouse and keyboard between multiple computers",
        "input-remapper" | "input-remapper-autoload" => "Remaps keyboard keys, mouse buttons, and gamepad inputs",
        "solaar" => "Logitech device manager — battery status and settings for wireless peripherals",

        _ => "",
    };

    if !desc.is_empty() {
        return desc.to_string();
    }

    // For systemd services, try matching service name patterns
    let id_lower = id.to_lowercase();
    let id_lower = id_lower.strip_prefix("systemd-").unwrap_or(&id_lower);

    if id_lower.contains("pipewire") {
        return "Audio/video server — manages media streams for applications".to_string();
    } else if id_lower.contains("pulse") {
        return "PulseAudio compatibility layer for PipeWire".to_string();
    } else if id_lower.contains("xdg-desktop-portal") {
        return "Desktop integration portal — provides sandboxed apps access to files, screen sharing, etc.".to_string();
    } else if id_lower.contains("dbus") {
        return "D-Bus message bus — inter-process communication for desktop services".to_string();
    }

    // Generate from command as last resort
    if !command.is_empty() {
        let binary = command
            .split_whitespace()
            .next()
            .unwrap_or("")
            .rsplit('/')
            .next()
            .unwrap_or("");
        if !binary.is_empty() {
            return format!("Runs {} on startup", binary);
        }
    }

    String::new()
}

/// Scan systemd user services.
fn scan_systemd_user_services(entries: &mut Vec<StartupEntry>) {
    // List all enabled user services
    let output = Command::new("systemctl")
        .args(["--user", "list-unit-files", "--type=service", "--state=enabled", "--no-pager", "--no-legend"])
        .output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let unit_name = parts[0];
                let service_name = unit_name.strip_suffix(".service").unwrap_or(unit_name);
                let description = get_systemd_description(unit_name);
                let id = format!("systemd-{}", service_name);
                let cmd = format!("systemctl --user start {}", unit_name);

                entries.push(StartupEntry {
                    comment: enrich_description(&description, &id, &cmd),
                    id,
                    name: service_name.to_string(),
                    command: cmd,
                    enabled: true,
                    source: "systemd".to_string(),
                    source_path: format!("systemd --user: {}", unit_name),
                    entry_type: "systemd".to_string(),
                    icon: String::new(),
                });
            }
        }
    }

    // Also list disabled user services
    let output = Command::new("systemctl")
        .args(["--user", "list-unit-files", "--type=service", "--state=disabled", "--no-pager", "--no-legend"])
        .output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let unit_name = parts[0];
                let service_name = unit_name.strip_suffix(".service").unwrap_or(unit_name);
                let description = get_systemd_description(unit_name);
                let id = format!("systemd-{}", service_name);
                let cmd = format!("systemctl --user start {}", unit_name);

                entries.push(StartupEntry {
                    comment: enrich_description(&description, &id, &cmd),
                    id,
                    name: service_name.to_string(),
                    command: cmd,
                    enabled: false,
                    source: "systemd".to_string(),
                    source_path: format!("systemd --user: {}", unit_name),
                    entry_type: "systemd".to_string(),
                    icon: String::new(),
                });
            }
        }
    }
}

/// Get the description of a systemd unit.
fn get_systemd_description(unit: &str) -> String {
    if !is_valid_unit_name(unit) {
        return String::new();
    }
    let output = Command::new("systemctl")
        .args(["--user", "show", "--", unit, "--property=Description", "--no-pager"])
        .output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        if let Some(desc) = stdout.trim().strip_prefix("Description=") {
            return desc.to_string();
        }
    }
    String::new()
}

/// Toggle a startup entry on or off.
pub fn toggle_startup_entry(id: &str, enabled: bool) -> Result<String, String> {
    // Find the entry first
    let entries = get_startup_entries();
    let entry = entries.iter().find(|e| e.id == id).ok_or("Entry not found")?;

    match entry.entry_type.as_str() {
        "desktop" => toggle_desktop_entry(entry, enabled),
        "systemd" => toggle_systemd_entry(entry, enabled),
        _ => Err("Unknown entry type".to_string()),
    }
}

/// Toggle a .desktop autostart entry.
fn toggle_desktop_entry(entry: &StartupEntry, enabled: bool) -> Result<String, String> {
    if entry.source == "system" {
        // For system entries, create a user override
        let config_dir = dirs::config_dir().ok_or("Cannot find config directory")?;
        let user_autostart = config_dir.join("autostart");
        fs::create_dir_all(&user_autostart).map_err(|e| e.to_string())?;

        let filename = format!("{}.desktop", entry.id);
        let user_path = user_autostart.join(&filename);

        // Copy the system file to user dir, then set Hidden
        let system_content = fs::read_to_string(&entry.source_path).map_err(|e| e.to_string())?;
        let new_content = set_desktop_hidden(&system_content, !enabled);
        fs::write(&user_path, new_content).map_err(|e| e.to_string())?;
    } else {
        // User entry — modify in place
        let content = fs::read_to_string(&entry.source_path).map_err(|e| e.to_string())?;
        let new_content = set_desktop_hidden(&content, !enabled);
        fs::write(&entry.source_path, new_content).map_err(|e| e.to_string())?;
    }

    let action = if enabled { "enabled" } else { "disabled" };
    Ok(format!("{} {}", entry.name, action))
}

/// Set or update the Hidden= key in a .desktop file.
fn set_desktop_hidden(content: &str, hidden: bool) -> String {
    let hidden_val = if hidden { "true" } else { "false" };
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let mut found_hidden = false;
    let mut found_gnome = false;
    let mut insert_idx = None;

    for (i, line) in lines.iter_mut().enumerate() {
        if line.starts_with("Hidden=") {
            *line = format!("Hidden={}", hidden_val);
            found_hidden = true;
        }
        if line.starts_with("X-GNOME-Autostart-enabled=") {
            *line = format!("X-GNOME-Autostart-enabled={}", !hidden);
            found_gnome = true;
        }
        // Track where [Desktop Entry] section content ends
        if line == "[Desktop Entry]" {
            insert_idx = Some(i + 1);
        }
    }

    if !found_hidden {
        if let Some(idx) = insert_idx {
            lines.insert(idx, format!("Hidden={}", hidden_val));
        } else {
            lines.push(format!("Hidden={}", hidden_val));
        }
    }

    if !found_gnome {
        if let Some(idx) = insert_idx {
            let pos = if !found_hidden { idx + 1 } else { idx };
            lines.insert(pos, format!("X-GNOME-Autostart-enabled={}", !hidden));
        }
    }

    lines.join("\n")
}

/// Validate a systemd unit name. Allows the standard charset and rejects
/// anything that could be interpreted as a flag (leading '-') or path.
fn is_valid_unit_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 256
        && !name.starts_with('-')
        && !name.contains('/')
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '@' || c == ':')
}

/// Toggle a systemd user service.
fn toggle_systemd_entry(entry: &StartupEntry, enabled: bool) -> Result<String, String> {
    if !is_valid_unit_name(&entry.name) {
        return Err("Invalid systemd unit name".to_string());
    }
    let service_name = format!("{}.service", entry.name);
    let action = if enabled { "enable" } else { "disable" };

    let output = Command::new("systemctl")
        .args(["--user", action, "--", &service_name])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to {} {}: {}", action, service_name, stderr.trim()));
    }

    Ok(format!("{} {}", entry.name, if enabled { "enabled" } else { "disabled" }))
}

/// Remove a user autostart entry.
pub fn remove_startup_entry(id: &str) -> Result<String, String> {
    let entries = get_startup_entries();
    let entry = entries.iter().find(|e| e.id == id).ok_or("Entry not found")?;

    match entry.entry_type.as_str() {
        "desktop" => {
            if entry.source != "user" {
                return Err("Cannot remove system entries. You can only disable them.".to_string());
            }
            fs::remove_file(&entry.source_path).map_err(|e| e.to_string())?;
            Ok(format!("Removed {}", entry.name))
        }
        "systemd" => {
            if !is_valid_unit_name(&entry.name) {
                return Err("Invalid systemd unit name".to_string());
            }
            let service_name = format!("{}.service", entry.name);
            // Disable first, then mask to prevent re-enabling
            let output = Command::new("systemctl")
                .args(["--user", "disable", "--", &service_name])
                .output()
                .map_err(|e| e.to_string())?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Failed to disable {}: {}", service_name, stderr.trim()));
            }
            Ok(format!("Disabled {}", entry.name))
        }
        _ => Err("Unknown entry type".to_string()),
    }
}

/// Add a new user autostart entry.
pub fn add_startup_entry(name: &str, command: &str, comment: &str) -> Result<String, String> {
    // Reject control characters in any field — these would allow injecting
    // arbitrary keys (e.g. malicious "Exec=") into the .desktop file.
    let has_ctrl = |s: &str| s.chars().any(|c| c.is_control());
    if has_ctrl(name) || has_ctrl(command) || has_ctrl(comment) {
        return Err("Input contains control characters".to_string());
    }
    if name.trim().is_empty() {
        return Err("Name cannot be empty".to_string());
    }
    if command.trim().is_empty() {
        return Err("Command cannot be empty".to_string());
    }
    // Reasonable length caps
    if name.len() > 128 || command.len() > 1024 || comment.len() > 512 {
        return Err("Input exceeds maximum length".to_string());
    }

    let config_dir = dirs::config_dir().ok_or("Cannot find config directory")?;
    let autostart_dir = config_dir.join("autostart");
    fs::create_dir_all(&autostart_dir).map_err(|e| e.to_string())?;

    // Sanitize name for filename
    let safe_name: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    if safe_name.is_empty() || safe_name == "." || safe_name == ".." {
        return Err("Invalid name".to_string());
    }

    let filename = format!("{}.desktop", safe_name);
    let filepath = autostart_dir.join(&filename);

    // Defense-in-depth: ensure resolved path is still inside autostart_dir
    // (no path traversal via crafted name even though we sanitize above).
    if filepath.parent() != Some(autostart_dir.as_path()) {
        return Err("Invalid path".to_string());
    }

    if filepath.exists() {
        return Err(format!("An entry named '{}' already exists", safe_name));
    }

    let content = format!(
        "[Desktop Entry]\nType=Application\nName={}\nExec={}\nComment={}\nHidden=false\nX-GNOME-Autostart-enabled=true\n",
        name, command, comment
    );

    fs::write(&filepath, content).map_err(|e| e.to_string())?;

    Ok(format!("Added '{}' to startup", name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_name_accepts_real_units() {
        for ok in ["sshd.service", "my-app@user.service", "timer-1.timer", "foo_bar.socket"] {
            assert!(is_valid_unit_name(ok), "should accept {ok}");
        }
    }

    #[test]
    fn unit_name_rejects_injection() {
        for bad in [
            "", "-foo.service", "foo bar.service", "foo;rm.service",
            "foo$(id).service", "foo`id`.service", "foo/bar.service",
            "foo\nbar", "foo\0bar", &"a".repeat(300),
        ] {
            assert!(!is_valid_unit_name(bad), "should reject {:?}", bad);
        }
    }
}
