use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::process::Stdio;

use crate::sysenv::system_command;

fn strip_ansi_codes(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            i += 2;
            while i < bytes.len() && !(bytes[i] >= b'@' && bytes[i] <= b'~') {
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    result
}

fn is_running_as_root() -> bool {
    system_command("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false)
}

/// Strict POSIX-portable username allow-list (IEEE Std 1003.1-2017, 3.437):
/// `[A-Za-z_][A-Za-z0-9._-]{0,31}`. Rejects everything else, defending against
/// shell injection / path traversal if the value is later interpolated.
fn is_valid_username(u: &str) -> bool {
    let bytes = u.as_bytes();
    if bytes.is_empty() || bytes.len() > 32 {
        return false;
    }
    let first = bytes[0];
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    bytes
        .iter()
        .all(|&b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-')
}

fn is_valid_uid(s: &str) -> bool {
    !s.is_empty() && s.len() <= 10 && s.bytes().all(|b| b.is_ascii_digit())
}

fn get_real_user() -> Option<String> {
    if let Ok(user) = std::env::var("SUDO_USER") {
        if is_valid_username(&user) && user != "root" {
            return Some(user);
        }
    }
    if let Ok(uid) = std::env::var("PKEXEC_UID") {
        if is_valid_uid(&uid) {
            if let Ok(out) = system_command("id").args(["-nu", "--", &uid]).output() {
                let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if is_valid_username(&name) && name != "root" {
                    return Some(name);
                }
            }
        }
    }
    if let Ok(user) = std::env::var("DOAS_USER") {
        if is_valid_username(&user) && user != "root" {
            return Some(user);
        }
    }
    if let Ok(out) = system_command("logname").output() {
        let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if is_valid_username(&name) && name != "root" {
            return Some(name);
        }
    }
    None
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub name: String,
    pub current_version: String,
    pub new_version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateCheckResult {
    pub updates: Vec<UpdateInfo>,
    pub flatpak_updates: Vec<String>,
    pub summary: String,
}

/// Replicate `checkupdates` from pacman-contrib without requiring root or
/// touching the system sync DB:
///   1. Create a private temp dbpath that mirrors `/var/lib/pacman` layout.
///   2. Hardlink-or-copy the existing sync DB into it (so we don't re-download
///      everything if the system DB is fresh).
///   3. `pacman -Sy --dbpath <tmp>` to refresh it (no root needed for a
///      user-owned dbpath; pacman only needs to write inside the temp dir).
///   4. `pacman -Qu --dbpath <tmp>` to list upgradable packages.
///
/// Returns the raw stdout of `pacman -Qu` (lines like
/// `name oldver -> newver`) on success, or `None` on any failure.
fn pacman_check_updates_via_temp_db() -> Option<String> {
    use std::fs;
    use std::path::PathBuf;

    let tmp_root = std::env::temp_dir().join(format!("lar-checkupdates-{}", std::process::id()));
    let sync_dst = tmp_root.join("sync");
    let local_link = tmp_root.join("local");

    let _ = fs::remove_dir_all(&tmp_root);
    fs::create_dir_all(&sync_dst).ok()?;

    // Mirror existing sync DB into temp (cheap; files are small).
    let sys_sync = PathBuf::from("/var/lib/pacman/sync");
    if let Ok(entries) = fs::read_dir(&sys_sync) {
        for entry in entries.flatten() {
            let src = entry.path();
            let dst = sync_dst.join(entry.file_name());
            if fs::hard_link(&src, &dst).is_err() {
                let _ = fs::copy(&src, &dst);
            }
        }
    }

    // pacman needs a `local` dir; symlink to the real one (read-only use).
    let _ = std::os::unix::fs::symlink("/var/lib/pacman/local", &local_link);

    // Refresh the temp sync DB (downloads only diffs, no root needed).
    let sync_status = system_command("pacman")
        .args(["-Sy", "--dbpath"])
        .arg(&tmp_root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;
    if !sync_status.success() {
        let _ = fs::remove_dir_all(&tmp_root);
        return None;
    }

    // pacman -Qu exits 1 when there are no updates; that's still success here.
    let qu = system_command("pacman")
        .args(["-Qu", "--dbpath"])
        .arg(&tmp_root)
        .output()
        .ok()?;

    let _ = fs::remove_dir_all(&tmp_root);
    Some(String::from_utf8_lossy(&qu.stdout).into_owned())
}

pub fn check_updates(package_manager: &str) -> UpdateCheckResult {
    let mut updates = Vec::new();
    let mut flatpak_updates = Vec::new();

    match package_manager {
        "pacman" => {
            // Check official repo updates. Prefer `checkupdates` from
            // pacman-contrib (no root needed, syncs to a temp DB).
            // If unavailable, replicate it ourselves so we never silently
            // return stale results from a stale local sync DB.
            let stdout_str = match system_command("checkupdates").output() {
                Ok(out) if out.status.success() || !out.stdout.is_empty() => {
                    String::from_utf8_lossy(&out.stdout).into_owned()
                }
                _ => pacman_check_updates_via_temp_db().unwrap_or_default(),
            };
            for line in stdout_str.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    updates.push(UpdateInfo {
                        name: parts[0].to_string(),
                        current_version: parts[1].to_string(),
                        new_version: parts[3].to_string(),
                    });
                }
            }

            // Check AUR updates via yay or paru if available
            let aur_helper = if system_command("yay").arg("--version").output().is_ok() {
                Some("yay")
            } else if system_command("paru").arg("--version").output().is_ok() {
                Some("paru")
            } else {
                None
            };
            if let Some(helper) = aur_helper {
                let aur_out = if is_running_as_root() {
                    if let Some(real_user) = get_real_user() {
                        system_command("sudo")
                            .args(["-u", &real_user, helper, "-Qua", "--color=never"])
                            .output()
                    } else {
                        system_command(helper).args(["-Qua", "--color=never"]).output()
                    }
                } else {
                    system_command(helper).args(["-Qua", "--color=never"]).output()
                };
                if let Ok(out) = aur_out {
                    let stdout = strip_ansi_codes(&String::from_utf8_lossy(&out.stdout));
                    for line in stdout.lines() {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 4 {
                            let name = parts[0].to_string();
                            // Avoid duplicates from official repo check
                            if !updates.iter().any(|u| u.name == name) {
                                updates.push(UpdateInfo {
                                    name,
                                    current_version: parts[1].to_string(),
                                    new_version: parts[3].to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
        "apt" => {
            // Use cached package data - apt update will run during apply_updates
            if let Ok(out) = system_command("apt")
                .args(["list", "--upgradable"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    if line.contains("[upgradable from:") {
                        if let Some(name) = line.split('/').next() {
                            let new_ver = line
                                .split_whitespace()
                                .nth(1)
                                .unwrap_or("?")
                                .to_string();
                            let old_ver = line
                                .rsplit("from: ")
                                .next()
                                .and_then(|s| s.strip_suffix(']'))
                                .unwrap_or("?")
                                .to_string();
                            updates.push(UpdateInfo {
                                name: name.to_string(),
                                current_version: old_ver,
                                new_version: new_ver,
                            });
                        }
                    }
                }
            }
        }
        "dnf" => {
            if let Ok(out) = system_command("dnf")
                .args(["check-update", "--quiet"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 && !line.starts_with("Last metadata") {
                        updates.push(UpdateInfo {
                            name: parts[0].to_string(),
                            current_version: String::new(),
                            new_version: parts[1].to_string(),
                        });
                    }
                }
            }
        }
        "zypper" => {
            if let Ok(out) = system_command("zypper")
                .args(["list-updates"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    if line.starts_with("v |") || line.starts_with("  |") {
                        let cols: Vec<&str> = line.split('|').collect();
                        if cols.len() >= 5 {
                            updates.push(UpdateInfo {
                                name: cols[2].trim().to_string(),
                                current_version: cols[3].trim().to_string(),
                                new_version: cols[4].trim().to_string(),
                            });
                        }
                    }
                }
            }
        }
        _ => {}
    }

    // Check flatpak updates
    if let Ok(out) = system_command("flatpak")
        .args(["remote-ls", "--updates", "--columns=application"])
        .output()
    {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && trimmed.contains('.') {
                    flatpak_updates.push(trimmed.to_string());
                }
            }
        }
    }

    let total = updates.len() + flatpak_updates.len();
    let summary = if total == 0 {
        "System is up to date".to_string()
    } else {
        let mut parts = Vec::new();
        if !updates.is_empty() {
            parts.push(format!("{} system package{}", updates.len(), if updates.len() == 1 { "" } else { "s" }));
        }
        if !flatpak_updates.is_empty() {
            parts.push(format!("{} flatpak{}", flatpak_updates.len(), if flatpak_updates.len() == 1 { "" } else { "s" }));
        }
        format!("{} update{} available: {}", total, if total == 1 { "" } else { "s" }, parts.join(", "))
    };

    UpdateCheckResult {
        updates,
        flatpak_updates,
        summary,
    }
}

pub fn apply_updates<F: Fn(&str)>(emit_line: &F, package_manager: &str, update_flatpak: bool) {
    // Use AUR helper for full system update if available (covers official + AUR)
    let aur_helper = if package_manager == "pacman" {
        if system_command("yay").arg("--version").output().is_ok() {
            Some("yay")
        } else if system_command("paru").arg("--version").output().is_ok() {
            Some("paru")
        } else {
            None
        }
    } else {
        None
    };

    let pm_args: Vec<String> = match package_manager {
        "pacman" => vec!["pacman", "-Syu", "--noconfirm"],
        "apt" => vec!["apt-get", "dist-upgrade", "-y"],
        "dnf" => vec!["dnf", "upgrade", "-y"],
        "zypper" => vec!["zypper", "update", "-y"],
        "xbps" => vec!["xbps-install", "-Su", "-y"],
        "apk" => vec!["apk", "upgrade"],
        _ => vec![],
    }
    .into_iter()
    .map(String::from)
    .collect();

    if !pm_args.is_empty() {
        // Always update official repos via pkexec pacman
        emit_line(&format!("$ pkexec {}", pm_args.join(" ")));
        match run_streaming(emit_line, "pkexec", &pm_args) {
            Ok((success, _)) => {
                if success {
                    emit_line("✓ System packages updated successfully");
                } else {
                    emit_line("✗ System package update finished with errors");
                }
            }
            Err(e) => emit_line(&format!("✗ Failed: {}", e)),
        }

        // Update AUR packages separately (must not run as root)
        // Use --sudo pkexec so yay uses polkit for privilege escalation
        // instead of sudo which requires a TTY for password input
        if let Some(helper) = aur_helper {
            emit_line("");
            emit_line(&format!("$ {} -Sua --noconfirm --sudo pkexec", helper));
            if is_running_as_root() {
                if let Some(real_user) = get_real_user() {
                    let args: Vec<String> = vec!["-u", &real_user, helper, "-Sua", "--noconfirm", "--sudo", "pkexec"]
                        .into_iter()
                        .map(String::from)
                        .collect();
                    match run_streaming(emit_line, "sudo", &args) {
                        Ok((success, _)) => {
                            if success {
                                emit_line("✓ AUR packages updated successfully");
                            } else {
                                emit_line("✗ AUR update finished with errors");
                            }
                        }
                        Err(e) => emit_line(&format!("✗ AUR update failed: {}", e)),
                    }
                } else {
                    emit_line("✗ Cannot update AUR packages: running as root and unable to determine regular user");
                    emit_line("  Run your AUR helper manually as a regular user");
                }
            } else {
                match run_streaming(
                    emit_line,
                    helper,
                    &["-Sua".into(), "--noconfirm".into(), "--sudo".into(), "pkexec".into()],
                ) {
                    Ok((success, _)) => {
                        if success {
                            emit_line("✓ AUR packages updated successfully");
                        } else {
                            emit_line("✗ AUR update finished with errors");
                        }
                    }
                    Err(e) => emit_line(&format!("✗ AUR update failed: {}", e)),
                }
            }
        }
    }

    if update_flatpak {
        emit_line("");
        emit_line("$ flatpak update -y");
        match run_streaming(
            emit_line,
            "flatpak",
            &["update".into(), "-y".into()],
        ) {
            Ok((success, _)) => {
                if success {
                    emit_line("✓ Flatpak apps updated successfully");
                } else {
                    emit_line("✗ Flatpak update finished with errors");
                }
            }
            Err(e) => emit_line(&format!("✗ Flatpak update failed: {}", e)),
        }
    }
}

fn run_streaming<F: Fn(&str)>(
    emit_line: &F,
    program: &str,
    args: &[String],
) -> Result<(bool, String), String> {
    let mut child = system_command(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn {}: {}", program, e))?;

    let stderr_handle = child.stderr.take().unwrap();
    let stderr_thread = std::thread::spawn(move || {
        let reader = BufReader::new(stderr_handle);
        reader.lines().flatten().collect::<Vec<_>>()
    });

    let stdout_handle = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout_handle);
    let mut output = String::new();

    for line in reader.lines().flatten() {
        emit_line(&line);
        output.push_str(&line);
        output.push('\n');
    }

    if let Ok(stderr_lines) = stderr_thread.join() {
        for line in &stderr_lines {
            emit_line(line);
            output.push_str(line);
            output.push('\n');
        }
    }

    let status = child.wait().map_err(|e| format!("Wait failed: {}", e))?;
    Ok((status.success(), output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_accepts_posix_names() {
        for ok in ["alice", "_svc", "user1", "a.b", "a_b-c", "u32"] {
            assert!(is_valid_username(ok), "should accept {ok}");
        }
    }

    #[test]
    fn username_rejects_injection_and_oversize() {
        for bad in [
            "", "1abc", "-rf", ".hidden", "a;rm", "a b", "a$b",
            "a`id`", "a/b", "a\nb", "a\0", &"a".repeat(40),
        ] {
            assert!(!is_valid_username(bad), "should reject {:?}", bad);
        }
    }

    #[test]
    fn uid_accepts_only_digits() {
        assert!(is_valid_uid("0"));
        assert!(is_valid_uid("1000"));
        for bad in ["", "-1", "1000;rm", "1000 ", "abc", "1000\n", &"9".repeat(20)] {
            assert!(!is_valid_uid(bad), "should reject {:?}", bad);
        }
    }
}
