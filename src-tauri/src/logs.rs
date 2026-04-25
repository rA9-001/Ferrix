use serde::Serialize;
use crate::sysenv::system_command;

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub priority: String,
    pub unit: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogQueryResult {
    pub entries: Vec<LogEntry>,
    pub total_returned: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct JournalUnit {
    pub name: String,
    pub description: String,
}

pub fn get_journal_units() -> Vec<JournalUnit> {
    let mut units = Vec::new();
    if let Ok(out) = system_command("journalctl")
        .args(["--field=_SYSTEMD_UNIT"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                units.push(JournalUnit {
                    name: trimmed.to_string(),
                    description: String::new(),
                });
            }
        }
        units.sort_by(|a, b| a.name.cmp(&b.name));
    }
    units
}

pub fn query_logs(
    priority: &str,
    unit: &str,
    boot: &str,
    lines: u32,
    grep: &str,
    since: &str,
) -> LogQueryResult {
    // Clamp line count to a sane upper bound to prevent DoS.
    let lines = lines.min(50_000);

    let mut args: Vec<String> = vec![
        "--no-pager".into(),
        "--output=json".into(),
        format!("--lines={}", lines),
    ];

    // priority: only digit(s) 0-7, optionally a range like "0..3"
    if !priority.is_empty()
        && priority.len() <= 5
        && priority
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.')
    {
        args.push(format!("--priority={}", priority));
    }
    if !unit.is_empty() {
        // Validate unit name (systemd unit charset)
        if unit.len() <= 256
            && unit.chars().all(|c| {
                c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '@' || c == ':'
            })
        {
            args.push(format!("--unit={}", unit));
        }
    }
    if !boot.is_empty() {
        // boot ID is either a signed integer offset or a 32-char hex string
        let valid_boot = (boot.len() <= 12
            && boot.starts_with(|c: char| c == '-' || c.is_ascii_digit())
            && boot[1..].chars().all(|c| c.is_ascii_digit()))
            || boot.chars().all(|c| c.is_ascii_hexdigit());
        if valid_boot && boot.len() <= 64 {
            args.push(format!("--boot={}", boot));
        }
    }
    if !grep.is_empty() {
        // UTF-8-safe truncation to 200 chars; reject control chars (\n etc.)
        let safe_grep: String = grep
            .chars()
            .filter(|c| !c.is_control())
            .take(200)
            .collect();
        if !safe_grep.is_empty() {
            args.push(format!("--grep={}", safe_grep));
        }
    }
    if !since.is_empty() {
        // Restrict to safe charset for journalctl --since (digits, letters, spaces, dashes, colons)
        if since.len() <= 30
            && since.chars().all(|c| {
                c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == ':' || c == '.'
            })
        {
            args.push(format!("--since={}", since));
        }
    }

    // Reverse so newest first
    args.push("--reverse".into());

    let mut entries = Vec::new();

    if let Ok(out) = system_command("journalctl")
        .args(&args)
        .output()
    {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                let timestamp = val
                    .get("__REALTIME_TIMESTAMP")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(|us| {
                        let secs = us / 1_000_000;
                        let ndt = chrono::DateTime::from_timestamp(secs as i64, 0)
                            .unwrap_or_default();
                        let local = ndt.with_timezone(&chrono::Local);
                        local.format("%Y-%m-%d %H:%M:%S").to_string()
                    })
                    .unwrap_or_default();

                let priority_num = val
                    .get("PRIORITY")
                    .and_then(|v| v.as_str())
                    .unwrap_or("6");
                let priority_label = match priority_num {
                    "0" => "emerg",
                    "1" => "alert",
                    "2" => "crit",
                    "3" => "err",
                    "4" => "warning",
                    "5" => "notice",
                    "6" => "info",
                    "7" => "debug",
                    _ => "unknown",
                };

                let unit_name = val
                    .get("_SYSTEMD_UNIT")
                    .or_else(|| val.get("SYSLOG_IDENTIFIER"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let message = val
                    .get("MESSAGE")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                entries.push(LogEntry {
                    timestamp,
                    priority: priority_label.to_string(),
                    unit: unit_name,
                    message,
                });
            }
        }
    }

    let total_returned = entries.len();
    LogQueryResult {
        entries,
        total_returned,
    }
}

pub fn get_boot_list() -> Vec<String> {
    let mut boots = Vec::new();
    if let Ok(out) = system_command("journalctl")
        .args(["--list-boots", "--no-pager"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                // Format: " -2 abc123 Wed 2024-01-01 ... — Wed 2024-01-02 ..."
                // We want a display label
                boots.push(trimmed.to_string());
            }
        }
    }
    boots
}
