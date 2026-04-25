use serde::{Deserialize, Serialize};
use crate::sysenv::system_command;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub world_writable: Vec<AuditEntry>,
    pub suid_binaries: Vec<AuditEntry>,
    pub sgid_binaries: Vec<AuditEntry>,
    pub home_dir_issues: Vec<AuditEntry>,
    pub summary: AuditSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub path: String,
    pub permissions: String,
    pub owner: String,
    pub group: String,
    pub size_bytes: u64,
    pub severity: String, // "critical", "warning", "info"
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSummary {
    pub world_writable_count: usize,
    pub suid_count: usize,
    pub sgid_count: usize,
    pub home_issues_count: usize,
    pub total_issues: usize,
    pub scan_paths: Vec<String>,
}

pub fn run_audit(scan_paths: Vec<String>) -> AuditReport {
    let default_paths = [
        "/usr",
        "/bin",
        "/sbin",
        "/usr/bin",
        "/usr/sbin",
        "/usr/local/bin",
        "/usr/local/sbin",
        "/tmp",
        "/var/tmp",
    ];
    let raw_paths: Vec<String> = if scan_paths.is_empty() {
        default_paths.iter().map(|s| s.to_string()).collect()
    } else {
        scan_paths
    };

    // Validate every scan path: must be an absolute, normal path that exists.
    // This prevents passing find expressions like "-delete" or shell metacharacters
    // that could change find's behavior or cause data loss.
    let paths: Vec<String> = raw_paths
        .into_iter()
        .filter(|p| is_safe_audit_path(p))
        .collect();

    let world_writable = find_world_writable(&paths);
    let suid_binaries = find_suid_binaries(&paths);
    let sgid_binaries = find_sgid_binaries(&paths);
    let home_dir_issues = check_home_dirs();

    let summary = AuditSummary {
        world_writable_count: world_writable.len(),
        suid_count: suid_binaries.len(),
        sgid_count: sgid_binaries.len(),
        home_issues_count: home_dir_issues.len(),
        total_issues: world_writable.len()
            + suid_binaries.len()
            + sgid_binaries.len()
            + home_dir_issues.len(),
        scan_paths: paths,
    };

    AuditReport {
        world_writable,
        suid_binaries,
        sgid_binaries,
        home_dir_issues,
        summary,
    }
}

fn find_world_writable(paths: &[String]) -> Vec<AuditEntry> {
    let mut results = Vec::new();
    if paths.is_empty() {
        return results;
    }

    let mut argv: Vec<&str> = Vec::with_capacity(paths.len() + 8);
    for p in paths {
        argv.push(p.as_str());
    }
    // Expressions must come AFTER paths in find; using validated paths above
    // means none can be mistaken for an expression.
    argv.extend([
        "-xdev", "-type", "f", "-perm", "-0002", "-not", "-type", "l",
    ]);

    let output = system_command("find").args(&argv).output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for path_str in stdout.lines().take(500) {
            if let Some(entry) = build_entry(path_str, "warning", "World-writable file") {
                results.push(entry);
            }
        }
    }

    results
}

fn find_suid_binaries(paths: &[String]) -> Vec<AuditEntry> {
    let mut results = Vec::new();

    // Known expected SUID binaries (not flagged as critical)
    let expected_suid: &[&str] = &[
        "/usr/bin/sudo",
        "/usr/bin/su",
        "/usr/bin/passwd",
        "/usr/bin/chsh",
        "/usr/bin/chfn",
        "/usr/bin/newgrp",
        "/usr/bin/gpasswd",
        "/usr/bin/mount",
        "/usr/bin/umount",
        "/usr/bin/pkexec",
        "/usr/lib/polkit-1/polkit-agent-helper-1",
        "/usr/bin/fusermount",
        "/usr/bin/fusermount3",
        "/usr/bin/ping",
        "/usr/bin/traceroute",
    ];

    let output = if paths.is_empty() {
        return results;
    } else {
        let mut argv: Vec<&str> = Vec::with_capacity(paths.len() + 5);
        for p in paths {
            argv.push(p.as_str());
        }
        argv.extend(["-xdev", "-type", "f", "-perm", "-4000"]);
        system_command("find").args(&argv).output()
    };

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for path_str in stdout.lines().take(500) {
            let severity = if expected_suid.iter().any(|e| *e == path_str) {
                "info"
            } else {
                "critical"
            };
            let desc = if severity == "info" {
                "SUID binary (expected system binary)"
            } else {
                "SUID binary (unexpected — review carefully)"
            };
            if let Some(entry) = build_entry(path_str, severity, desc) {
                results.push(entry);
            }
        }
    }

    // Sort critical first
    results.sort_by(|a, b| severity_order(&a.severity).cmp(&severity_order(&b.severity)));
    results
}

fn find_sgid_binaries(paths: &[String]) -> Vec<AuditEntry> {
    let mut results = Vec::new();
    if paths.is_empty() {
        return results;
    }

    let mut argv: Vec<&str> = Vec::with_capacity(paths.len() + 5);
    for p in paths {
        argv.push(p.as_str());
    }
    argv.extend(["-xdev", "-type", "f", "-perm", "-2000"]);
    let output = system_command("find").args(&argv).output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for path_str in stdout.lines().take(500) {
            if let Some(entry) = build_entry(path_str, "warning", "SGID binary") {
                results.push(entry);
            }
        }
    }

    results
}

fn check_home_dirs() -> Vec<AuditEntry> {
    let mut results = Vec::new();

    let home_base = Path::new("/home");
    if let Ok(entries) = fs::read_dir(home_base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            if let Ok(meta) = fs::metadata(&path) {
                let mode = meta.permissions().mode();
                let path_str = path.display().to_string();

                // Check if home dir is world-readable (o+r)
                if mode & 0o004 != 0 {
                    results.push(AuditEntry {
                        path: path_str.clone(),
                        permissions: format_permissions(mode),
                        owner: uid_to_name(meta.uid()),
                        group: gid_to_name(meta.gid()),
                        size_bytes: 0,
                        severity: "warning".to_string(),
                        description: "Home directory is world-readable".to_string(),
                    });
                }

                // Check if home dir is world-writable (o+w)
                if mode & 0o002 != 0 {
                    results.push(AuditEntry {
                        path: path_str.clone(),
                        permissions: format_permissions(mode),
                        owner: uid_to_name(meta.uid()),
                        group: gid_to_name(meta.gid()),
                        size_bytes: 0,
                        severity: "critical".to_string(),
                        description: "Home directory is world-writable".to_string(),
                    });
                }

                // Check .ssh permissions
                let ssh_dir = path.join(".ssh");
                if ssh_dir.exists() {
                    if let Ok(ssh_meta) = fs::metadata(&ssh_dir) {
                        let ssh_mode = ssh_meta.permissions().mode();
                        if ssh_mode & 0o077 != 0 {
                            results.push(AuditEntry {
                                path: ssh_dir.display().to_string(),
                                permissions: format_permissions(ssh_mode),
                                owner: uid_to_name(ssh_meta.uid()),
                                group: gid_to_name(ssh_meta.gid()),
                                size_bytes: 0,
                                severity: "critical".to_string(),
                                description: ".ssh directory has overly permissive permissions (should be 700)".to_string(),
                            });
                        }
                    }

                    // Check authorized_keys
                    let auth_keys = ssh_dir.join("authorized_keys");
                    if auth_keys.exists() {
                        if let Ok(ak_meta) = fs::metadata(&auth_keys) {
                            let ak_mode = ak_meta.permissions().mode();
                            if ak_mode & 0o077 != 0 {
                                results.push(AuditEntry {
                                    path: auth_keys.display().to_string(),
                                    permissions: format_permissions(ak_mode),
                                    owner: uid_to_name(ak_meta.uid()),
                                    group: gid_to_name(ak_meta.gid()),
                                    size_bytes: ak_meta.len(),
                                    severity: "critical".to_string(),
                                    description: "authorized_keys has overly permissive permissions (should be 600)".to_string(),
                                });
                            }
                        }
                    }

                    // Check private keys
                    if let Ok(ssh_files) = fs::read_dir(&ssh_dir) {
                        for f in ssh_files.flatten() {
                            let fname = f.file_name().to_string_lossy().to_string();
                            if fname.starts_with("id_") && !fname.ends_with(".pub") {
                                if let Ok(key_meta) = f.metadata() {
                                    let key_mode = key_meta.permissions().mode();
                                    if key_mode & 0o077 != 0 {
                                        results.push(AuditEntry {
                                            path: f.path().display().to_string(),
                                            permissions: format_permissions(key_mode),
                                            owner: uid_to_name(key_meta.uid()),
                                            group: gid_to_name(key_meta.gid()),
                                            size_bytes: key_meta.len(),
                                            severity: "critical".to_string(),
                                            description: "SSH private key has overly permissive permissions (should be 600)".to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    results
}

fn build_entry(path_str: &str, severity: &str, description: &str) -> Option<AuditEntry> {
    let path = Path::new(path_str);
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return None,
    };

    Some(AuditEntry {
        path: path_str.to_string(),
        permissions: format_permissions(meta.permissions().mode()),
        owner: uid_to_name(meta.uid()),
        group: gid_to_name(meta.gid()),
        size_bytes: meta.len(),
        severity: severity.to_string(),
        description: description.to_string(),
    })
}

fn format_permissions(mode: u32) -> String {
    let file_type = match mode & 0o170000 {
        0o100000 => '-',
        0o040000 => 'd',
        0o120000 => 'l',
        _ => '?',
    };

    let rwx = |shift: u32| {
        let r = if mode >> shift & 4 != 0 { 'r' } else { '-' };
        let w = if mode >> shift & 2 != 0 { 'w' } else { '-' };
        let x = if mode >> shift & 1 != 0 { 'x' } else { '-' };
        format!("{}{}{}", r, w, x)
    };

    let mut perms = format!("{}{}{}{}", file_type, rwx(6), rwx(3), rwx(0));

    // Show SUID/SGID/sticky
    let bytes = unsafe { perms.as_bytes_mut() };
    if mode & 0o4000 != 0 {
        bytes[3] = if bytes[3] == b'x' { b's' } else { b'S' };
    }
    if mode & 0o2000 != 0 {
        bytes[6] = if bytes[6] == b'x' { b's' } else { b'S' };
    }
    if mode & 0o1000 != 0 {
        bytes[9] = if bytes[9] == b'x' { b't' } else { b'T' };
    }

    perms
}

fn uid_to_name(uid: u32) -> String {
    let output = system_command("id")
        .args(["-nu", &uid.to_string()])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        _ => uid.to_string(),
    }
}

fn gid_to_name(gid: u32) -> String {
    let output = system_command("getent")
        .args(["group", &gid.to_string()])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.split(':').next().unwrap_or(&gid.to_string()).trim().to_string()
        }
        _ => gid.to_string(),
    }
}

fn severity_order(severity: &str) -> u8 {
    match severity {
        "critical" => 0,
        "warning" => 1,
        "info" => 2,
        _ => 3,
    }
}

/// Validate a path for use as a positional argument to `find`.
/// Must be absolute, cannot start with '-' (would be a find expression),
/// must not contain NUL bytes, and the path must currently exist as a directory.
fn is_safe_audit_path(p: &str) -> bool {
    if p.is_empty() || p.len() > 4096 {
        return false;
    }
    if p.starts_with('-') {
        return false;
    }
    if p.contains('\0') {
        return false;
    }
    if !p.starts_with('/') {
        return false;
    }
    Path::new(p).is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_path_rejects_unsafe() {
        // Must reject everything that isn't an absolute, real, non-flag dir path.
        for bad in [
            "", "-anything", "--checkpoint=1", "relative/path",
            "/etc/passwd\0", "/nope/does-not-exist-xyz",
            &format!("/{}", "a".repeat(5000)),
        ] {
            assert!(!is_safe_audit_path(bad), "should reject {:?}", bad);
        }
    }

    #[test]
    fn audit_path_accepts_real_dir() {
        // /tmp must exist on any sane Linux test environment.
        assert!(is_safe_audit_path("/tmp"));
    }
}
