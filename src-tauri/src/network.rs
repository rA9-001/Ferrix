use serde::{Deserialize, Serialize};
use crate::sysenv::system_command;
use std::collections::HashMap;
use std::fs;

// ── Data Structures ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSnapshot {
    pub interfaces: Vec<InterfaceInfo>,
    pub connections: Vec<Connection>,
    pub dns: DnsInfo,
    pub traffic: Vec<InterfaceTraffic>,
    pub listening: Vec<ListeningPort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceInfo {
    pub name: String,
    pub state: String,
    pub mac: String,
    pub ipv4: String,
    pub ipv6: String,
    pub mtu: u32,
    pub speed: String,
    pub interface_type: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub rx_dropped: u64,
    pub tx_dropped: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceTraffic {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub protocol: String,
    pub state: String,
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: String,
    pub remote_port: u16,
    pub process: String,
    pub pid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListeningPort {
    pub protocol: String,
    pub port: u16,
    pub address: String,
    pub process: String,
    pub pid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsInfo {
    pub servers: Vec<String>,
    pub search_domains: Vec<String>,
    pub resolver: String,
    pub gateway: String,
    pub public_ip: String,
}

// ── Interface Info ───────────────────────────────────────────────

fn get_interfaces() -> Vec<InterfaceInfo> {
    let mut interfaces = Vec::new();
    let net_dir = "/sys/class/net";

    let entries = match fs::read_dir(net_dir) {
        Ok(e) => e,
        Err(_) => return interfaces,
    };

    // Parse /proc/net/dev for traffic counters
    let dev_stats = parse_proc_net_dev();
    // Get IP addresses via `ip -j addr`
    let ip_info = get_ip_addresses();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "lo" {
            continue;
        }

        let base = format!("{}/{}", net_dir, name);
        let state = read_sys_file(&format!("{}/operstate", base));
        let mac = read_sys_file(&format!("{}/address", base));
        let mtu: u32 = read_sys_file(&format!("{}/mtu", base)).parse().unwrap_or(0);
        let speed_raw = read_sys_file(&format!("{}/speed", base));
        let speed = if speed_raw.is_empty() || speed_raw == "-1" {
            "—".to_string()
        } else {
            format!("{} Mbps", speed_raw)
        };

        let iface_type = detect_interface_type(&name, &base);
        let (ipv4, ipv6) = ip_info.get(&name).cloned().unwrap_or_default();
        let stats = dev_stats.get(&name);

        interfaces.push(InterfaceInfo {
            name: name.clone(),
            state,
            mac,
            ipv4,
            ipv6,
            mtu,
            speed,
            interface_type: iface_type,
            rx_bytes: stats.map(|s| s.0).unwrap_or(0),
            tx_bytes: stats.map(|s| s.1).unwrap_or(0),
            rx_packets: stats.map(|s| s.2).unwrap_or(0),
            tx_packets: stats.map(|s| s.3).unwrap_or(0),
            rx_errors: stats.map(|s| s.4).unwrap_or(0),
            tx_errors: stats.map(|s| s.5).unwrap_or(0),
            rx_dropped: stats.map(|s| s.6).unwrap_or(0),
            tx_dropped: stats.map(|s| s.7).unwrap_or(0),
        });
    }

    // Sort: up interfaces first, then by name
    interfaces.sort_by(|a, b| {
        let a_up = a.state == "up";
        let b_up = b.state == "up";
        b_up.cmp(&a_up).then(a.name.cmp(&b.name))
    });

    interfaces
}

fn detect_interface_type(name: &str, sys_path: &str) -> String {
    if name.starts_with("wl") || name.starts_with("wlan") {
        "WiFi".to_string()
    } else if name.starts_with("en") || name.starts_with("eth") {
        "Ethernet".to_string()
    } else if name.starts_with("br") {
        "Bridge".to_string()
    } else if name.starts_with("docker") || name.starts_with("veth") {
        "Docker".to_string()
    } else if name.starts_with("vir") || name.starts_with("virbr") {
        "Virtual".to_string()
    } else if name.starts_with("tun") || name.starts_with("tap") {
        "VPN/Tunnel".to_string()
    } else if name.starts_with("wg") {
        "WireGuard".to_string()
    } else if name.starts_with("tailscale") || name.starts_with("ts") {
        "Tailscale".to_string()
    } else if name.starts_with("waydroid") {
        "Waydroid".to_string()
    } else if fs::metadata(format!("{}/wireless", sys_path)).is_ok() {
        "WiFi".to_string()
    } else {
        "Network".to_string()
    }
}

// ── Traffic Counters (for real-time delta) ───────────────────────

fn parse_proc_net_dev() -> HashMap<String, (u64, u64, u64, u64, u64, u64, u64, u64)> {
    let mut map = HashMap::new();
    let content = match fs::read_to_string("/proc/net/dev") {
        Ok(c) => c,
        Err(_) => return map,
    };

    for line in content.lines().skip(2) {
        let line = line.trim();
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 17 {
            continue;
        }
        let name = parts[0].trim_end_matches(':');
        if name == "lo" {
            continue;
        }

        let rx_bytes: u64 = parts[1].parse().unwrap_or(0);
        let rx_packets: u64 = parts[2].parse().unwrap_or(0);
        let rx_errors: u64 = parts[3].parse().unwrap_or(0);
        let rx_dropped: u64 = parts[4].parse().unwrap_or(0);
        let tx_bytes: u64 = parts[9].parse().unwrap_or(0);
        let tx_packets: u64 = parts[10].parse().unwrap_or(0);
        let tx_errors: u64 = parts[11].parse().unwrap_or(0);
        let tx_dropped: u64 = parts[12].parse().unwrap_or(0);

        map.insert(
            name.to_string(),
            (rx_bytes, tx_bytes, rx_packets, tx_packets, rx_errors, tx_errors, rx_dropped, tx_dropped),
        );
    }
    map
}

pub fn get_traffic_snapshot() -> Vec<InterfaceTraffic> {
    parse_proc_net_dev()
        .into_iter()
        .map(|(name, (rx_b, tx_b, rx_p, tx_p, ..))| InterfaceTraffic {
            name,
            rx_bytes: rx_b,
            tx_bytes: tx_b,
            rx_packets: rx_p,
            tx_packets: tx_p,
        })
        .collect()
}

// ── Active Connections ───────────────────────────────────────────

fn get_connections() -> Vec<Connection> {
    let output = system_command("ss")
        .args(["-tunap", "--no-header"])
        .output();

    let out = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Vec::new(),
    };

    let mut connections = Vec::new();

    for line in out.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }

        let protocol = parts[0].to_string();
        let state = parts[1].to_string();
        let local_full = parts[4];
        let remote_full = if parts.len() > 5 { parts[5] } else { "" };

        let (local_addr, local_port) = parse_addr_port(local_full);
        let (remote_addr, remote_port) = parse_addr_port(remote_full);

        // Skip LISTEN entries (they go to listening ports)
        if state == "LISTEN" {
            continue;
        }

        // Parse process info: users:(("name",pid=123,fd=4))
        let (process, pid) = if parts.len() > 6 {
            parse_process_info(parts[6])
        } else {
            (String::new(), 0)
        };

        connections.push(Connection {
            protocol,
            state,
            local_addr,
            local_port,
            remote_addr,
            remote_port,
            process,
            pid,
        });
    }

    connections
}

// ── Listening Ports ──────────────────────────────────────────────

fn get_listening_ports() -> Vec<ListeningPort> {
    let output = system_command("ss")
        .args(["-tulnp", "--no-header"])
        .output();

    let out = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Vec::new(),
    };

    let mut ports = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for line in out.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }

        let protocol = parts[0].to_string();
        let local_full = parts[4];
        let (address, port) = parse_addr_port(local_full);

        let key = format!("{}:{}", protocol, port);
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);

        let (process, pid) = if parts.len() > 6 {
            parse_process_info(parts[6])
        } else {
            (String::new(), 0)
        };

        ports.push(ListeningPort {
            protocol,
            port,
            address,
            process,
            pid,
        });
    }

    ports.sort_by_key(|p| p.port);
    ports
}

// ── DNS Info ─────────────────────────────────────────────────────

fn get_dns_info() -> DnsInfo {
    let mut servers = Vec::new();
    let mut search_domains = Vec::new();
    let mut resolver = String::new();

    // Try systemd-resolve --status first
    let resolve_out = system_command("resolvectl")
        .args(["status", "--no-pager"])
        .output();

    if let Ok(out) = &resolve_out {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            resolver = "systemd-resolved".to_string();
            for line in stdout.lines() {
                let line = line.trim();
                if line.starts_with("DNS Servers:") || line.starts_with("Current DNS Server:") {
                    if let Some(val) = line.split(':').nth(1) {
                        for s in val.split_whitespace() {
                            if !servers.contains(&s.to_string()) {
                                servers.push(s.to_string());
                            }
                        }
                    }
                } else if line.starts_with("DNS Domain:") {
                    if let Some(val) = line.split(':').nth(1) {
                        for d in val.split_whitespace() {
                            if !search_domains.contains(&d.to_string()) {
                                search_domains.push(d.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // Fallback: parse /etc/resolv.conf
    if servers.is_empty() {
        if let Ok(content) = fs::read_to_string("/etc/resolv.conf") {
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with('#') {
                    continue;
                }
                if let Some(rest) = line.strip_prefix("nameserver") {
                    let server = rest.trim().to_string();
                    if !servers.contains(&server) {
                        servers.push(server);
                    }
                } else if let Some(rest) = line.strip_prefix("search") {
                    for domain in rest.split_whitespace() {
                        search_domains.push(domain.to_string());
                    }
                }
            }
            if resolver.is_empty() {
                resolver = "resolv.conf".to_string();
            }
        }
    }

    // Get default gateway
    let gateway = get_default_gateway();

    DnsInfo {
        servers,
        search_domains,
        resolver,
        gateway,
        public_ip: String::new(), // filled async on frontend if needed
    }
}

fn get_default_gateway() -> String {
    let output = system_command("ip")
        .args(["-j", "route", "show", "default"])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // Parse JSON: [{"gateway":"192.168.1.1",...}]
            if let Some(start) = stdout.find("\"gateway\":\"") {
                let rest = &stdout[start + 11..];
                if let Some(end) = rest.find('"') {
                    return rest[..end].to_string();
                }
            }
        }
    }

    // Fallback: parse plain `ip route`
    let output = system_command("ip")
        .args(["route", "show", "default"])
        .output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        // "default via 192.168.1.1 dev eth0 ..."
        for part in stdout.split_whitespace() {
            if part.contains('.') && !part.starts_with("default") {
                return part.to_string();
            }
        }
    }

    String::new()
}

fn get_ip_addresses() -> HashMap<String, (String, String)> {
    let mut map: HashMap<String, (String, String)> = HashMap::new();

    let output = system_command("ip")
        .args(["-j", "addr", "show"])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // Simple JSON parsing for ifname and addr_info
            // We'll parse it manually to avoid adding a JSON dependency
            parse_ip_json(&stdout, &mut map);
        }
    }

    map
}

fn parse_ip_json(json: &str, map: &mut HashMap<String, (String, String)>) {
    // Parse the ip -j addr output to extract interface names and IPs
    // Format: [{"ifname":"eth0","addr_info":[{"family":"inet","local":"1.2.3.4",...},{"family":"inet6","local":"::1",...}]}]
    let mut current_ifname = String::new();
    let mut current_ipv4 = String::new();
    let mut current_ipv6 = String::new();
    let mut in_addr_info = false;
    let mut current_family = String::new();

    for line_raw in json.split(',') {
        let line = line_raw.trim();

        if let Some(val) = extract_json_string(line, "ifname") {
            if !current_ifname.is_empty() {
                map.insert(
                    current_ifname.clone(),
                    (current_ipv4.clone(), current_ipv6.clone()),
                );
            }
            current_ifname = val;
            current_ipv4 = String::new();
            current_ipv6 = String::new();
        }

        if line.contains("\"addr_info\"") {
            in_addr_info = true;
        }

        if in_addr_info {
            if let Some(val) = extract_json_string(line, "family") {
                current_family = val;
            }
            if let Some(val) = extract_json_string(line, "local") {
                if current_family == "inet" && current_ipv4.is_empty() {
                    current_ipv4 = val;
                } else if current_family == "inet6" && current_ipv6.is_empty() && !val.starts_with("fe80") {
                    current_ipv6 = val;
                }
            }
        }
    }

    if !current_ifname.is_empty() {
        map.insert(current_ifname, (current_ipv4, current_ipv6));
    }
}

fn extract_json_string(text: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":\"", key);
    if let Some(start) = text.find(&pattern) {
        let rest = &text[start + pattern.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

// ── Helpers ──────────────────────────────────────────────────────

fn parse_addr_port(addr_str: &str) -> (String, u16) {
    // Handle IPv6: [::1]:8080 or ::1:8080
    // Handle IPv4: 192.168.1.1:8080 or *:8080
    if let Some(bracket_end) = addr_str.rfind(']') {
        // IPv6 with brackets: [::1]:port
        let addr = addr_str[1..bracket_end].to_string();
        let port_str = &addr_str[bracket_end + 2..];
        let port: u16 = port_str.parse().unwrap_or(0);
        return (addr, port);
    }

    // Find the last colon for port separator
    if let Some(last_colon) = addr_str.rfind(':') {
        let addr_part = &addr_str[..last_colon];
        let port_str = &addr_str[last_colon + 1..];
        let port: u16 = port_str.parse().unwrap_or(0);
        let addr = if addr_part == "*" || addr_part == "0.0.0.0" {
            "0.0.0.0".to_string()
        } else {
            addr_part.to_string()
        };
        return (addr, port);
    }

    (addr_str.to_string(), 0)
}

fn parse_process_info(info: &str) -> (String, u32) {
    // Format: users:(("processname",pid=12345,fd=6))
    let mut process = String::new();
    let mut pid: u32 = 0;

    if let Some(start) = info.find("((\"") {
        let rest = &info[start + 3..];
        if let Some(end) = rest.find('"') {
            process = rest[..end].to_string();
        }
    }

    if let Some(start) = info.find("pid=") {
        let rest = &info[start + 4..];
        let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        pid = num_str.parse().unwrap_or(0);
    }

    (process, pid)
}

fn read_sys_file(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_default()
        .trim()
        .to_string()
}

// ── Public API ───────────────────────────────────────────────────

pub fn get_network_snapshot() -> NetworkSnapshot {
    let interfaces = get_interfaces();
    let connections = get_connections();
    let listening = get_listening_ports();
    let dns = get_dns_info();
    let traffic = get_traffic_snapshot();

    NetworkSnapshot {
        interfaces,
        connections,
        dns,
        traffic,
        listening,
    }
}
