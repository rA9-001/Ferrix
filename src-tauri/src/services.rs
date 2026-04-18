use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub description: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub enabled: String,
    pub service_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceLogs {
    pub lines: Vec<String>,
}

pub fn list_services() -> Vec<ServiceInfo> {
    let mut services = Vec::new();

    let output = Command::new("systemctl")
        .args(["list-units", "--type=service", "--all", "--no-pager", "--no-legend", "--plain"])
        .output();

    let unit_names: Vec<String> = match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.is_empty() {
                        return None;
                    }
                    let name = parts[0].strip_suffix(".service").unwrap_or(parts[0]);
                    Some(name.to_string())
                })
                .collect()
        }
        Err(_) => return services,
    };

    // Get enabled state for all units
    let enabled_output = Command::new("systemctl")
        .args(["list-unit-files", "--type=service", "--no-pager", "--no-legend", "--plain"])
        .output();

    let mut enabled_map = std::collections::HashMap::new();
    if let Ok(out) = enabled_output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].strip_suffix(".service").unwrap_or(parts[0]);
                enabled_map.insert(name.to_string(), parts[1].to_string());
            }
        }
    }

    // Get detailed info via show
    for name in &unit_names {
        let show_output = Command::new("systemctl")
            .args([
                "show",
                &format!("{}.service", name),
                "--no-pager",
                "--property=Description,LoadState,ActiveState,SubState,Type",
            ])
            .output();

        let (mut description, mut load_state, mut active_state, mut sub_state, mut svc_type) =
            (String::new(), String::new(), String::new(), String::new(), String::new());

        if let Ok(out) = show_output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if let Some((key, val)) = line.split_once('=') {
                    match key {
                        "Description" => description = val.to_string(),
                        "LoadState" => load_state = val.to_string(),
                        "ActiveState" => active_state = val.to_string(),
                        "SubState" => sub_state = val.to_string(),
                        "Type" => svc_type = val.to_string(),
                        _ => {}
                    }
                }
            }
        }

        let enabled = enabled_map
            .get(name)
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());

        services.push(ServiceInfo {
            name: name.clone(),
            description,
            load_state,
            active_state,
            sub_state,
            enabled,
            service_type: svc_type,
        });
    }

    services.sort_by(|a, b| a.name.cmp(&b.name));
    services
}

pub fn start_service(name: &str) -> ServiceResult {
    run_systemctl("start", name)
}

pub fn stop_service(name: &str) -> ServiceResult {
    run_systemctl("stop", name)
}

pub fn restart_service(name: &str) -> ServiceResult {
    run_systemctl("restart", name)
}

pub fn enable_service(name: &str) -> ServiceResult {
    run_systemctl("enable", name)
}

pub fn disable_service(name: &str) -> ServiceResult {
    run_systemctl("disable", name)
}

pub fn get_service_logs(name: &str, lines: u32) -> ServiceLogs {
    let output = Command::new("journalctl")
        .args([
            "-u",
            &format!("{}.service", name),
            "--no-pager",
            "-n",
            &lines.to_string(),
            "--output=short-iso",
        ])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            ServiceLogs {
                lines: stdout.lines().map(|l| l.to_string()).collect(),
            }
        }
        Err(e) => ServiceLogs {
            lines: vec![format!("Failed to read logs: {}", e)],
        },
    }
}

fn run_systemctl(action: &str, name: &str) -> ServiceResult {
    // Validate service name: only allow alphanumeric, dash, underscore, dot, @
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '@')
    {
        return ServiceResult {
            success: false,
            message: "Invalid service name".to_string(),
        };
    }

    let output = Command::new("pkexec")
        .args(["systemctl", action, &format!("{}.service", name)])
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                ServiceResult {
                    success: true,
                    message: format!("Successfully {} {}", past_tense(action), name),
                }
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                ServiceResult {
                    success: false,
                    message: format!("Failed to {} {}: {}", action, name, stderr.trim()),
                }
            }
        }
        Err(e) => ServiceResult {
            success: false,
            message: format!("Failed to execute: {}", e),
        },
    }
}

fn past_tense(action: &str) -> &str {
    match action {
        "start" => "started",
        "stop" => "stopped",
        "restart" => "restarted",
        "enable" => "enabled",
        "disable" => "disabled",
        _ => action,
    }
}
