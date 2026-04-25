use serde::{Deserialize, Serialize};
use crate::sysenv::system_command;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub host: HostInfo,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub gpus: Vec<GpuInfo>,
    pub disks: Vec<DiskInfo>,
    pub network: Vec<NetworkInterface>,
    pub sensors: Vec<SensorGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostInfo {
    pub hostname: String,
    pub kernel: String,
    pub architecture: String,
    pub uptime: String,
    pub board_vendor: String,
    pub board_name: String,
    pub bios_vendor: String,
    pub bios_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub model: String,
    pub cores: u32,
    pub threads: u32,
    pub max_freq_mhz: f64,
    pub architecture: String,
    pub cache: String,
    pub vendor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GpuInfo {
    pub name: String,
    pub vendor: String,
    pub driver: String,
    pub pci_slot: String,
    pub vram_total_mb: u64,
    pub vram_used_mb: u64,
    pub vram_free_mb: u64,
    pub clock_gpu_mhz: u32,
    pub clock_gpu_max_mhz: u32,
    pub clock_mem_mhz: u32,
    pub clock_mem_max_mhz: u32,
    pub power_draw_w: f64,
    pub power_limit_w: f64,
    pub temperature_c: u32,
    pub pcie_link_speed: String,
    pub pcie_link_width: String,
    pub fan_speed_pct: String,
    pub pstate: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub name: String,
    pub model: String,
    pub size_bytes: u64,
    pub disk_type: String,
    pub partitions: Vec<DiskPartitionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskPartitionInfo {
    pub name: String,
    pub size_bytes: u64,
    pub mountpoint: String,
    pub fstype: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub mac: String,
    pub ipv4: String,
    pub ipv6: String,
    pub state: String,
    pub speed: String,
    pub iface_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorGroup {
    pub name: String,
    pub readings: Vec<SensorReading>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    pub label: String,
    pub value: f64,
    pub unit: String,
    pub high: Option<f64>,
    pub critical: Option<f64>,
}

pub fn get_hardware_info() -> HardwareInfo {
    HardwareInfo {
        host: gather_host_info(),
        cpu: gather_cpu_info(),
        memory: gather_memory_info(),
        gpus: gather_gpu_info(),
        disks: gather_disk_info(),
        network: gather_network_info(),
        sensors: gather_sensors(),
    }
}

fn read_sys_file(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn gather_host_info() -> HostInfo {
    let hostname = read_sys_file("/proc/sys/kernel/hostname");
    let kernel = read_sys_file("/proc/sys/kernel/osrelease");
    let architecture = std::env::consts::ARCH.to_string();

    // Uptime
    let uptime_str = read_sys_file("/proc/uptime");
    let uptime_secs: f64 = uptime_str
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    let uptime = format_uptime(uptime_secs as u64);

    // DMI info (may need root, gracefully fallback)
    let dmi = "/sys/devices/virtual/dmi/id";
    let board_vendor = read_sys_file(&format!("{}/board_vendor", dmi));
    let board_name = read_sys_file(&format!("{}/board_name", dmi));
    let bios_vendor = read_sys_file(&format!("{}/bios_vendor", dmi));
    let bios_version = read_sys_file(&format!("{}/bios_version", dmi));

    HostInfo {
        hostname,
        kernel,
        architecture,
        uptime,
        board_vendor,
        board_name,
        bios_vendor,
        bios_version,
    }
}

fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 {
        format!("{}d {}h {}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

fn gather_cpu_info() -> CpuInfo {
    let cpuinfo = read_sys_file("/proc/cpuinfo");
    let mut model = String::new();
    let mut vendor = String::new();
    let mut cache = String::new();
    let mut seen_processors = 0u32;
    let mut core_ids = std::collections::HashSet::new();

    for line in cpuinfo.lines() {
        if let Some((key, val)) = line.split_once(':') {
            let key = key.trim();
            let val = val.trim();
            match key {
                "model name" => {
                    if model.is_empty() {
                        model = val.to_string();
                    }
                }
                "vendor_id" => {
                    if vendor.is_empty() {
                        vendor = val.to_string();
                    }
                }
                "processor" => {
                    seen_processors += 1;
                }
                "core id" => {
                    core_ids.insert(val.to_string());
                }
                "cache size" => {
                    if cache.is_empty() {
                        cache = val.to_string();
                    }
                }
                _ => {}
            }
        }
    }

    let threads = seen_processors;
    let cores = if core_ids.is_empty() {
        threads
    } else {
        core_ids.len() as u32
    };

    // Try to get max freq from scaling
    let max_freq_mhz = read_sys_file("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq")
        .parse::<f64>()
        .map(|khz| khz / 1000.0)
        .unwrap_or(0.0);

    CpuInfo {
        model,
        cores,
        threads,
        max_freq_mhz,
        architecture: std::env::consts::ARCH.to_string(),
        cache,
        vendor,
    }
}

fn gather_memory_info() -> MemoryInfo {
    let meminfo = read_sys_file("/proc/meminfo");
    let mut map: HashMap<String, u64> = HashMap::new();

    for line in meminfo.lines() {
        if let Some((key, rest)) = line.split_once(':') {
            let val_kb: u64 = rest
                .trim()
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            map.insert(key.to_string(), val_kb * 1024);
        }
    }

    let total = *map.get("MemTotal").unwrap_or(&0);
    let available = *map.get("MemAvailable").unwrap_or(&0);
    let swap_total = *map.get("SwapTotal").unwrap_or(&0);
    let swap_free = *map.get("SwapFree").unwrap_or(&0);

    MemoryInfo {
        total_bytes: total,
        available_bytes: available,
        used_bytes: total.saturating_sub(available),
        swap_total_bytes: swap_total,
        swap_used_bytes: swap_total.saturating_sub(swap_free),
    }
}

fn gather_gpu_info() -> Vec<GpuInfo> {
    let mut gpus = Vec::new();

    // Use lspci to find GPUs
    let output = system_command("lspci")
        .args(["-mm", "-nn"])
        .output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let lower = line.to_lowercase();
            if lower.contains("vga") || lower.contains("3d controller") || lower.contains("display") {
                let gpu = parse_lspci_line(line);
                if let Some(g) = gpu {
                    gpus.push(g);
                }
            }
        }
    }

    // Fallback: try reading /sys/class/drm
    if gpus.is_empty() {
        if let Ok(entries) = fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("card") && !name.contains('-') {
                    let device_path = format!("/sys/class/drm/{}/device", name);
                    let vendor = read_sys_file(&format!("{}/vendor", device_path));
                    let device = read_sys_file(&format!("{}/device", device_path));
                    if !vendor.is_empty() {
                        gpus.push(GpuInfo {
                            name: format!("GPU {}", name),
                            vendor,
                            driver: read_sys_file(&format!("{}/driver/module/drivers", device_path)),
                            pci_slot: device,
                            ..Default::default()
                        });
                    }
                }
            }
        }
    }

    // Enrich with nvidia-smi data
    enrich_nvidia_gpus(&mut gpus);

    // Read PCIe link info from sysfs for each GPU
    for gpu in &mut gpus {
        if !gpu.pci_slot.is_empty() {
            let dev_path = format!("/sys/bus/pci/devices/0000:{}", gpu.pci_slot);
            let speed = read_sys_file(&format!("{}/current_link_speed", dev_path));
            let width = read_sys_file(&format!("{}/current_link_width", dev_path));
            if !speed.is_empty() {
                gpu.pcie_link_speed = speed;
            }
            if !width.is_empty() {
                gpu.pcie_link_width = format!("x{}", width);
            }
        }
    }

    gpus
}

fn enrich_nvidia_gpus(gpus: &mut [GpuInfo]) {
    let output = system_command("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total,memory.used,memory.free,clocks.max.graphics,clocks.current.graphics,clocks.max.memory,clocks.current.memory,power.limit,power.draw,temperature.gpu,pstate,fan.speed",
            "--format=csv,noheader,nounits",
        ])
        .output();

    let out = match output {
        Ok(o) if o.status.success() => o,
        _ => return,
    };

    let stdout = String::from_utf8_lossy(&out.stdout);
    for (i, line) in stdout.lines().enumerate() {
        let fields: Vec<&str> = line.split(", ").collect();
        if fields.len() < 13 {
            continue;
        }
        // Match by index (nvidia-smi lists GPUs in order)
        let gpu = match gpus.get_mut(i) {
            Some(g) => g,
            None => continue,
        };

        gpu.vram_total_mb = fields[1].trim().parse().unwrap_or(0);
        gpu.vram_used_mb = fields[2].trim().parse().unwrap_or(0);
        gpu.vram_free_mb = fields[3].trim().parse().unwrap_or(0);
        gpu.clock_gpu_max_mhz = fields[4].trim().parse().unwrap_or(0);
        gpu.clock_gpu_mhz = fields[5].trim().parse().unwrap_or(0);
        gpu.clock_mem_max_mhz = fields[6].trim().parse().unwrap_or(0);
        gpu.clock_mem_mhz = fields[7].trim().parse().unwrap_or(0);
        gpu.power_limit_w = fields[8].trim().parse().unwrap_or(0.0);
        gpu.power_draw_w = fields[9].trim().parse().unwrap_or(0.0);
        gpu.temperature_c = fields[10].trim().parse().unwrap_or(0);
        gpu.pstate = fields[11].trim().to_string();
        let fan_str = fields[12].trim();
        if fan_str != "[Not Supported]" {
            gpu.fan_speed_pct = format!("{}%", fan_str);
        }

        // Override name with nvidia-smi's clean name
        let smi_name = fields[0].trim();
        if !smi_name.is_empty() {
            gpu.name = smi_name.to_string();
        }
    }
}

fn strip_pci_ids(s: &str) -> String {
    // Remove PCI ID suffixes like " [10de]" or " [2484]"
    let mut result = s.to_string();
    while let Some(start) = result.rfind(" [") {
        if let Some(end) = result[start..].find(']') {
            let bracket_content = &result[start + 2..start + end];
            // Only strip if it looks like a hex PCI ID (4 hex chars)
            if bracket_content.len() == 4 && bracket_content.chars().all(|c| c.is_ascii_hexdigit()) {
                result = format!("{}{}", &result[..start], &result[start + end + 1..]);
                continue;
            }
        }
        break;
    }
    result.trim().to_string()
}

fn parse_lspci_line(line: &str) -> Option<GpuInfo> {
    // lspci -mm -nn format: Slot "Class" "Vendor" "Device" ...
    let parts: Vec<&str> = line.splitn(2, ' ').collect();
    if parts.len() < 2 {
        return None;
    }
    let pci_slot = parts[0].to_string();

    // Extract quoted fields
    let quoted: Vec<String> = extract_quoted_fields(parts[1]);

    let vendor = quoted.get(1).map(|s| strip_pci_ids(s)).unwrap_or_default();
    let name = quoted.get(2).map(|s| strip_pci_ids(s)).unwrap_or_default();

    // Try to find driver
    let driver_path = format!("/sys/bus/pci/devices/0000:{}/driver", pci_slot);
    let driver = if let Ok(link) = fs::read_link(&driver_path) {
        link.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    Some(GpuInfo {
        name,
        vendor,
        driver,
        pci_slot,
        ..Default::default()
    })
}

fn extract_quoted_fields(s: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c == '"' {
            chars.next();
            let mut field = String::new();
            for ch in chars.by_ref() {
                if ch == '"' {
                    break;
                }
                field.push(ch);
            }
            fields.push(field);
        } else {
            chars.next();
        }
    }
    fields
}

fn gather_disk_info() -> Vec<DiskInfo> {
    let mut disks = Vec::new();

    // Use lsblk for a clean view
    let output = system_command("lsblk")
        .args(["-b", "-J", "-o", "NAME,SIZE,TYPE,MODEL,MOUNTPOINT,FSTYPE,ROTA"])
        .output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
            if let Some(devices) = json["blockdevices"].as_array() {
                for dev in devices {
                    let dtype = dev["type"].as_str().unwrap_or("");
                    if dtype != "disk" {
                        continue;
                    }

                    let name = dev["name"].as_str().unwrap_or("").to_string();
                    let model = dev["model"]
                        .as_str()
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    let size = dev["size"].as_u64().unwrap_or(0);
                    let rotational = dev["rota"]
                        .as_bool()
                        .unwrap_or(true);
                    let disk_type = if rotational { "HDD" } else { "SSD" }.to_string();

                    let mut partitions = Vec::new();
                    if let Some(children) = dev["children"].as_array() {
                        for child in children {
                            let ctype = child["type"].as_str().unwrap_or("");
                            if ctype != "part" {
                                continue;
                            }
                            partitions.push(DiskPartitionInfo {
                                name: child["name"].as_str().unwrap_or("").to_string(),
                                size_bytes: child["size"].as_u64().unwrap_or(0),
                                mountpoint: child["mountpoint"]
                                    .as_str()
                                    .unwrap_or("")
                                    .to_string(),
                                fstype: child["fstype"]
                                    .as_str()
                                    .unwrap_or("")
                                    .to_string(),
                            });
                        }
                    }

                    disks.push(DiskInfo {
                        name,
                        model,
                        size_bytes: size,
                        disk_type,
                        partitions,
                    });
                }
            }
        }
    }

    disks
}

fn gather_network_info() -> Vec<NetworkInterface> {
    let mut interfaces = Vec::new();
    let net_dir = Path::new("/sys/class/net");

    if let Ok(entries) = fs::read_dir(net_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "lo" {
                continue;
            }

            let base = format!("/sys/class/net/{}", name);
            let state = read_sys_file(&format!("{}/operstate", base));
            let mac = read_sys_file(&format!("{}/address", base));
            let speed_raw = read_sys_file(&format!("{}/speed", base));
            let speed = if speed_raw.is_empty() || speed_raw.starts_with('-') {
                String::new()
            } else {
                format!("{} Mbps", speed_raw)
            };

            // Determine type
            let iface_type = if name.starts_with('w') {
                "WiFi"
            } else if name.starts_with('e') {
                "Ethernet"
            } else if name.starts_with("br") {
                "Bridge"
            } else if name.starts_with("veth") || name.starts_with("docker") {
                "Virtual"
            } else if name.starts_with("tun") || name.starts_with("tap") {
                "VPN"
            } else {
                "Other"
            }
            .to_string();

            // Get IP addresses via ip command
            let (ipv4, ipv6) = get_interface_ips(&name);

            interfaces.push(NetworkInterface {
                name,
                mac,
                ipv4,
                ipv6,
                state,
                speed,
                iface_type,
            });
        }
    }

    // Sort: up first, then by name
    interfaces.sort_by(|a, b| {
        let a_up = a.state == "up";
        let b_up = b.state == "up";
        b_up.cmp(&a_up).then(a.name.cmp(&b.name))
    });

    interfaces
}

fn get_interface_ips(name: &str) -> (String, String) {
    let mut ipv4 = String::new();
    let mut ipv6 = String::new();

    let output = system_command("ip")
        .args(["-o", "addr", "show", name])
        .output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(idx) = parts.iter().position(|&p| p == "inet" || p == "inet6") {
                if let Some(addr) = parts.get(idx + 1) {
                    let addr_only = addr.split('/').next().unwrap_or(addr);
                    if parts[idx] == "inet" && ipv4.is_empty() {
                        ipv4 = addr_only.to_string();
                    } else if parts[idx] == "inet6"
                        && ipv6.is_empty()
                        && !addr_only.starts_with("fe80")
                    {
                        ipv6 = addr_only.to_string();
                    }
                }
            }
        }
    }

    (ipv4, ipv6)
}

fn friendly_sensor_name(raw: &str) -> String {
    match raw.to_lowercase().as_str() {
        "coretemp" => "CPU Core Temperature".to_string(),
        "acpitz" => "ACPI Thermal Zone (Motherboard)".to_string(),
        "k10temp" => "AMD CPU Temperature".to_string(),
        "zenpower" => "AMD Zen CPU Temperature".to_string(),
        "it8688" | "it8686" | "it8665" | "it8792" | "it8628" | "it87" => format!("Motherboard Sensors ({})", raw.to_uppercase()),
        "nct6775" | "nct6776" | "nct6779" | "nct6791" | "nct6792" | "nct6793" | "nct6795" | "nct6796" | "nct6797" | "nct6798" | "nct6799" => format!("Motherboard Sensors ({})", raw.to_uppercase()),
        "w83627ehf" | "w83627dhg" | "w83667hg" | "w83795g" => format!("Motherboard Sensors ({})", raw.to_uppercase()),
        "f71882fg" | "f71889ed" | "f71858fg" => format!("Motherboard Sensors ({})", raw.to_uppercase()),
        "asus_wmi_sensors" | "asus-wmi-sensors" => "ASUS WMI Sensors (Motherboard)".to_string(),
        "asus-ec-sensors" => "ASUS EC Sensors (Motherboard)".to_string(),
        "amdgpu" => "AMD GPU".to_string(),
        "nouveau" => "NVIDIA GPU (Nouveau)".to_string(),
        "nvidia" => "NVIDIA GPU".to_string(),
        "radeon" => "AMD/ATI GPU (Radeon)".to_string(),
        "intel_powerclamp" => "Intel PowerClamp (CPU Idle)".to_string(),
        "pch_cannonlake" | "pch_cometlake" | "pch_skylake" | "pch_haswell" => "PCH (Chipset) Temperature".to_string(),
        "iwlwifi_1" | "iwlwifi" => "WiFi Adapter (Intel)".to_string(),
        "mt7921_phy0" | "mt7922_phy0" | "mt7921e" => "WiFi Adapter (MediaTek)".to_string(),
        "thinkpad" => "ThinkPad Embedded Controller".to_string(),
        "dell_smm" => "Dell SMM (Fan/Thermal)".to_string(),
        "hp_wmi" | "hp-wmi" => "HP WMI Sensors".to_string(),
        "nvme" => "NVMe SSD".to_string(),
        "drivetemp" => "HDD/SSD Temperature".to_string(),
        "bat0" | "bat1" | "bat2" => format!("Battery ({})", raw.to_uppercase()),
        "tpacpi" => "ThinkPad ACPI".to_string(),
        name if name.starts_with("nvme") => format!("NVMe SSD ({})", raw),
        _ => raw.to_uppercase(),
    }
}

fn gather_sensors() -> Vec<SensorGroup> {
    let mut groups = Vec::new();
    let hwmon_dir = Path::new("/sys/class/hwmon");

    if let Ok(entries) = fs::read_dir(hwmon_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = read_sys_file(&format!("{}/name", path.display()));
            if name.is_empty() {
                continue;
            }

            let mut readings = Vec::new();

            // Scan for temp, fan, and voltage inputs
            if let Ok(files) = fs::read_dir(&path) {
                let mut sensor_files: Vec<String> = files
                    .flatten()
                    .filter_map(|f| {
                        let fname = f.file_name().to_string_lossy().to_string();
                        if fname.ends_with("_input") {
                            Some(fname)
                        } else {
                            None
                        }
                    })
                    .collect();
                sensor_files.sort();

                for fname in sensor_files {
                    let prefix = fname.strip_suffix("_input").unwrap_or(&fname);
                    let raw = read_sys_file(&format!("{}/{}", path.display(), fname));
                    let val: f64 = raw.parse().unwrap_or(0.0);

                    if val == 0.0 {
                        continue;
                    }

                    let label_file = format!("{}/{}_label", path.display(), prefix);
                    let label = read_sys_file(&label_file);
                    let label = if label.is_empty() {
                        prefix.to_string()
                    } else {
                        label
                    };

                    let (value, unit) = if prefix.starts_with("temp") {
                        (val / 1000.0, "°C".to_string())
                    } else if prefix.starts_with("fan") {
                        (val, "RPM".to_string())
                    } else if prefix.starts_with("in") {
                        (val / 1000.0, "V".to_string())
                    } else if prefix.starts_with("power") {
                        (val / 1000000.0, "W".to_string())
                    } else if prefix.starts_with("curr") {
                        (val / 1000.0, "A".to_string())
                    } else {
                        continue;
                    };

                    // Read high/critical thresholds for temps
                    let high = if prefix.starts_with("temp") {
                        read_sys_file(&format!("{}/{}_max", path.display(), prefix))
                            .parse::<f64>()
                            .ok()
                            .map(|v| v / 1000.0)
                    } else {
                        None
                    };

                    let critical = if prefix.starts_with("temp") {
                        read_sys_file(&format!("{}/{}_crit", path.display(), prefix))
                            .parse::<f64>()
                            .ok()
                            .map(|v| v / 1000.0)
                    } else {
                        None
                    };

                    readings.push(SensorReading {
                        label,
                        value,
                        unit,
                        high,
                        critical,
                    });
                }
            }

            if !readings.is_empty() {
                groups.push(SensorGroup { name: friendly_sensor_name(&name), readings });
            }
        }
    }

    groups.sort_by(|a, b| a.name.cmp(&b.name));
    groups
}

// ── Live System Stats ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    pub cpu_usage: f64,
    pub per_core_usage: Vec<f64>,
    pub memory: MemoryInfo,
    pub net_interfaces: Vec<NetStats>,
    pub top_processes: Vec<ProcessInfo>,
    pub load_average: [f64; 3],
    pub uptime: String,
    pub sensors: Vec<SensorGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetStats {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_rate: f64,
    pub tx_rate: f64,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f64,
    pub mem_bytes: u64,
    pub state: String,
}

struct CpuSnapshot {
    total: Vec<u64>,
    idle: Vec<u64>,
}

struct NetSnapshot {
    rx: HashMap<String, u64>,
    tx: HashMap<String, u64>,
    timestamp: std::time::Instant,
}

static PREV_CPU: OnceLock<Mutex<Option<CpuSnapshot>>> = OnceLock::new();
static PREV_NET: OnceLock<Mutex<Option<NetSnapshot>>> = OnceLock::new();

fn cpu_mutex() -> &'static Mutex<Option<CpuSnapshot>> {
    PREV_CPU.get_or_init(|| Mutex::new(None))
}

fn net_mutex() -> &'static Mutex<Option<NetSnapshot>> {
    PREV_NET.get_or_init(|| Mutex::new(None))
}

pub fn get_system_stats() -> SystemStats {
    let (cpu_usage, per_core_usage) = calc_cpu_usage();
    let memory = gather_memory_info();
    let net_interfaces = calc_net_stats();
    let top_processes = gather_top_processes(&memory);
    let load_average = gather_load_average();
    let uptime_str = read_sys_file("/proc/uptime");
    let uptime_secs: f64 = uptime_str
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    let uptime = format_uptime(uptime_secs as u64);
    let sensors = gather_sensors();

    SystemStats {
        cpu_usage,
        per_core_usage,
        memory,
        net_interfaces,
        top_processes,
        load_average,
        uptime,
        sensors,
    }
}

fn read_cpu_times() -> CpuSnapshot {
    let stat = read_sys_file("/proc/stat");
    let mut total = Vec::new();
    let mut idle = Vec::new();

    for line in stat.lines() {
        if line.starts_with("cpu") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 5 {
                continue;
            }
            let values: Vec<u64> = parts[1..]
                .iter()
                .filter_map(|s| s.parse().ok())
                .collect();
            let sum: u64 = values.iter().sum();
            let idle_val = values.get(3).copied().unwrap_or(0)
                + values.get(4).copied().unwrap_or(0); // idle + iowait
            total.push(sum);
            idle.push(idle_val);
        }
    }

    CpuSnapshot { total, idle }
}

fn calc_cpu_usage() -> (f64, Vec<f64>) {
    let current = read_cpu_times();
    let mut guard = cpu_mutex().lock().unwrap();

    let result = if let Some(prev) = guard.as_ref() {
        let mut per_core = Vec::new();
        let mut overall = 0.0;

        for i in 0..current.total.len() {
            if i < prev.total.len() {
                let total_delta = current.total[i].saturating_sub(prev.total[i]);
                let idle_delta = current.idle[i].saturating_sub(prev.idle[i]);
                let usage = if total_delta > 0 {
                    ((total_delta - idle_delta) as f64 / total_delta as f64) * 100.0
                } else {
                    0.0
                };
                if i == 0 {
                    overall = usage;
                } else {
                    per_core.push(usage);
                }
            }
        }

        (overall, per_core)
    } else {
        // First call — no delta available
        let cores = if current.total.len() > 1 {
            current.total.len() - 1
        } else {
            1
        };
        (0.0, vec![0.0; cores])
    };

    *guard = Some(current);
    result
}

fn calc_net_stats() -> Vec<NetStats> {
    let mut current_rx: HashMap<String, u64> = HashMap::new();
    let mut current_tx: HashMap<String, u64> = HashMap::new();
    let now = std::time::Instant::now();

    let net_dev = read_sys_file("/proc/net/dev");
    let mut iface_names = Vec::new();

    for line in net_dev.lines().skip(2) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 10 {
            continue;
        }
        let name = parts[0].trim_end_matches(':').to_string();
        if name == "lo" {
            continue;
        }
        let rx: u64 = parts[1].parse().unwrap_or(0);
        let tx: u64 = parts[9].parse().unwrap_or(0);
        current_rx.insert(name.clone(), rx);
        current_tx.insert(name.clone(), tx);
        iface_names.push(name);
    }

    let mut guard = net_mutex().lock().unwrap();
    let mut stats = Vec::new();

    for name in &iface_names {
        let rx = *current_rx.get(name).unwrap_or(&0);
        let tx = *current_tx.get(name).unwrap_or(&0);
        let state = read_sys_file(&format!("/sys/class/net/{}/operstate", name));

        let (rx_rate, tx_rate) = if let Some(prev) = guard.as_ref() {
            let elapsed = now.duration_since(prev.timestamp).as_secs_f64();
            if elapsed > 0.0 {
                let prev_rx = prev.rx.get(name).copied().unwrap_or(rx);
                let prev_tx = prev.tx.get(name).copied().unwrap_or(tx);
                (
                    rx.saturating_sub(prev_rx) as f64 / elapsed,
                    tx.saturating_sub(prev_tx) as f64 / elapsed,
                )
            } else {
                (0.0, 0.0)
            }
        } else {
            (0.0, 0.0)
        };

        stats.push(NetStats {
            name: name.clone(),
            rx_bytes: rx,
            tx_bytes: tx,
            rx_rate,
            tx_rate,
            state,
        });
    }

    // Sort: up first
    stats.sort_by(|a, b| {
        let a_up = a.state == "up";
        let b_up = b.state == "up";
        b_up.cmp(&a_up).then(a.name.cmp(&b.name))
    });

    *guard = Some(NetSnapshot {
        rx: current_rx,
        tx: current_tx,
        timestamp: now,
    });

    stats
}

fn gather_top_processes(mem: &MemoryInfo) -> Vec<ProcessInfo> {
    let mut procs = Vec::new();
    let page_size: u64 = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as u64 };
    let ticks_per_sec: f64 = unsafe { libc::sysconf(libc::_SC_CLK_TCK) as f64 };

    let uptime_str = read_sys_file("/proc/uptime");
    let uptime_secs: f64 = uptime_str
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1.0);

    // Read /proc/[pid]/stat for each process
    let proc_dir = match fs::read_dir("/proc") {
        Ok(d) => d,
        Err(_) => return procs,
    };

    for entry in proc_dir.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        let pid: u32 = match name_str.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };

        let stat_path = format!("/proc/{}/stat", pid);
        let stat_content = match fs::read_to_string(&stat_path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Parse /proc/[pid]/stat — name is in parens, fields after last ')'
        let comm_end = match stat_content.rfind(')') {
            Some(i) => i,
            None => continue,
        };
        let comm_start = match stat_content.find('(') {
            Some(i) => i,
            None => continue,
        };
        let proc_name = stat_content[comm_start + 1..comm_end].to_string();

        let rest = &stat_content[comm_end + 2..];
        let fields: Vec<&str> = rest.split_whitespace().collect();
        if fields.len() < 22 {
            continue;
        }

        let state_char = fields[0];
        let utime: u64 = fields[11].parse().unwrap_or(0);
        let stime: u64 = fields[12].parse().unwrap_or(0);
        let starttime: u64 = fields[19].parse().unwrap_or(0);
        let rss_pages: u64 = fields[21].parse().unwrap_or(0);

        let total_time = utime + stime;
        let proc_uptime = uptime_secs - (starttime as f64 / ticks_per_sec);
        let cpu_percent = if proc_uptime > 0.0 {
            (total_time as f64 / ticks_per_sec / proc_uptime) * 100.0
        } else {
            0.0
        };

        let mem_bytes = rss_pages * page_size;

        let state = match state_char {
            "R" => "Running",
            "S" => "Sleeping",
            "D" => "Disk Wait",
            "Z" => "Zombie",
            "T" => "Stopped",
            "I" => "Idle",
            _ => "Other",
        }
        .to_string();

        procs.push(ProcessInfo {
            pid,
            name: proc_name,
            cpu_percent,
            mem_bytes,
            state,
        });
    }

    // Sort by CPU desc, pick top 15
    procs.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal));
    procs.truncate(15);

    // Filter out kernel threads with 0 memory
    let _total_mem = mem.total_bytes;
    procs.retain(|p| p.mem_bytes > 0 || p.cpu_percent > 0.1);
    procs.truncate(10);

    procs
}

fn gather_load_average() -> [f64; 3] {
    let loadavg = read_sys_file("/proc/loadavg");
    let parts: Vec<&str> = loadavg.split_whitespace().collect();
    [
        parts.first().and_then(|s| s.parse().ok()).unwrap_or(0.0),
        parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0),
        parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0),
    ]
}
