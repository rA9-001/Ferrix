use serde::{Deserialize, Serialize};
use crate::sysenv::system_command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallStatus {
    pub backend: String,
    pub active: bool,
    pub default_incoming: String,
    pub default_outgoing: String,
    pub logging: String,
    pub rules: Vec<FirewallRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallRule {
    pub number: u32,
    pub to: String,
    pub action: String,
    pub from: String,
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallActionResponse {
    pub result: FirewallResult,
    pub status: FirewallStatus,
}

/// Run a ufw mutation command and then read back both status views in one pkexec call.
/// Returns (command_output, verbose_status, numbered_status).
fn run_ufw_with_status(action_args: &[&str]) -> Result<(String, String, String), String> {
    let ufw_cmd = format!("ufw {} && echo __UFW_SEP__ && ufw status verbose && echo __UFW_SEP__ && ufw status numbered",
        action_args.iter().map(|a| shell_escape(a)).collect::<Vec<_>>().join(" "));

    let output = system_command("pkexec")
        .args(["bash", "-c", &ufw_cmd])
        .output()
        .map_err(|e| format!("Failed to run pkexec: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !output.status.success() {
        return Err(if stderr.is_empty() { stdout.trim().to_string() } else { stderr });
    }

    let parts: Vec<&str> = stdout.split("__UFW_SEP__").collect();
    if parts.len() >= 3 {
        Ok((
            parts[0].trim().to_string(),
            parts[1].trim().to_string(),
            parts[2].trim().to_string(),
        ))
    } else {
        // Fallback: command ran but separator parsing failed
        Ok((stdout.trim().to_string(), String::new(), String::new()))
    }
}

/// Read both status views in a single pkexec call.
fn read_ufw_status() -> Result<(String, String), String> {
    let output = system_command("pkexec")
        .args(["bash", "-c", "ufw status verbose && echo __UFW_SEP__ && ufw status numbered"])
        .output()
        .map_err(|e| format!("Failed to run pkexec: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !output.status.success() {
        return Err(if stderr.is_empty() { stdout.trim().to_string() } else { stderr });
    }

    let parts: Vec<&str> = stdout.split("__UFW_SEP__").collect();
    if parts.len() >= 2 {
        Ok((parts[0].trim().to_string(), parts[1].trim().to_string()))
    } else {
        Ok((stdout.trim().to_string(), String::new()))
    }
}

fn shell_escape(s: &str) -> String {
    if s.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/' || c == ':') {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

fn build_status_from(verbose: &str, numbered: &str) -> FirewallStatus {
    let backend = match detect_backend() {
        Some(b) => b,
        None => "none",
    };
    let (active, incoming, outgoing, logging) = parse_verbose_status(verbose);
    let rules = parse_numbered_rules(numbered);
    FirewallStatus {
        backend: backend.into(),
        active,
        default_incoming: incoming,
        default_outgoing: outgoing,
        logging,
        rules,
    }
}

fn detect_backend() -> Option<&'static str> {
    if system_command("which").arg("ufw").output().map(|o| o.status.success()).unwrap_or(false) {
        Some("ufw")
    } else {
        None
    }
}

fn parse_verbose_status(output: &str) -> (bool, String, String, String) {
    let mut active = false;
    let mut incoming = String::from("unknown");
    let mut outgoing = String::from("unknown");
    let mut logging = String::from("off");

    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("Status:") {
            active = line.contains("active") && !line.contains("inactive");
        } else if line.starts_with("Default:") {
            let defaults = line.trim_start_matches("Default:").trim();
            for part in defaults.split(',') {
                let part = part.trim();
                if part.contains("(incoming)") {
                    incoming = part.split_whitespace().next().unwrap_or("unknown").to_string();
                } else if part.contains("(outgoing)") {
                    outgoing = part.split_whitespace().next().unwrap_or("unknown").to_string();
                }
            }
        } else if line.starts_with("Logging:") {
            logging = line.trim_start_matches("Logging:").trim().to_string();
        }
    }

    (active, incoming, outgoing, logging)
}

fn parse_numbered_rules(output: &str) -> Vec<FirewallRule> {
    let mut rules = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        // Match lines like: [ 1] 22/tcp ALLOW IN Anywhere
        if !line.starts_with('[') {
            continue;
        }

        let bracket_end = match line.find(']') {
            Some(i) => i,
            None => continue,
        };

        let num_str = line[1..bracket_end].trim();
        let number: u32 = match num_str.parse() {
            Ok(n) => n,
            Err(_) => continue,
        };

        let rest = line[bracket_end + 1..].trim();

        // Extract comment if present (marked with # at the end)
        let (rule_part, comment) = if let Some(hash_pos) = rest.rfind('#') {
            (rest[..hash_pos].trim(), rest[hash_pos + 1..].trim().to_string())
        } else {
            (rest, String::new())
        };

        // Parse: TO  ACTION  FROM
        // The action keywords are: ALLOW, DENY, REJECT, LIMIT
        // Direction indicators: IN, OUT, FWD
        let action_keywords = ["ALLOW", "DENY", "REJECT", "LIMIT"];
        let mut action_start = None;
        let mut action_end = None;

        let upper = rule_part.to_uppercase();
        for keyword in &action_keywords {
            if let Some(pos) = upper.find(keyword) {
                action_start = Some(pos);
                // Find end of action (includes IN/OUT/FWD if present)
                let after = &upper[pos + keyword.len()..];
                let extra = if after.starts_with(" IN") {
                    keyword.len() + 3
                } else if after.starts_with(" OUT") {
                    keyword.len() + 4
                } else if after.starts_with(" FWD") {
                    keyword.len() + 4
                } else {
                    keyword.len()
                };
                action_end = Some(pos + extra);
                break;
            }
        }

        let (to, action, from) = match (action_start, action_end) {
            (Some(start), Some(end)) => {
                let to = rule_part[..start].trim().to_string();
                let action = rule_part[start..end].trim().to_string();
                let from = rule_part[end..].trim().to_string();
                (to, action, from)
            }
            _ => (rule_part.to_string(), String::new(), String::new()),
        };

        rules.push(FirewallRule {
            number,
            to,
            action,
            from,
            comment,
        });
    }

    rules
}

pub fn get_firewall_status() -> FirewallStatus {
    if detect_backend().is_none() {
        return FirewallStatus {
            backend: "none".into(),
            active: false,
            default_incoming: "unknown".into(),
            default_outgoing: "unknown".into(),
            logging: "off".into(),
            rules: vec![],
        };
    }

    let (verbose, numbered) = read_ufw_status().unwrap_or_default();
    build_status_from(&verbose, &numbered)
}

pub fn toggle_firewall(enable: bool) -> FirewallActionResponse {
    let arg = if enable { "enable" } else { "disable" };
    let action_args: Vec<&str> = if enable {
        vec!["--force", "enable"]
    } else {
        vec!["disable"]
    };

    match run_ufw_with_status(&action_args) {
        Ok((msg, verbose, numbered)) => FirewallActionResponse {
            result: FirewallResult {
                success: true,
                message: format!("Firewall {}d: {}", arg, msg),
            },
            status: build_status_from(&verbose, &numbered),
        },
        Err(msg) => FirewallActionResponse {
            result: FirewallResult {
                success: false,
                message: msg,
            },
            status: get_firewall_status(),
        },
    }
}

pub fn set_default_policy(direction: &str, policy: &str) -> FirewallActionResponse {
    let valid_dirs = ["incoming", "outgoing", "routed"];
    let valid_policies = ["allow", "deny", "reject"];

    if !valid_dirs.contains(&direction) {
        return FirewallActionResponse {
            result: FirewallResult {
                success: false,
                message: format!("Invalid direction: {}. Use: incoming, outgoing, routed", direction),
            },
            status: get_firewall_status(),
        };
    }
    if !valid_policies.contains(&policy) {
        return FirewallActionResponse {
            result: FirewallResult {
                success: false,
                message: format!("Invalid policy: {}. Use: allow, deny, reject", policy),
            },
            status: get_firewall_status(),
        };
    }

    match run_ufw_with_status(&["default", policy, direction]) {
        Ok((msg, verbose, numbered)) => FirewallActionResponse {
            result: FirewallResult {
                success: true,
                message: msg,
            },
            status: build_status_from(&verbose, &numbered),
        },
        Err(msg) => FirewallActionResponse {
            result: FirewallResult {
                success: false,
                message: msg,
            },
            status: get_firewall_status(),
        },
    }
}

pub fn add_rule(
    action: &str,
    direction: &str,
    port: &str,
    protocol: &str,
    from_ip: &str,
    comment: &str,
) -> FirewallActionResponse {
    let valid_actions = ["allow", "deny", "reject", "limit"];
    if !valid_actions.contains(&action) {
        return FirewallActionResponse {
            result: FirewallResult {
                success: false,
                message: format!("Invalid action: {}", action),
            },
            status: get_firewall_status(),
        };
    }

    // Strict allow-list validation for every user-supplied field.
    // This is defense in depth — even though shell_escape handles quoting,
    // we never want anything weird reaching the shell layer.
    let valid_dirs = ["", "in", "out"];
    if !valid_dirs.contains(&direction) {
        return FirewallActionResponse {
            result: FirewallResult { success: false, message: "Invalid direction".into() },
            status: get_firewall_status(),
        };
    }
    if !is_valid_port(port) {
        return FirewallActionResponse {
            result: FirewallResult { success: false, message: "Invalid port".into() },
            status: get_firewall_status(),
        };
    }
    let valid_protos = ["", "any", "tcp", "udp"];
    if !valid_protos.contains(&protocol) {
        return FirewallActionResponse {
            result: FirewallResult { success: false, message: "Invalid protocol".into() },
            status: get_firewall_status(),
        };
    }
    if !is_valid_from_ip(from_ip) {
        return FirewallActionResponse {
            result: FirewallResult { success: false, message: "Invalid source address".into() },
            status: get_firewall_status(),
        };
    }
    if !is_valid_comment(comment) {
        return FirewallActionResponse {
            result: FirewallResult { success: false, message: "Invalid comment (max 80 chars, no control chars)".into() },
            status: get_firewall_status(),
        };
    }

    let mut args: Vec<String> = Vec::new();

    // Build: ufw [allow|deny|reject|limit] [in|out] [from <ip>] [to any port <port>[/<proto>]] [comment <comment>]
    args.push(action.to_string());

    if direction == "out" {
        args.push("out".into());
    }

    if !from_ip.is_empty() && from_ip != "any" {
        args.push("from".into());
        args.push(from_ip.to_string());
    }

    if !port.is_empty() {
        args.push("to".into());
        args.push("any".into());
        args.push("port".into());

        if !protocol.is_empty() && protocol != "any" {
            args.push(port.to_string());
            args.push("proto".into());
            args.push(protocol.to_string());
        } else {
            args.push(port.to_string());
        }
    }

    if !comment.is_empty() {
        args.push("comment".into());
        args.push(comment.to_string());
    }

    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    match run_ufw_with_status(&arg_refs) {
        Ok((msg, verbose, numbered)) => FirewallActionResponse {
            result: FirewallResult {
                success: true,
                message: msg,
            },
            status: build_status_from(&verbose, &numbered),
        },
        Err(msg) => FirewallActionResponse {
            result: FirewallResult {
                success: false,
                message: msg,
            },
            status: get_firewall_status(),
        },
    }
}

pub fn delete_rule(number: u32) -> FirewallActionResponse {
    if number == 0 {
        return FirewallActionResponse {
            result: FirewallResult {
                success: false,
                message: "Invalid rule number".into(),
            },
            status: get_firewall_status(),
        };
    }

    let num_str = number.to_string();
    match run_ufw_with_status(&["--force", "delete", &num_str]) {
        Ok((msg, verbose, numbered)) => FirewallActionResponse {
            result: FirewallResult {
                success: true,
                message: msg,
            },
            status: build_status_from(&verbose, &numbered),
        },
        Err(msg) => FirewallActionResponse {
            result: FirewallResult {
                success: false,
                message: msg,
            },
            status: get_firewall_status(),
        },
    }
}

pub fn set_logging(level: &str) -> FirewallActionResponse {
    let valid = ["off", "low", "medium", "high", "full"];
    if !valid.contains(&level) {
        return FirewallActionResponse {
            result: FirewallResult {
                success: false,
                message: format!("Invalid logging level: {}. Use: off, low, medium, high, full", level),
            },
            status: get_firewall_status(),
        };
    }

    match run_ufw_with_status(&["logging", level]) {
        Ok((msg, verbose, numbered)) => FirewallActionResponse {
            result: FirewallResult {
                success: true,
                message: msg,
            },
            status: build_status_from(&verbose, &numbered),
        },
        Err(msg) => FirewallActionResponse {
            result: FirewallResult {
                success: false,
                message: msg,
            },
            status: get_firewall_status(),
        },
    }
}

/// Validate a port: empty (no port), single number 1-65535, range "n:m", or comma list.
fn is_valid_port(port: &str) -> bool {
    if port.is_empty() {
        return true;
    }
    if port.len() > 64 {
        return false;
    }
    for part in port.split(',') {
        if part.is_empty() {
            return false;
        }
        for sub in part.split(':') {
            if sub.is_empty() || sub.len() > 5 {
                return false;
            }
            match sub.parse::<u32>() {
                Ok(n) if (1..=65535).contains(&n) => {}
                _ => return false,
            }
        }
    }
    true
}

/// Validate an IP source: empty, "any", or an IPv4/IPv6 (optionally with /prefix).
fn is_valid_from_ip(s: &str) -> bool {
    if s.is_empty() || s == "any" {
        return true;
    }
    if s.len() > 64 {
        return false;
    }
    // Allowed charset for IP/CIDR: hex digits, dots, colons, and one optional /n suffix.
    let (addr, prefix) = match s.split_once('/') {
        Some((a, p)) => (a, Some(p)),
        None => (s, None),
    };
    if addr.is_empty()
        || !addr
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c == '.' || c == ':')
    {
        return false;
    }
    if let Some(p) = prefix {
        match p.parse::<u32>() {
            Ok(n) if n <= 128 => {}
            _ => return false,
        }
    }
    // Must parse as a real IP address.
    addr.parse::<std::net::IpAddr>().is_ok()
}

/// Validate a comment: printable ASCII only, no control chars or quotes, max 80.
fn is_valid_comment(s: &str) -> bool {
    if s.len() > 80 {
        return false;
    }
    s.chars().all(|c| {
        !c.is_control()
            && c != '\''
            && c != '"'
            && c != '`'
            && c != '\\'
            && c != '$'
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_accepts_valid() {
        for ok in ["", "22", "80,443", "8000:9000", "1", "65535"] {
            assert!(is_valid_port(ok), "should accept {ok}");
        }
    }

    #[test]
    fn port_rejects_injection() {
        for bad in [
            "-1", "22;rm", "22 80", "22|nc", "$(id)", "`id`",
            "22\n80", "22\080", "22/tcp/foo", "22 ",
            // Length > 64
            &"1".repeat(70),
        ] {
            assert!(!is_valid_port(bad), "should reject {:?}", bad);
        }
    }

    #[test]
    fn from_ip_accepts_valid() {
        for ok in ["", "any", "192.168.1.1", "10.0.0.0/8", "::1", "fe80::1/64"] {
            assert!(is_valid_from_ip(ok), "should accept {ok}");
        }
    }

    #[test]
    fn from_ip_rejects_injection() {
        for bad in [
            "1.2.3.4;rm", "1.2.3.4 5.6.7.8", "$(id)", "`id`",
            "1.2.3.4\n", "1.2.3.4\0", "1.2.3.4|nc", &"1".repeat(60),
        ] {
            assert!(!is_valid_from_ip(bad), "should reject {:?}", bad);
        }
    }

    #[test]
    fn comment_rejects_control_and_metachars() {
        for bad in ["hi\nthere", "hi`id`", "hi$x", "hi\0", &"a".repeat(500)] {
            assert!(!is_valid_comment(bad), "should reject {:?}", bad);
        }
        assert!(is_valid_comment("allow ssh from office"));
    }
}
