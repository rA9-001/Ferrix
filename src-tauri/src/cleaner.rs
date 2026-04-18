use crate::distro::DistroInfo;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct CleanupCategory {
    pub name: String,
    pub description: String,
    pub items: Vec<CleanupItem>,
    pub total_size: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CleanupItem {
    pub path: String,
    pub size: u64,
    pub item_type: String, // "file" or "directory"
}

#[derive(Debug, Clone, Serialize)]
pub struct CleanupResult {
    pub category: String,
    pub freed_bytes: u64,
    pub removed_count: u32,
    pub errors: Vec<String>,
    pub log: Vec<CleanupLogEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CleanupLogEntry {
    pub path: String,
    pub size: u64,
    pub status: String, // "removed" or "error"
    pub message: String,
}

/// Scan all cleanup categories and return what can be cleaned.
pub fn scan_cleanup_targets(distro: &DistroInfo) -> Vec<CleanupCategory> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home"));
    let mut categories = Vec::new();

    // 1. User cache
    let cache_dir = home.join(".cache");
    if cache_dir.exists() {
        categories.push(scan_directory(
            "User Cache",
            "Cached data from applications (~/.cache)",
            &cache_dir,
        ));
    }

    // 2. Trash
    let trash_dir = home.join(".local/share/Trash");
    if trash_dir.exists() {
        categories.push(scan_directory(
            "Trash",
            "Files in your trash bin",
            &trash_dir,
        ));
    }

    // 3. Thumbnail cache
    let thumb_dir = home.join(".cache/thumbnails");
    if thumb_dir.exists() {
        categories.push(scan_directory(
            "Thumbnails",
            "Cached image thumbnails",
            &thumb_dir,
        ));
    }

    // 4. Package manager cache (requires root for full cleanup, but we can scan)
    let pkg_cache = match distro.package_manager.as_str() {
        "pacman" => Some(("/var/cache/pacman/pkg", "Pacman package cache")),
        "apt" => Some(("/var/cache/apt/archives", "APT package cache")),
        "dnf" => Some(("/var/cache/dnf", "DNF package cache")),
        "zypper" => Some(("/var/cache/zypp/packages", "Zypper package cache")),
        _ => None,
    };
    if let Some((path, desc)) = pkg_cache {
        let p = Path::new(path);
        if p.exists() {
            categories.push(scan_directory("Package Cache", desc, p));
        }
    }

    // 5. Old log files
    let log_dir = Path::new("/var/log");
    if log_dir.exists() {
        categories.push(scan_log_files(log_dir));
    }

    // 6. Systemd journal logs
    let journal_dir = Path::new("/var/log/journal");
    if journal_dir.exists() {
        categories.push(scan_directory(
            "Journal Logs",
            "Systemd journal logs — cleanup reduces to 50 MB",
            journal_dir,
        ));
    }

    // 7. Recent documents / recently-used
    let recent_file = home.join(".local/share/recently-used.xbel");
    if recent_file.exists() {
        if let Ok(meta) = recent_file.metadata() {
            categories.push(CleanupCategory {
                name: "Recent History".into(),
                description: "Recently used files history".into(),
                total_size: meta.len(),
                items: vec![CleanupItem {
                    path: recent_file.to_string_lossy().into(),
                    size: meta.len(),
                    item_type: "file".into(),
                }],
            });
        }
    }

    categories
}

fn scan_directory(name: &str, description: &str, dir: &Path) -> CleanupCategory {
    let mut items = Vec::new();
    let mut total_size: u64 = 0;

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let size = dir_size(&path);
            let item_type = if path.is_dir() { "directory" } else { "file" };

            total_size += size;
            items.push(CleanupItem {
                path: path.to_string_lossy().into(),
                size,
                item_type: item_type.into(),
            });
        }
    }

    // Sort by size descending so the UI shows biggest items first
    items.sort_by(|a, b| b.size.cmp(&a.size));

    CleanupCategory {
        name: name.into(),
        description: description.into(),
        items,
        total_size,
    }
}

fn scan_log_files(dir: &Path) -> CleanupCategory {
    let mut items = Vec::new();
    let mut total_size: u64 = 0;

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            // Only target rotated/compressed log files, not active ones
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name.ends_with(".gz")
                || name.ends_with(".old")
                || name.ends_with(".1")
                || name.ends_with(".2")
                || name.ends_with(".xz")
            {
                let size = dir_size(&path);
                total_size += size;
                items.push(CleanupItem {
                    path: path.to_string_lossy().into(),
                    size,
                    item_type: "file".into(),
                });
            }
        }
    }

    items.sort_by(|a, b| b.size.cmp(&a.size));

    CleanupCategory {
        name: "Old Log Files".into(),
        description: "Rotated and compressed log files in /var/log".into(),
        items,
        total_size,
    }
}

/// Recursively calculate the size of a path. Does NOT follow symlinks: a
/// symlink is treated as a 0-byte entry, so a malicious symlink to `/` cannot
/// cause runaway traversal.
fn dir_size(path: &Path) -> u64 {
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return 0,
    };
    if meta.file_type().is_symlink() {
        return 0;
    }
    if meta.is_file() {
        return meta.len();
    }

    let mut size = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            let m = match fs::symlink_metadata(&p) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let ft = m.file_type();
            if ft.is_symlink() {
                continue;
            }
            if ft.is_file() {
                size += m.len();
            } else if ft.is_dir() {
                size += dir_size(&p);
            }
        }
    }
    size
}

/// Execute cleanup for selected categories.
/// Root-required operations (package cache, old logs, journal) are batched
/// into a single pkexec call so the user only enters their password once.
pub fn execute_cleanup(
    categories: &[String],
    distro: &DistroInfo,
) -> Vec<CleanupResult> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home"));
    let mut results = Vec::new();

    // ── Phase 1: user-owned directories (no pkexec needed) ──
    for category in categories {
        match category.as_str() {
            "User Cache" => results.push(clean_user_directory(&home.join(".cache"), category)),
            "Trash" => results.push(clean_user_directory(&home.join(".local/share/Trash"), category)),
            "Thumbnails" => results.push(clean_user_directory(&home.join(".cache/thumbnails"), category)),
            "Recent History" => {
                let path = home.join(".local/share/recently-used.xbel");
                let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                match fs::remove_file(&path) {
                    Ok(_) => results.push(CleanupResult {
                        category: category.clone(),
                        freed_bytes: size,
                        removed_count: 1,
                        errors: vec![],
                        log: vec![CleanupLogEntry {
                            path: path.to_string_lossy().into(),
                            size,
                            status: "removed".into(),
                            message: "Removed successfully".into(),
                        }],
                    }),
                    Err(e) => results.push(CleanupResult {
                        category: category.clone(),
                        freed_bytes: 0,
                        removed_count: 0,
                        errors: vec![e.to_string()],
                        log: vec![CleanupLogEntry {
                            path: path.to_string_lossy().into(),
                            size,
                            status: "error".into(),
                            message: e.to_string(),
                        }],
                    }),
                }
            }
            _ => {} // root categories handled below
        }
    }

    // ── Phase 2: root-required operations batched into ONE pkexec ──
    let needs_pkg_cache = categories.iter().any(|c| c == "Package Cache");
    let needs_old_logs = categories.iter().any(|c| c == "Old Log Files");
    let needs_journal = categories.iter().any(|c| c == "Journal Logs");

    if needs_pkg_cache || needs_old_logs || needs_journal {
        let root_results = run_batched_root_cleanup(
            distro,
            needs_pkg_cache,
            needs_old_logs,
            needs_journal,
        );
        results.extend(root_results);
    }

    results
}

/// Clean a user-owned directory (no root needed).
/// Uses rm -rf via Command for stubborn files the running user still owns.
fn clean_user_directory(dir: &Path, category: &str) -> CleanupResult {
    let mut freed: u64 = 0;
    let mut removed: u32 = 0;
    let mut errors = Vec::new();
    let mut log = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            // Use symlink_metadata to avoid following symlinks (TOCTOU defense:
            // never recurse via remove_dir_all into a directory reached through
            // a symlink that an attacker may have swapped in).
            let meta = match fs::symlink_metadata(&path) {
                Ok(m) => m,
                Err(e) => {
                    errors.push(e.to_string());
                    continue;
                }
            };
            let file_type = meta.file_type();
            let size = if file_type.is_symlink() { 0 } else { dir_size(&path) };

            let result = if file_type.is_symlink() || file_type.is_file() {
                fs::remove_file(&path)
            } else if file_type.is_dir() {
                fs::remove_dir_all(&path)
            } else {
                // sockets, fifos, devices — skip
                continue;
            };

            match result {
                Ok(_) => {
                    freed += size;
                    removed += 1;
                    log.push(CleanupLogEntry {
                        path: path.to_string_lossy().into(),
                        size,
                        status: "removed".into(),
                        message: "Removed successfully".into(),
                    });
                }
                Err(e) => {
                    let msg = e.to_string();
                    errors.push(format!("{}: {}", path.display(), e));
                    log.push(CleanupLogEntry {
                        path: path.to_string_lossy().into(),
                        size,
                        status: "error".into(),
                        message: msg,
                    });
                }
            }
        }
    }

    CleanupResult {
        category: category.into(),
        freed_bytes: freed,
        removed_count: removed,
        errors,
        log,
    }
}

/// Build a single bash script that performs all root cleanup tasks,
/// separated by markers so we can parse per-category output.
fn run_batched_root_cleanup(
    distro: &DistroInfo,
    pkg_cache: bool,
    old_logs: bool,
    journal: bool,
) -> Vec<CleanupResult> {
    let mut script_parts: Vec<String> = Vec::new();
    let sep = "__CLEANUP_SEP__";

    // ── Measure sizes BEFORE cleanup ──
    let pkg_path = match distro.package_manager.as_str() {
        "pacman" => "/var/cache/pacman/pkg",
        "apt" => "/var/cache/apt/archives",
        "dnf" => "/var/cache/dnf",
        "zypper" => "/var/cache/zypp/packages",
        _ => "",
    };
    let size_before_pkg = if pkg_cache && !pkg_path.is_empty() {
        dir_size(Path::new(pkg_path))
    } else {
        0
    };
    let size_before_logs = if old_logs {
        scan_log_files(Path::new("/var/log")).total_size
    } else {
        0
    };
    let size_before_journal = if journal {
        dir_size(Path::new("/var/log/journal"))
    } else {
        0
    };

    // -- Package cache --
    if pkg_cache {
        let cmd = match distro.package_manager.as_str() {
            "pacman" => {
                // paccache -rk0 removes ALL cached package files;
                // fallback to find -delete if pacman-contrib is not installed
                format!(
                    "if command -v paccache >/dev/null 2>&1; then paccache -rk0 2>&1; else find /var/cache/pacman/pkg -name '*.pkg.tar.*' -delete -print 2>&1; fi"
                )
            }
            "apt" => "apt-get clean 2>&1".into(),
            "dnf" => "dnf clean all 2>&1".into(),
            "zypper" => "zypper clean --all 2>&1".into(),
            _ => "echo 'Unsupported package manager'".into(),
        };
        script_parts.push(format!("echo '{}PKG_START'", sep));
        script_parts.push(cmd);
        script_parts.push(format!("echo '{}PKG_RC='$?", sep));
    }

    // -- Old log files --
    if old_logs {
        script_parts.push(format!("echo '{}LOGS_START'", sep));
        // Remove rotated logs in one go and report what was deleted
        script_parts.push(
            "find /var/log -maxdepth 1 \\( -name '*.gz' -o -name '*.old' -o -name '*.1' -o -name '*.2' -o -name '*.xz' \\) -print -delete 2>&1".into()
        );
        script_parts.push(format!("echo '{}LOGS_RC='$?", sep));
    }

    // -- Journal vacuum --
    if journal {
        script_parts.push(format!("echo '{}JOURNAL_START'", sep));
        script_parts.push("journalctl --vacuum-size=50M 2>&1".into());
        script_parts.push(format!("echo '{}JOURNAL_RC='$?", sep));
    }

    let script = script_parts.join("\n");

    let output = match Command::new("pkexec")
        .args(["bash", "-c", &script])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            let msg = format!("Failed to run pkexec: {}", e);
            let mut r = Vec::new();
            if pkg_cache {
                r.push(make_error_result("Package Cache", &msg));
            }
            if old_logs {
                r.push(make_error_result("Old Log Files", &msg));
            }
            if journal {
                r.push(make_error_result("Journal Logs", &msg));
            }
            return r;
        }
    };

    let full_output = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // ── Measure sizes AFTER cleanup ──
    let size_after_pkg = if pkg_cache && !pkg_path.is_empty() {
        dir_size(Path::new(pkg_path))
    } else {
        0
    };
    let size_after_logs = if old_logs {
        scan_log_files(Path::new("/var/log")).total_size
    } else {
        0
    };
    let size_after_journal = if journal {
        dir_size(Path::new("/var/log/journal"))
    } else {
        0
    };

    let mut results = Vec::new();

    if pkg_cache {
        let freed = size_before_pkg.saturating_sub(size_after_pkg);
        results.push(parse_section_result(
            &full_output,
            &format!("{}PKG_START", sep),
            &format!("{}PKG_RC=", sep),
            "Package Cache",
            "Package cache cleaned",
            pkg_path,
            freed,
        ));
    }
    if old_logs {
        let freed = size_before_logs.saturating_sub(size_after_logs);
        results.push(parse_section_result(
            &full_output,
            &format!("{}LOGS_START", sep),
            &format!("{}LOGS_RC=", sep),
            "Old Log Files",
            "Old log files removed",
            "/var/log",
            freed,
        ));
    }
    if journal {
        let freed = size_before_journal.saturating_sub(size_after_journal);
        results.push(parse_section_result(
            &full_output,
            &format!("{}JOURNAL_START", sep),
            &format!("{}JOURNAL_RC=", sep),
            "Journal Logs",
            "Journal vacuumed (50 MB retention)",
            "/var/log/journal",
            freed,
        ));
    }

    results
}

fn parse_section_result(
    full: &str,
    start_marker: &str,
    rc_marker: &str,
    category: &str,
    default_msg: &str,
    path: &str,
    freed_bytes: u64,
) -> CleanupResult {
    // Extract text between start marker and RC marker
    let body = if let Some(start_pos) = full.find(start_marker) {
        let after_start = &full[start_pos + start_marker.len()..];
        if let Some(rc_pos) = after_start.find(rc_marker) {
            after_start[..rc_pos].trim().to_string()
        } else {
            after_start.trim().to_string()
        }
    } else {
        String::new()
    };

    // Extract return code
    let rc = if let Some(rc_pos) = full.find(rc_marker) {
        let after_rc = &full[rc_pos + rc_marker.len()..];
        after_rc.lines().next().unwrap_or("1").trim().parse::<i32>().unwrap_or(1)
    } else {
        1
    };

    let message = if body.is_empty() { default_msg.to_string() } else { body.clone() };

    if rc == 0 {
        // Count lines that look like deleted file paths (from find -print -delete)
        let removed_count = if category == "Old Log Files" {
            body.lines().filter(|l| l.starts_with("/var/log/")).count() as u32
        } else {
            1
        };

        CleanupResult {
            category: category.into(),
            freed_bytes,
            removed_count,
            errors: vec![],
            log: vec![CleanupLogEntry {
                path: path.into(),
                size: freed_bytes,
                status: "removed".into(),
                message,
            }],
        }
    } else {
        CleanupResult {
            category: category.into(),
            freed_bytes: 0,
            removed_count: 0,
            errors: vec![message.clone()],
            log: vec![CleanupLogEntry {
                path: path.into(),
                size: 0,
                status: "error".into(),
                message,
            }],
        }
    }
}

fn make_error_result(category: &str, msg: &str) -> CleanupResult {
    CleanupResult {
        category: category.into(),
        freed_bytes: 0,
        removed_count: 0,
        errors: vec![msg.into()],
        log: vec![CleanupLogEntry {
            path: String::new(),
            size: 0,
            status: "error".into(),
            message: msg.into(),
        }],
    }
}
