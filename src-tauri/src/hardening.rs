use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardeningStatus {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub sysctl_path: String,
    pub current_value: String,
    pub recommended_value: String,
    pub default_value: String,
    pub is_applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardeningResult {
    pub id: String,
    pub success: bool,
    pub message: String,
}

fn read_sysctl(key: &str) -> String {
    let path = format!("/proc/sys/{}", key.replace('.', "/"));
    fs::read_to_string(&path)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn apply_sysctl(key: &str, value: &str) -> Result<String, String> {
    // Apply at runtime
    let param = format!("{}={}", key, value);
    let output = Command::new("pkexec")
        .args(["sysctl", "-w", &param])
        .output()
        .map_err(|e| format!("Failed to run pkexec: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    // Persist via /etc/sysctl.d/ so it survives reboot
    persist_sysctl(key, value);

    Ok(format!("Set {} = {}", key, value))
}

fn persist_sysctl(key: &str, value: &str) {
    let conf_path = "/etc/sysctl.d/99-ferrix-hardening.conf";
    let line = format!("{} = {}", key, value);

    // Read existing file or start fresh
    let existing = Command::new("cat")
        .arg(conf_path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    // Update or append the key
    let mut lines: Vec<String> = existing
        .lines()
        .filter(|l| {
            let trimmed = l.trim();
            !trimmed.is_empty()
                && !trimmed.starts_with('#')
                && !trimmed.starts_with(key)
                || trimmed.starts_with('#')
        })
        .map(|l| l.to_string())
        .collect();

    lines.push(line);
    lines.sort();

    let header = "# Managed by Ferrix - Security Hardening\n# Do not edit manually; changes are overwritten on apply/restore.\n";
    let content = format!("{}{}\n", header, lines.join("\n"));

    // Write via tee (reuses the existing pkexec auth cache)
    let _ = Command::new("pkexec")
        .args(["tee", conf_path])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                stdin.write_all(content.as_bytes())?;
            }
            child.wait()
        });
}

fn remove_persisted_sysctl(key: &str) {
    let conf_path = "/etc/sysctl.d/99-ferrix-hardening.conf";

    let existing = Command::new("cat")
        .arg(conf_path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    if existing.is_empty() {
        return;
    }

    let lines: Vec<&str> = existing
        .lines()
        .filter(|l| {
            let trimmed = l.trim();
            trimmed.starts_with('#') || (!trimmed.starts_with(key) && !trimmed.is_empty())
        })
        .collect();

    // If only header comments remain, delete the file
    let has_settings = lines.iter().any(|l| !l.trim().starts_with('#') && !l.trim().is_empty());
    if !has_settings {
        let _ = Command::new("pkexec")
            .args(["rm", "-f", conf_path])
            .output();
        return;
    }

    let content = format!("{}\n", lines.join("\n"));
    let _ = Command::new("pkexec")
        .args(["tee", conf_path])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                stdin.write_all(content.as_bytes())?;
            }
            child.wait()
        });
}

struct HardeningDef {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    category: &'static str,
    sysctl_key: &'static str,
    recommended: &'static str,
    default: &'static str,
}

const HARDENING_DEFS: &[HardeningDef] = &[
    // === Network Hardening ===
    HardeningDef {
        id: "icmp_redirects_ipv4",
        name: "Disable ICMP Redirects (IPv4)",
        description: "Prevents attackers from poisoning routing tables via ICMP redirect messages (CIS 3.3.2).",
        category: "Network",
        sysctl_key: "net.ipv4.conf.all.accept_redirects",
        recommended: "0",
        default: "1",
    },
    HardeningDef {
        id: "icmp_redirects_ipv6",
        name: "Disable ICMP Redirects (IPv6)",
        description: "Same protection as IPv4 redirect blocking, applied to IPv6 interfaces (CIS 3.3.2).",
        category: "Network",
        sysctl_key: "net.ipv6.conf.all.accept_redirects",
        recommended: "0",
        default: "1",
    },
    HardeningDef {
        id: "source_route_ipv4",
        name: "Disable Source Routing (IPv4)",
        description: "Blocks packets with pre-defined routes, preventing IP spoofing attacks (CIS 3.3.1).",
        category: "Network",
        sysctl_key: "net.ipv4.conf.all.accept_source_route",
        recommended: "0",
        default: "0",
    },
    HardeningDef {
        id: "tcp_syncookies",
        name: "Enable SYN Cookies",
        description: "Protects against SYN flood denial-of-service attacks by using cryptographic cookies (CIS 3.3.8).",
        category: "Network",
        sysctl_key: "net.ipv4.tcp_syncookies",
        recommended: "1",
        default: "1",
    },
    HardeningDef {
        id: "icmp_ignore_broadcasts",
        name: "Ignore ICMP Broadcasts",
        description: "Prevents Smurf amplification attacks by ignoring broadcast pings (CIS 3.3.4).",
        category: "Network",
        sysctl_key: "net.ipv4.icmp_echo_ignore_broadcasts",
        recommended: "1",
        default: "1",
    },
    HardeningDef {
        id: "rp_filter",
        name: "Reverse Path Filtering",
        description: "Drops incoming packets with source addresses that don't match the routing table, blocking spoofed traffic (CIS 3.3.7).",
        category: "Network",
        sysctl_key: "net.ipv4.conf.all.rp_filter",
        recommended: "1",
        default: "0",
    },
    HardeningDef {
        id: "log_martians",
        name: "Log Martian Packets",
        description: "Logs packets with impossible source addresses for security auditing (CIS 3.3.4).",
        category: "Network",
        sysctl_key: "net.ipv4.conf.all.log_martians",
        recommended: "1",
        default: "0",
    },
    // === Kernel Hardening ===
    HardeningDef {
        id: "kptr_restrict",
        name: "Hide Kernel Pointers",
        description: "Hides kernel memory addresses from unprivileged users, making exploits harder to write (KSPP).",
        category: "Kernel",
        sysctl_key: "kernel.kptr_restrict",
        recommended: "2",
        default: "0",
    },
    HardeningDef {
        id: "dmesg_restrict",
        name: "Restrict dmesg Access",
        description: "Only root can read kernel log messages, preventing information leakage to unprivileged users (CIS 1.5.2).",
        category: "Kernel",
        sysctl_key: "kernel.dmesg_restrict",
        recommended: "1",
        default: "0",
    },
    HardeningDef {
        id: "ptrace_scope",
        name: "Restrict Process Debugging",
        description: "Only allows parent processes to debug children. Debuggers like GDB still work normally (CIS 1.5.4, YAMA LSM).",
        category: "Kernel",
        sysctl_key: "kernel.yama.ptrace_scope",
        recommended: "1",
        default: "0",
    },
    HardeningDef {
        id: "sysrq",
        name: "Restrict SysRq Keys",
        description: "Limits Magic SysRq to safe operations (sync + reboot) only. Blocks dangerous keys like memory dumps (KSPP).",
        category: "Kernel",
        sysctl_key: "kernel.sysrq",
        recommended: "176",
        default: "438",
    },
    HardeningDef {
        id: "randomize_va_space",
        name: "Full ASLR (Address Randomization)",
        description: "Enables full address space layout randomization for stack, heap, and mmap, making exploits unreliable (CIS 1.5.3).",
        category: "Kernel",
        sysctl_key: "kernel.randomize_va_space",
        recommended: "2",
        default: "2",
    },
    HardeningDef {
        id: "perf_event_paranoid",
        name: "Restrict Performance Events",
        description: "Blocks unprivileged access to CPU performance counters, preventing side-channel attacks (KSPP).",
        category: "Kernel",
        sysctl_key: "kernel.perf_event_paranoid",
        recommended: "3",
        default: "2",
    },
    HardeningDef {
        id: "unprivileged_bpf",
        name: "Disable Unprivileged BPF",
        description: "Prevents unprivileged users from loading BPF programs, closing a common privilege escalation vector (KSPP).",
        category: "Kernel",
        sysctl_key: "kernel.unprivileged_bpf_disabled",
        recommended: "1",
        default: "0",
    },
    HardeningDef {
        id: "bpf_jit_harden",
        name: "Harden BPF JIT Compiler",
        description: "Applies constant blinding and other mitigations to BPF JIT output, preventing JIT spraying attacks (KSPP).",
        category: "Kernel",
        sysctl_key: "net.core.bpf_jit_harden",
        recommended: "2",
        default: "0",
    },
    // === Filesystem Hardening ===
    HardeningDef {
        id: "protected_hardlinks",
        name: "Protect Hardlinks",
        description: "Prevents unprivileged users from creating hardlinks to files they don't own, blocking privilege escalation (CIS 1.6.1).",
        category: "Filesystem",
        sysctl_key: "fs.protected_hardlinks",
        recommended: "1",
        default: "1",
    },
    HardeningDef {
        id: "protected_symlinks",
        name: "Protect Symlinks",
        description: "Prevents symlink-following attacks in world-writable sticky directories like /tmp (CIS 1.6.1).",
        category: "Filesystem",
        sysctl_key: "fs.protected_symlinks",
        recommended: "1",
        default: "1",
    },
    HardeningDef {
        id: "protected_fifos",
        name: "Protect FIFOs",
        description: "Restricts FIFO creation in sticky directories, preventing data interception attacks (CIS 1.6.1).",
        category: "Filesystem",
        sysctl_key: "fs.protected_fifos",
        recommended: "2",
        default: "0",
    },
    HardeningDef {
        id: "suid_dumpable",
        name: "Restrict Core Dumps (SUID)",
        description: "Prevents SUID programs from creating core dumps that could leak sensitive memory contents (CIS 1.5.1).",
        category: "Filesystem",
        sysctl_key: "fs.suid_dumpable",
        recommended: "0",
        default: "2",
    },
];

pub fn get_hardening_status() -> Vec<HardeningStatus> {
    HARDENING_DEFS
        .iter()
        .filter_map(|def| {
            let current = read_sysctl(def.sysctl_key);
            if current.is_empty() {
                return None; // sysctl not available on this kernel
            }
            Some(HardeningStatus {
                id: def.id.to_string(),
                name: def.name.to_string(),
                description: def.description.to_string(),
                category: def.category.to_string(),
                sysctl_path: def.sysctl_key.to_string(),
                current_value: current.clone(),
                recommended_value: def.recommended.to_string(),
                default_value: def.default.to_string(),
                is_applied: current == def.recommended,
            })
        })
        .collect()
}

fn find_def(id: &str) -> Option<&'static HardeningDef> {
    HARDENING_DEFS.iter().find(|d| d.id == id)
}

pub fn apply_hardening(id: &str) -> HardeningResult {
    let result = match find_def(id) {
        Some(def) => apply_sysctl(def.sysctl_key, def.recommended),
        None => Err(format!("Unknown hardening: {}", id)),
    };

    match result {
        Ok(msg) => HardeningResult {
            id: id.to_string(),
            success: true,
            message: msg,
        },
        Err(msg) => HardeningResult {
            id: id.to_string(),
            success: false,
            message: msg,
        },
    }
}

pub fn restore_hardening(id: &str) -> HardeningResult {
    let def = match find_def(id) {
        Some(d) => d,
        None => {
            return HardeningResult {
                id: id.to_string(),
                success: false,
                message: format!("Unknown hardening: {}", id),
            }
        }
    };

    // Apply runtime restore
    let result = apply_sysctl(def.sysctl_key, def.default);

    // Remove from persistent config (restore = back to OS default, no need to persist)
    remove_persisted_sysctl(def.sysctl_key);

    match result {
        Ok(msg) => HardeningResult {
            id: id.to_string(),
            success: true,
            message: msg,
        },
        Err(msg) => HardeningResult {
            id: id.to_string(),
            success: false,
            message: msg,
        },
    }
}
