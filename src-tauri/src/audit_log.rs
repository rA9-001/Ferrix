//! Append-only audit log + per-command rate limiting for privileged operations.
//!
//! - Audit log lives at `$XDG_STATE_HOME/linux-utility-app/audit.log`
//!   (falling back to `~/.local/state/linux-utility-app/audit.log`).
//! - Rate limiting uses a process-wide `Mutex<HashMap<String, Instant>>`
//!   to throttle each named command to one invocation per `min_interval`.
//!
//! Both helpers are best-effort: a failure to write the audit log MUST NOT
//! prevent the privileged operation from completing (the user already
//! authenticated via polkit). Failures are logged to stderr.

use std::collections::HashMap;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static RATE_LIMITER: Mutex<Option<HashMap<String, Instant>>> = Mutex::new(None);

fn audit_log_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|h| PathBuf::from(h).join(".local").join("state"))
        })?;
    let dir = base.join("linux-utility-app");
    if let Err(e) = create_dir_all(&dir) {
        eprintln!("audit_log: failed to create {:?}: {}", dir, e);
        return None;
    }
    Some(dir.join("audit.log"))
}

/// Append a single audit record. `action` should be a short, stable identifier
/// (e.g. `"install_packages"`). `detail` may contain user-supplied data — it is
/// sanitized to strip newlines / control chars before being written.
pub fn record(action: &str, detail: &str) {
    let Some(path) = audit_log_path() else { return };

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let safe_detail: String = detail
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(512)
        .collect();
    let safe_action: String = action
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(64)
        .collect();

    let line = format!("{}\t{}\t{}\n", ts, safe_action, safe_detail);

    match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(mut f) => {
            if let Err(e) = f.write_all(line.as_bytes()) {
                eprintln!("audit_log: write failed: {}", e);
            }
        }
        Err(e) => eprintln!("audit_log: open {:?} failed: {}", path, e),
    }
}

/// Return `Err(message)` if `command` was invoked within the last `min_interval`.
/// Otherwise record the new invocation timestamp and return `Ok(())`.
pub fn check_rate_limit(command: &str, min_interval: Duration) -> Result<(), String> {
    let mut guard = match RATE_LIMITER.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(), // recover from poisoned mutex
    };
    let map = guard.get_or_insert_with(HashMap::new);
    let now = Instant::now();
    if let Some(prev) = map.get(command) {
        if let Some(remaining) = min_interval.checked_sub(now.duration_since(*prev)) {
            if !remaining.is_zero() {
                return Err(format!(
                    "Rate limited: please wait {}ms before retrying '{}'",
                    remaining.as_millis(),
                    command
                ));
            }
        }
    }
    map.insert(command.to_string(), now);
    Ok(())
}
