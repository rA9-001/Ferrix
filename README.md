# Linux Utility App

An all-in-one desktop application for managing, monitoring, and optimizing your Linux system. Built with **Tauri v2**, **Svelte 5**, and **Rust** — fully offline, local, and privacy-respecting.

![License](https://img.shields.io/badge/license-MIT-blue)
![Platform](https://img.shields.io/badge/platform-Linux-lightgrey)
![Rust](https://img.shields.io/badge/rust-2021-orange)
![Svelte](https://img.shields.io/badge/svelte-5-ff3e00)

---

## Features

### 🗑️ System Cleanup
Scan and remove unused files, caches, and logs. Detects cleanup targets per distro and lets you selectively clean with a detailed results console.

### 📊 Disk Usage
Visual overview of all mounted partitions with usage bars and color-coded thresholds. Drill into directories with lazy-loaded space analysis and per-folder breakdowns.

### ⚙️ Startup Manager
Manage XDG autostart entries and systemd user services. Toggle, add, or remove startup items with full visibility into what runs at boot.

### 🖥️ System Monitor
Live hardware dashboard with configurable polling (0.5s–10s). Tracks CPU, memory, network I/O with 60-point rolling history. Shows detailed info for CPU, GPU, disks, network interfaces, and temperature sensors.

### ⚡ Performance Optimizer
Kernel and system performance tweaks across CPU, memory, storage, network, and kernel categories. View current vs. recommended values, apply individually, and restore defaults at any time.

### 🛡️ Security Hardening
Sysctl-based security hardening with toggles for network, kernel, and filesystem protections. Changes persist via `/etc/sysctl.d/` and can be individually reverted.

### 🔥 Firewall Manager
Full UFW frontend — view status and rules, add/delete rules with direction, port, protocol, source IP, and comments. Toggle the firewall on/off and set default policies.

### 🌐 Network Monitor
Real-time network snapshot: interfaces with bandwidth stats, active connections, DNS config, listening ports. Sortable and filterable with live traffic calculation between snapshots.

### 🔧 Service Manager
Browse all systemd services with status filtering (active/inactive/failed). Start, stop, restart, enable, or disable services. Inline log viewer per service.

### 🔍 Permissions Auditor
Security audit for SUID/SGID binaries, world-writable files, and home directory permission issues. Results are categorized by severity (critical/warning/info) with explanations of each detection.

### 📦 Package Manager
Curated catalog of ~70+ applications across 16 categories with multi-distro support (pacman, apt, dnf, zypper, xbps, apk, flatpak). Batch install/remove with a single password prompt and live console output.

### ⬆️ System Updates
Check for available updates across your native package manager and Flatpak — no root required for checking. On Arch-based distros, detects AUR helpers (yay/paru) for seamless AUR updates. Searchable update list with version comparison and streaming update output.

### 📋 Log Viewer
Browse journalctl logs with filters for priority level, systemd unit, boot session, time range, and grep pattern. Color-coded severity badges with expandable entry details.

---

## Distro Support

The app auto-detects your distribution and adapts accordingly:

| Distro Family | Package Manager | Tested |
|---|---|---|
| Arch / CachyOS / Manjaro / EndeavourOS | pacman (+ yay/paru for AUR) | ✅ |
| Debian / Ubuntu / Pop!_OS / Mint | apt | ○ |
| Fedora / RHEL / CentOS | dnf | ○ |
| openSUSE | zypper | ○ |
| Void Linux | xbps | ○ |
| Alpine | apk | ○ |

✅ = tested &nbsp; ○ = supported but not yet tested

---

## Tech Stack

- **Frontend:** Svelte 5 (SvelteKit + static adapter)
- **Backend:** Rust (Tauri v2)
- **IPC:** Tauri command system with event streaming for live output
- **Privilege escalation:** `pkexec` (Polkit) — only when a root action is performed
- **No network calls** — everything runs locally via system commands and `/proc`/`/sys` reads

---

## Building from Source

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) (v18+)
- [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/) (system dependencies for your distro)

### Build & Run

```bash
# Clone the repo
git clone https://github.com/rA9-001/Linux-Utility-App.git
cd Linux-Utility-App

# Install frontend dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

The production binary will be in `src-tauri/target/release/`.

---

## Project Structure

```
src/                    # Svelte frontend
├── lib/                # Feature components (13 modules)
├── routes/             # SvelteKit pages
├── app.html            # HTML shell
└── global.css          # Global styles

src-tauri/              # Rust backend
├── src/
│   ├── lib.rs          # Tauri command registration & rate limiting
│   ├── audit_log.rs    # Append-only audit log for privileged actions
│   ├── cleaner.rs      # System cleanup scanning & execution
│   ├── disk.rs         # Partition & directory space analysis
│   ├── distro.rs       # Distribution detection
│   ├── firewall.rs     # UFW wrapper with input validation
│   ├── hardening.rs    # Sysctl security hardening
│   ├── hardware.rs     # Hardware info & live system stats
│   ├── logs.rs         # journalctl log querying
│   ├── network.rs      # Network interfaces, connections, traffic
│   ├── optimizer.rs    # Kernel/system performance tweaks
│   ├── packages.rs     # Multi-distro package catalog & installer
│   ├── permissions.rs  # SUID/SGID/world-writable auditing
│   ├── services.rs     # systemd service management
│   ├── startup.rs      # Autostart entry management
│   └── updates.rs      # System update checking & applying
├── audit.toml          # cargo-audit advisory config
├── deny.toml           # cargo-deny license/advisory config
├── Cargo.toml
└── tauri.conf.json
```

---

## Security & Privacy

- **Fully offline** — no telemetry, no analytics, no network requests
- **Minimal permissions** — root access is only requested via `pkexec` when an action explicitly needs it (installing packages, applying system tweaks, managing services, etc.)
- **Read-only by default** — browsing hardware info, logs, disk usage, and network status requires no elevated privileges
- **Strict input validation** — all user-facing inputs (ports, IPs, package names, paths, unit names) are validated against allow-list patterns before reaching system commands
- **Audit logging** — every privileged action is logged to `$XDG_STATE_HOME/linux-utility-app/audit.log` with timestamps
- **Rate limiting** — privileged commands are rate-limited to prevent accidental rapid-fire execution
- **Hardened builds** — release binaries use LTO, single codegen unit, overflow checks, and symbol stripping
- **CSP & prototype freeze** — frontend is locked down with a strict Content Security Policy and frozen prototypes
- **Open source** — audit the entire codebase yourself

---

## License

[MIT](LICENSE)

---

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
