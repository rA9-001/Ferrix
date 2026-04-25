use serde::{Deserialize, Serialize};
use crate::sysenv::system_command;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TweakStatus {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub sys_path: String,
    pub current_value: String,
    pub recommended_value: String,
    pub default_value: String,
    pub is_applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TweakResult {
    pub id: String,
    pub success: bool,
    pub message: String,
}

fn read_sys(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn run_cmd(program: &str, args: &[&str]) -> Result<String, String> {
    let output = system_command(program)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run {}: {}", program, e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

const SYSCTL_PERF_CONF: &str = "/etc/sysctl.d/99-ferrix-performance.conf";
const TMPFILES_PERF_CONF: &str = "/etc/tmpfiles.d/99-ferrix-performance.conf";

fn persist_sysctl_perf(key: &str, value: &str) {
    let line = format!("{} = {}", key, value);

    let existing = system_command("cat")
        .arg(SYSCTL_PERF_CONF)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let mut settings: Vec<String> = existing
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('#') && !t.starts_with(key)
        })
        .map(|l| l.to_string())
        .collect();

    settings.push(line);
    settings.sort();

    let header = "# Managed by Ferrix - Performance Optimizer\n\
                  # Do not edit manually; changes are overwritten on apply/restore.\n";
    let content = format!("{}{}\n", header, settings.join("\n"));

    let _ = system_command("pkexec")
        .args(["tee", SYSCTL_PERF_CONF])
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

fn remove_persisted_sysctl_perf(key: &str) {
    let existing = system_command("cat")
        .arg(SYSCTL_PERF_CONF)
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
            let t = l.trim();
            t.starts_with('#') || (!t.starts_with(key) && !t.is_empty())
        })
        .collect();

    let has_settings = lines.iter().any(|l| {
        let t = l.trim();
        !t.starts_with('#') && !t.is_empty()
    });

    if !has_settings {
        let _ = system_command("pkexec")
            .args(["rm", "-f", SYSCTL_PERF_CONF])
            .output();
        return;
    }

    let content = format!("{}\n", lines.join("\n"));
    let _ = system_command("pkexec")
        .args(["tee", SYSCTL_PERF_CONF])
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

fn persist_sysfs(path: &str, value: &str) {
    let line = format!("w {} - - - - {}", path, value);

    let existing = system_command("cat")
        .arg(TMPFILES_PERF_CONF)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let mut settings: Vec<String> = existing
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('#') && !t.contains(path)
        })
        .map(|l| l.to_string())
        .collect();

    settings.push(line);
    settings.sort();

    let header = "# Managed by Ferrix - Performance Optimizer\n\
                  # Do not edit manually; changes are overwritten on apply/restore.\n";
    let content = format!("{}{}\n", header, settings.join("\n"));

    let _ = system_command("pkexec")
        .args(["tee", TMPFILES_PERF_CONF])
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

fn remove_persisted_sysfs(path: &str) {
    let existing = system_command("cat")
        .arg(TMPFILES_PERF_CONF)
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
            let t = l.trim();
            t.starts_with('#') || (!t.contains(path) && !t.is_empty())
        })
        .collect();

    let has_settings = lines.iter().any(|l| {
        let t = l.trim();
        !t.starts_with('#') && !t.is_empty()
    });

    if !has_settings {
        let _ = system_command("pkexec")
            .args(["rm", "-f", TMPFILES_PERF_CONF])
            .output();
        return;
    }

    let content = format!("{}\n", lines.join("\n"));
    let _ = system_command("pkexec")
        .args(["tee", TMPFILES_PERF_CONF])
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

fn parse_scheduler(raw: &str) -> String {
    // Format: "none [mq-deadline] kyber" — extract the one in brackets
    if let Some(start) = raw.find('[') {
        if let Some(end) = raw.find(']') {
            return raw[start + 1..end].to_string();
        }
    }
    raw.to_string()
}

fn parse_thp(raw: &str) -> String {
    parse_scheduler(raw) // same bracket format
}

fn detect_block_device() -> Option<String> {
    // Find the first real block device (nvme or sd*)
    if let Ok(entries) = fs::read_dir("/sys/block") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if (name.starts_with("nvme") || name.starts_with("sd"))
                && name.chars().all(|c| c.is_ascii_alphanumeric())
            {
                return Some(name);
            }
        }
    }
    None
}

fn is_zram_active() -> bool {
    fs::read_to_string("/proc/swaps")
        .map(|s| s.lines().any(|l| l.contains("zram")))
        .unwrap_or(false)
}

fn detect_default_governor() -> String {
    let available = read_sys("/sys/devices/system/cpu/cpu0/cpufreq/scaling_available_governors");
    for pref in ["schedutil", "ondemand", "powersave"] {
        if available.contains(pref) {
            return pref.to_string();
        }
    }
    "schedutil".to_string()
}

pub fn get_tweaks() -> Vec<TweakStatus> {
    let mut tweaks = Vec::new();

    // 1. CPU Governor
    let gov = read_sys("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor");
    if !gov.is_empty() {
        let default_gov = detect_default_governor();
        tweaks.push(TweakStatus {
            id: "cpu_governor".into(),
            name: "CPU Governor → Performance".into(),
            description: "Sets all CPU cores to maximum frequency. Best for gaming and heavy workloads.".into(),
            category: "CPU".into(),
            sys_path: "/sys/devices/system/cpu/cpu*/cpufreq/scaling_governor".into(),
            current_value: gov.clone(),
            recommended_value: "performance".into(),
            default_value: default_gov,
            is_applied: gov == "performance",
        });
    }

    // 2. Swappiness (skip when ZRAM is active — high swappiness is correct for ZRAM)
    if !is_zram_active() {
        let swappiness = read_sys("/proc/sys/vm/swappiness");
        if !swappiness.is_empty() {
            tweaks.push(TweakStatus {
                id: "swappiness".into(),
                name: "Reduce Swappiness".into(),
                description: "Reduces how aggressively the kernel swaps memory to disk. Lower values keep more data in RAM.".into(),
                category: "Memory".into(),
                sys_path: "/proc/sys/vm/swappiness".into(),
                current_value: swappiness.clone(),
                recommended_value: "10".into(),
                default_value: "60".into(),
                is_applied: swappiness.parse::<u32>().unwrap_or(60) <= 10,
            });
        }
    }

    // 3. Transparent Hugepages
    let thp_raw = read_sys("/sys/kernel/mm/transparent_hugepage/enabled");
    if !thp_raw.is_empty() {
        let thp = parse_thp(&thp_raw);
        tweaks.push(TweakStatus {
            id: "transparent_hugepages".into(),
            name: "Transparent Hugepages → madvise".into(),
            description: "Set to 'madvise' so only applications that request hugepages use them. Prevents latency spikes in games.".into(),
            category: "Memory".into(),
            sys_path: "/sys/kernel/mm/transparent_hugepage/enabled".into(),
            current_value: thp.clone(),
            recommended_value: "madvise".into(),
            default_value: "always".into(),
            is_applied: thp == "madvise",
        });
    }

    // 4. VFS Cache Pressure
    let vfs = read_sys("/proc/sys/vm/vfs_cache_pressure");
    if !vfs.is_empty() {
        tweaks.push(TweakStatus {
            id: "vfs_cache_pressure".into(),
            name: "Lower VFS Cache Pressure".into(),
            description: "Keeps directory and inode caches in memory longer, speeding up file system operations.".into(),
            category: "Memory".into(),
            sys_path: "/proc/sys/vm/vfs_cache_pressure".into(),
            current_value: vfs.clone(),
            recommended_value: "50".into(),
            default_value: "100".into(),
            is_applied: vfs.parse::<u32>().unwrap_or(100) <= 50,
        });
    }

    // 5. I/O Scheduler
    if let Some(dev) = detect_block_device() {
        let sched_path = format!("/sys/block/{}/queue/scheduler", dev);
        let sched_raw = read_sys(&sched_path);
        if !sched_raw.is_empty() {
            let sched = parse_scheduler(&sched_raw);
            let is_nvme = dev.starts_with("nvme");
            let recommended = if is_nvme { "none" } else { "mq-deadline" };
            let default_sched = if is_nvme { "none" } else { "mq-deadline" };
            tweaks.push(TweakStatus {
                id: "io_scheduler".into(),
                name: format!("I/O Scheduler → {} ({})", recommended, dev),
                description: if is_nvme {
                    "NVMe drives perform best with no scheduler overhead.".into()
                } else {
                    "mq-deadline provides low latency for rotational and SATA drives.".into()
                },
                category: "Storage".into(),
                sys_path: sched_path.clone(),
                current_value: sched.clone(),
                recommended_value: recommended.into(),
                default_value: default_sched.into(),
                is_applied: sched == recommended,
            });
        }
    }

    // 6. Compaction Proactiveness
    let compaction = read_sys("/proc/sys/vm/compaction_proactiveness");
    if !compaction.is_empty() {
        tweaks.push(TweakStatus {
            id: "compaction_proactiveness".into(),
            name: "Disable Proactive Compaction".into(),
            description: "Prevents background memory compaction that can cause micro-stutters during gaming.".into(),
            category: "Memory".into(),
            sys_path: "/proc/sys/vm/compaction_proactiveness".into(),
            current_value: compaction.clone(),
            recommended_value: "0".into(),
            default_value: "20".into(),
            is_applied: compaction == "0",
        });
    }

    // 7. Split Lock Mitigate
    let split_lock = read_sys("/proc/sys/kernel/split_lock_mitigate");
    if !split_lock.is_empty() {
        tweaks.push(TweakStatus {
            id: "split_lock_mitigate".into(),
            name: "Disable Split Lock Mitigation".into(),
            description: "Improves performance in some games and emulators that use split-lock instructions.".into(),
            category: "Kernel".into(),
            sys_path: "/proc/sys/kernel/split_lock_mitigate".into(),
            current_value: split_lock.clone(),
            recommended_value: "0".into(),
            default_value: "1".into(),
            is_applied: split_lock == "0",
        });
    }

    // 8. NMI Watchdog
    let nmi = read_sys("/proc/sys/kernel/nmi_watchdog");
    if !nmi.is_empty() {
        tweaks.push(TweakStatus {
            id: "nmi_watchdog".into(),
            name: "Disable NMI Watchdog".into(),
            description: "Frees up a performance counter and reduces overhead. Only needed for kernel debugging.".into(),
            category: "Kernel".into(),
            sys_path: "/proc/sys/kernel/nmi_watchdog".into(),
            current_value: nmi.clone(),
            recommended_value: "0".into(),
            default_value: "1".into(),
            is_applied: nmi == "0",
        });
    }

    // 9. CPU Boost (Turbo)
    let boost = read_sys("/sys/devices/system/cpu/cpufreq/boost");
    if !boost.is_empty() {
        tweaks.push(TweakStatus {
            id: "cpu_boost".into(),
            name: "Enable CPU Turbo Boost".into(),
            description: "Enables turbo/boost frequencies for higher single-thread performance.".into(),
            category: "CPU".into(),
            sys_path: "/sys/devices/system/cpu/cpufreq/boost".into(),
            current_value: boost.clone(),
            recommended_value: "1".into(),
            default_value: "1".into(),
            is_applied: boost == "1",
        });
    }

    // 10. Dirty Bytes (write-back tuning)
    let dirty_bytes = read_sys("/proc/sys/vm/dirty_bytes");
    let dirty_bg_bytes = read_sys("/proc/sys/vm/dirty_background_bytes");
    // Only show if not already using bytes-based limits (0 means ratio mode is active)
    let dirty_cur = if dirty_bytes == "0" {
        let ratio = read_sys("/proc/sys/vm/dirty_ratio");
        format!("ratio {}%", ratio)
    } else {
        format!("{}B", dirty_bytes)
    };
    tweaks.push(TweakStatus {
        id: "dirty_bytes".into(),
        name: "Optimize Dirty Page Writeback".into(),
        description: "Flushes dirty pages in smaller batches (256MB/128MB) to avoid large I/O stalls during writes.".into(),
        category: "Storage".into(),
        sys_path: "/proc/sys/vm/dirty_bytes + dirty_background_bytes".into(),
        current_value: dirty_cur,
        recommended_value: "256MB / 128MB bg".into(),
        default_value: "ratio 20%".into(),
        is_applied: dirty_bytes == "268435456" && dirty_bg_bytes == "134217728",
    });

    // 11. TCP Congestion Control → BBR
    let tcp_cc = read_sys("/proc/sys/net/ipv4/tcp_congestion_control");
    if !tcp_cc.is_empty() {
        tweaks.push(TweakStatus {
            id: "tcp_congestion".into(),
            name: "TCP Congestion → BBR".into(),
            description: "Google's BBR algorithm provides better throughput and lower latency than CUBIC.".into(),
            category: "Network".into(),
            sys_path: "/proc/sys/net/ipv4/tcp_congestion_control".into(),
            current_value: tcp_cc.clone(),
            recommended_value: "bbr".into(),
            default_value: "cubic".into(),
            is_applied: tcp_cc == "bbr",
        });
    }

    // 12. Default Queueing Discipline → fq
    let qdisc = read_sys("/proc/sys/net/core/default_qdisc");
    if !qdisc.is_empty() {
        tweaks.push(TweakStatus {
            id: "net_qdisc".into(),
            name: "Network Queue → fq".into(),
            description: "Fair Queue discipline pairs with BBR for optimal network performance.".into(),
            category: "Network".into(),
            sys_path: "/proc/sys/net/core/default_qdisc".into(),
            current_value: qdisc.clone(),
            recommended_value: "fq".into(),
            default_value: "fq_codel".into(),
            is_applied: qdisc == "fq",
        });
    }

    // 13. Watermark Boost Factor
    let wbf = read_sys("/proc/sys/vm/watermark_boost_factor");
    if !wbf.is_empty() {
        tweaks.push(TweakStatus {
            id: "watermark_boost_factor".into(),
            name: "Disable Watermark Boost".into(),
            description: "Prevents aggressive memory reclaim after fragmentation events, reducing unnecessary page scanning.".into(),
            category: "Memory".into(),
            sys_path: "/proc/sys/vm/watermark_boost_factor".into(),
            current_value: wbf.clone(),
            recommended_value: "0".into(),
            default_value: "15000".into(),
            is_applied: wbf == "0",
        });
    }

    // 14. Max Memory Map Count
    let mmap = read_sys("/proc/sys/vm/max_map_count");
    if !mmap.is_empty() {
        let mmap_val = mmap.parse::<u64>().unwrap_or(65530);
        tweaks.push(TweakStatus {
            id: "max_map_count".into(),
            name: "Increase Memory Map Limit".into(),
            description: "Required by some Steam/Proton games and large applications. Default since kernel 6.6+.".into(),
            category: "Memory".into(),
            sys_path: "/proc/sys/vm/max_map_count".into(),
            current_value: mmap.clone(),
            recommended_value: "2147483642".into(),
            default_value: "65530".into(),
            is_applied: mmap_val >= 2147483642,
        });
    }

    tweaks
}

pub fn apply_tweak(id: &str) -> TweakResult {
    let result = match id {
        "cpu_governor" => {
            let r = apply_cpu_governor("performance");
            if r.is_ok() {
                persist_sysfs("/sys/devices/system/cpu/cpu[0-9]*/cpufreq/scaling_governor", "performance");
            }
            r
        }
        "swappiness" => {
            let r = apply_sysctl("vm.swappiness", "10");
            if r.is_ok() { persist_sysctl_perf("vm.swappiness", "10"); }
            r
        }
        "transparent_hugepages" => {
            let r = apply_thp("madvise");
            if r.is_ok() {
                persist_sysfs("/sys/kernel/mm/transparent_hugepage/enabled", "madvise");
            }
            r
        }
        "vfs_cache_pressure" => {
            let r = apply_sysctl("vm.vfs_cache_pressure", "50");
            if r.is_ok() { persist_sysctl_perf("vm.vfs_cache_pressure", "50"); }
            r
        }
        "io_scheduler" => {
            let r = apply_io_scheduler();
            if r.is_ok() {
                if let Some(dev) = detect_block_device() {
                    let sched = if dev.starts_with("nvme") { "none" } else { "mq-deadline" };
                    persist_sysfs(&format!("/sys/block/{}/queue/scheduler", dev), sched);
                }
            }
            r
        }
        "compaction_proactiveness" => {
            let r = apply_sysctl("vm.compaction_proactiveness", "0");
            if r.is_ok() { persist_sysctl_perf("vm.compaction_proactiveness", "0"); }
            r
        }
        "split_lock_mitigate" => {
            let r = apply_sysctl("kernel.split_lock_mitigate", "0");
            if r.is_ok() { persist_sysctl_perf("kernel.split_lock_mitigate", "0"); }
            r
        }
        "nmi_watchdog" => {
            let r = apply_sysctl("kernel.nmi_watchdog", "0");
            if r.is_ok() { persist_sysctl_perf("kernel.nmi_watchdog", "0"); }
            r
        }
        "cpu_boost" => {
            let r = apply_sysfs("/sys/devices/system/cpu/cpufreq/boost", "1");
            if r.is_ok() {
                persist_sysfs("/sys/devices/system/cpu/cpufreq/boost", "1");
            }
            r
        }
        "dirty_bytes" => {
            let r = apply_dirty_bytes("268435456", "134217728");
            if r.is_ok() {
                persist_sysctl_perf("vm.dirty_ratio", "0");
                persist_sysctl_perf("vm.dirty_background_ratio", "0");
                persist_sysctl_perf("vm.dirty_bytes", "268435456");
                persist_sysctl_perf("vm.dirty_background_bytes", "134217728");
            }
            r
        }
        "tcp_congestion" => {
            let r = apply_sysctl("net.ipv4.tcp_congestion_control", "bbr");
            if r.is_ok() { persist_sysctl_perf("net.ipv4.tcp_congestion_control", "bbr"); }
            r
        }
        "net_qdisc" => {
            let r = apply_sysctl("net.core.default_qdisc", "fq");
            if r.is_ok() { persist_sysctl_perf("net.core.default_qdisc", "fq"); }
            r
        }
        "watermark_boost_factor" => {
            let r = apply_sysctl("vm.watermark_boost_factor", "0");
            if r.is_ok() { persist_sysctl_perf("vm.watermark_boost_factor", "0"); }
            r
        }
        "max_map_count" => {
            let r = apply_sysctl("vm.max_map_count", "2147483642");
            if r.is_ok() { persist_sysctl_perf("vm.max_map_count", "2147483642"); }
            r
        }
        _ => Err(format!("Unknown tweak: {}", id)),
    };

    match result {
        Ok(msg) => TweakResult {
            id: id.to_string(),
            success: true,
            message: msg,
        },
        Err(msg) => TweakResult {
            id: id.to_string(),
            success: false,
            message: msg,
        },
    }
}

fn apply_cpu_governor(governor: &str) -> Result<String, String> {
    // Count CPUs
    let cpu_count = fs::read_dir("/sys/devices/system/cpu")
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    let n = e.file_name().to_string_lossy().to_string();
                    n.starts_with("cpu") && n[3..].chars().all(|c| c.is_ascii_digit())
                })
                .count()
        })
        .unwrap_or(0);

    if cpu_count == 0 {
        return Err("No CPUs found".into());
    }

    // Use pkexec to write to each CPU's governor
    let script = format!(
        "for i in $(seq 0 {}); do echo {} > /sys/devices/system/cpu/cpu$i/cpufreq/scaling_governor 2>/dev/null; done",
        cpu_count - 1,
        governor
    );
    run_cmd("pkexec", &["bash", "-c", &script])?;
    Ok(format!("Set CPU governor to '{}' on {} cores", governor, cpu_count))
}

fn apply_sysctl(key: &str, value: &str) -> Result<String, String> {
    let param = format!("{}={}", key, value);
    run_cmd("pkexec", &["sysctl", "-w", &param])?;
    Ok(format!("Set {} = {}", key, value))
}

fn apply_thp(mode: &str) -> Result<String, String> {
    let script = format!(
        "echo {} > /sys/kernel/mm/transparent_hugepage/enabled",
        mode
    );
    run_cmd("pkexec", &["bash", "-c", &script])?;
    Ok(format!("Set transparent hugepages to '{}'", mode))
}

fn apply_io_scheduler() -> Result<String, String> {
    let dev = detect_block_device().ok_or("No block device found")?;
    let is_nvme = dev.starts_with("nvme");
    let sched = if is_nvme { "none" } else { "mq-deadline" };

    let script = format!(
        "echo {} > /sys/block/{}/queue/scheduler",
        sched, dev
    );
    run_cmd("pkexec", &["bash", "-c", &script])?;
    Ok(format!("Set I/O scheduler to '{}' on {}", sched, dev))
}

fn apply_sysfs(path: &str, value: &str) -> Result<String, String> {
    let script = format!("echo {} > {}", value, path);
    run_cmd("pkexec", &["bash", "-c", &script])?;
    Ok(format!("Set {} = {}", path, value))
}

fn apply_dirty_bytes(bytes: &str, bg_bytes: &str) -> Result<String, String> {
    // Must set dirty_ratio/dirty_background_ratio to 0 first when switching to byte mode
    run_cmd("pkexec", &["sysctl", "-w", "vm.dirty_ratio=0"])?;
    run_cmd("pkexec", &["sysctl", "-w", "vm.dirty_background_ratio=0"])?;
    run_cmd("pkexec", &["sysctl", "-w", &format!("vm.dirty_bytes={}", bytes)])?;
    run_cmd("pkexec", &["sysctl", "-w", &format!("vm.dirty_background_bytes={}", bg_bytes)])?;
    Ok("Set dirty writeback to 256MB / 128MB background".into())
}

fn restore_dirty_bytes() -> Result<String, String> {
    // Switch back to ratio mode: set bytes to 0, then restore ratios
    run_cmd("pkexec", &["sysctl", "-w", "vm.dirty_bytes=0"])?;
    run_cmd("pkexec", &["sysctl", "-w", "vm.dirty_background_bytes=0"])?;
    run_cmd("pkexec", &["sysctl", "-w", "vm.dirty_ratio=20"])?;
    run_cmd("pkexec", &["sysctl", "-w", "vm.dirty_background_ratio=10"])?;
    Ok("Restored dirty writeback to ratio 20% / 10% background".into())
}

pub fn restore_tweak(id: &str) -> TweakResult {
    let result = match id {
        "cpu_governor" => {
            let gov = detect_default_governor();
            let r = apply_cpu_governor(&gov);
            remove_persisted_sysfs("/sys/devices/system/cpu/cpu[0-9]*/cpufreq/scaling_governor");
            r
        }
        "swappiness" => {
            let r = apply_sysctl("vm.swappiness", "60");
            remove_persisted_sysctl_perf("vm.swappiness");
            r
        }
        "transparent_hugepages" => {
            let r = apply_thp("always");
            remove_persisted_sysfs("/sys/kernel/mm/transparent_hugepage/enabled");
            r
        }
        "vfs_cache_pressure" => {
            let r = apply_sysctl("vm.vfs_cache_pressure", "100");
            remove_persisted_sysctl_perf("vm.vfs_cache_pressure");
            r
        }
        "io_scheduler" => {
            let r = apply_io_scheduler();
            if let Some(dev) = detect_block_device() {
                remove_persisted_sysfs(&format!("/sys/block/{}/queue/scheduler", dev));
            }
            r
        }
        "compaction_proactiveness" => {
            let r = apply_sysctl("vm.compaction_proactiveness", "20");
            remove_persisted_sysctl_perf("vm.compaction_proactiveness");
            r
        }
        "split_lock_mitigate" => {
            let r = apply_sysctl("kernel.split_lock_mitigate", "1");
            remove_persisted_sysctl_perf("kernel.split_lock_mitigate");
            r
        }
        "nmi_watchdog" => {
            let r = apply_sysctl("kernel.nmi_watchdog", "1");
            remove_persisted_sysctl_perf("kernel.nmi_watchdog");
            r
        }
        "cpu_boost" => {
            let r = apply_sysfs("/sys/devices/system/cpu/cpufreq/boost", "1");
            remove_persisted_sysfs("/sys/devices/system/cpu/cpufreq/boost");
            r
        }
        "dirty_bytes" => {
            let r = restore_dirty_bytes();
            remove_persisted_sysctl_perf("vm.dirty_ratio");
            remove_persisted_sysctl_perf("vm.dirty_background_ratio");
            remove_persisted_sysctl_perf("vm.dirty_bytes");
            remove_persisted_sysctl_perf("vm.dirty_background_bytes");
            r
        }
        "tcp_congestion" => {
            let r = apply_sysctl("net.ipv4.tcp_congestion_control", "cubic");
            remove_persisted_sysctl_perf("net.ipv4.tcp_congestion_control");
            r
        }
        "net_qdisc" => {
            let r = apply_sysctl("net.core.default_qdisc", "fq_codel");
            remove_persisted_sysctl_perf("net.core.default_qdisc");
            r
        }
        "watermark_boost_factor" => {
            let r = apply_sysctl("vm.watermark_boost_factor", "15000");
            remove_persisted_sysctl_perf("vm.watermark_boost_factor");
            r
        }
        "max_map_count" => {
            let r = apply_sysctl("vm.max_map_count", "65530");
            remove_persisted_sysctl_perf("vm.max_map_count");
            r
        }
        _ => Err(format!("Unknown tweak: {}", id)),
    };

    match result {
        Ok(msg) => TweakResult {
            id: id.to_string(),
            success: true,
            message: msg,
        },
        Err(msg) => TweakResult {
            id: id.to_string(),
            success: false,
            message: msg,
        },
    }
}
