use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

/// Cached virtual mount points — computed once per process.
fn virtual_mount_points() -> &'static HashSet<String> {
    static CACHE: OnceLock<HashSet<String>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut set = HashSet::new();
        if let Ok(content) = fs::read_to_string("/proc/mounts") {
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 3 {
                    continue;
                }
                let mount = parts[1];
                let fstype = parts[2];
                match fstype {
                    "proc" | "sysfs" | "devtmpfs" | "devpts" | "tmpfs"
                    | "cgroup" | "cgroup2" | "overlay" | "squashfs"
                    | "fuse.portal" | "fusectl" | "efivarfs"
                    | "securityfs" | "debugfs" | "configfs"
                    | "hugetlbfs" | "mqueue" | "pstore"
                    | "autofs" | "binfmt_misc" | "tracefs" => {
                        set.insert(mount.to_string());
                    }
                    _ => {}
                }
            }
        }
        set
    })
}

/// Check if a path is a virtual mount point (exact match only for top-level skipping).
fn is_virtual_path(path: &str) -> bool {
    let skip = virtual_mount_points();
    skip.contains(path) || skip.iter().any(|vp| path.starts_with(&format!("{}/", vp)))
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskPartition {
    pub device: String,
    pub mount_point: String,
    pub fs_type: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub usage_percent: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpaceEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub children: Vec<SpaceEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskOverview {
    pub partitions: Vec<DiskPartition>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpaceAnalysis {
    pub root_path: String,
    pub total_size: u64,
    pub entries: Vec<SpaceEntry>,
}

/// Get an overview of all mounted partitions by parsing /proc/mounts + statvfs.
pub fn get_disk_overview() -> DiskOverview {
    let mut partitions = Vec::new();

    if let Ok(content) = fs::read_to_string("/proc/mounts") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }
            let device = parts[0];
            let mount = parts[1];
            let fstype = parts[2];

            // Skip virtual/pseudo filesystems
            if !device.starts_with('/') {
                continue;
            }
            // Skip snap mounts and kernel pseudo-mounts
            if mount.starts_with("/snap/")
                || mount == "/proc"
                || mount.starts_with("/proc/")
                || mount == "/sys"
                || mount.starts_with("/sys/")
            {
                continue;
            }
            // Skip known pseudo filesystem types
            match fstype {
                "proc" | "sysfs" | "devtmpfs" | "devpts" | "tmpfs"
                | "cgroup" | "cgroup2" | "overlay" | "squashfs"
                | "fuse.portal" | "fusectl" | "efivarfs"
                | "securityfs" | "debugfs" | "configfs"
                | "hugetlbfs" | "mqueue" | "pstore"
                | "autofs" | "binfmt_misc" | "tracefs" => continue,
                _ => {}
            }

            if let Some(stat) = statvfs(mount) {
                let total = stat.total_bytes;
                let available = stat.available_bytes;
                let used = total.saturating_sub(available);
                let pct = if total > 0 {
                    (used as f64 / total as f64) * 100.0
                } else {
                    0.0
                };

                partitions.push(DiskPartition {
                    device: device.to_string(),
                    mount_point: mount.to_string(),
                    fs_type: fstype.to_string(),
                    total_bytes: total,
                    used_bytes: used,
                    available_bytes: available,
                    usage_percent: (pct * 10.0).round() / 10.0,
                });
            }
        }
    }

    // Sort: root first, then by used_bytes descending
    partitions.sort_by(|a, b| {
        if a.mount_point == "/" {
            std::cmp::Ordering::Less
        } else if b.mount_point == "/" {
            std::cmp::Ordering::Greater
        } else {
            b.used_bytes.cmp(&a.used_bytes)
        }
    });

    DiskOverview { partitions }
}

/// Analyze space usage from a root path using jwalk for maximum parallel I/O.
/// Returns top 25 entries sorted by size. Children are lazy-loaded by the frontend.
pub fn analyze_space(root: &str) -> SpaceAnalysis {
    let root_path = Path::new(root);

    // Use jwalk to walk the entire tree in parallel.
    // We accumulate sizes per direct child of root.
    let mut child_sizes: HashMap<String, u64> = HashMap::new();
    let mut child_is_dir: HashMap<String, bool> = HashMap::new();

    let root_depth = root_path.components().count();

    for entry in jwalk::WalkDir::new(root)
        .skip_hidden(false)
        .follow_links(false)
        .sort(false)
        .process_read_dir(|_depth, _path, _state, children| {
            // Skip virtual filesystem directories inside the walker
            children.retain(|child_result| {
                if let Ok(child) = child_result {
                    let p = child.path();
                    let ps = p.to_string_lossy();
                    if is_virtual_path(&ps) {
                        return false;
                    }
                    // Skip symlinks
                    if child.file_type.is_symlink() {
                        return false;
                    }
                }
                true
            });
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let entry_path = entry.path();
        let entry_depth = entry_path.components().count();

        // Skip the root directory itself
        if entry_depth <= root_depth {
            continue;
        }

        // Get file size (skip impossibly large virtual files)
        let size = match entry.metadata() {
            Ok(meta) => {
                if meta.is_symlink() {
                    continue;
                }
                let len = meta.len();
                if !meta.is_dir() && len > 1_099_511_627_776 {
                    continue;
                }
                if meta.is_dir() {
                    0 // directories don't have size themselves
                } else {
                    len
                }
            }
            Err(_) => continue,
        };

        // Find which direct child of root this belongs to
        let relative = match entry_path.strip_prefix(root_path) {
            Ok(r) => r,
            Err(_) => continue,
        };

        // The first component is our direct child
        if let Some(first) = relative.components().next() {
            let child_name = first.as_os_str().to_string_lossy().to_string();
            let child_full_path = root_path.join(&child_name);
            let key = child_full_path.to_string_lossy().to_string();

            *child_sizes.entry(key.clone()).or_insert(0) += size;

            // Record if the direct child itself is a directory
            if entry_depth == root_depth + 1 {
                let is_dir = entry.file_type.is_dir();
                child_is_dir.insert(key, is_dir);
            }
        }
    }

    // Build entries from accumulated sizes
    let mut entries: Vec<SpaceEntry> = child_sizes
        .into_iter()
        .filter_map(|(path, size)| {
            if size == 0 {
                return None;
            }
            let is_dir = child_is_dir.get(&path).copied().unwrap_or(false);
            let name = Path::new(&path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.clone());

            Some(SpaceEntry {
                name,
                path,
                size,
                is_dir,
                children: Vec::new(),
            })
        })
        .collect();

    entries.sort_by(|a, b| b.size.cmp(&a.size));
    entries.truncate(25);

    let total_size: u64 = entries.iter().map(|e| e.size).sum();

    SpaceAnalysis {
        root_path: root.to_string(),
        total_size,
        entries,
    }
}

struct StatVfsResult {
    total_bytes: u64,
    available_bytes: u64,
}

/// Call libc statvfs to get filesystem capacity info.
fn statvfs(path: &str) -> Option<StatVfsResult> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;

    let c_path = CString::new(path).ok()?;
    let mut buf = MaybeUninit::<libc::statvfs>::uninit();

    let ret = unsafe { libc::statvfs(c_path.as_ptr(), buf.as_mut_ptr()) };

    if ret != 0 {
        return None;
    }

    let stat = unsafe { buf.assume_init() };
    let block_size = stat.f_frsize as u64;

    Some(StatVfsResult {
        total_bytes: stat.f_blocks * block_size,
        available_bytes: stat.f_bavail * block_size,
    })
}
