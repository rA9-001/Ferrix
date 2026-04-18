<p align="center">
  <img src="static/ferrix-icon.png" alt="Ferrix" width="128" />
</p>

<h1 align="center">Ferrix</h1>

<p align="center">
  A Linux system utility built with Tauri v2, Svelte 5, and Rust.<br/>
  Offline-only. No telemetry. No network calls.
</p>

<p align="center">

![License](https://img.shields.io/badge/license-MIT-blue)
![Platform](https://img.shields.io/badge/platform-Linux-lightgrey)
![Rust](https://img.shields.io/badge/rust-2021-orange)
![Svelte](https://img.shields.io/badge/svelte-5-ff3e00)

</p>

---

## What it does

Ferrix is a single desktop app that handles the kind of system tasks you'd normally do across a dozen terminal commands. Everything runs locally — no accounts, no servers, no background services.

**Modules:**

- **System Cleanup** — scan and remove unused caches, logs, and temp files
- **Disk Usage** — partition overview with per-directory drill-down
- **Startup Manager** — manage XDG autostart entries and systemd user services
- **System Monitor** — live CPU, memory, network I/O with rolling history graphs
- **Performance Optimizer** — sysctl tweaks for CPU, memory, storage, network, and kernel; view current vs. recommended values
- **Security Hardening** — toggle sysctl-based protections for network, kernel, and filesystem; changes persist via `/etc/sysctl.d/`
- **Firewall Manager** — UFW frontend for rules, policies, and status
- **Network Monitor** — interfaces, active connections, DNS, listening ports, live traffic
- **Service Manager** — browse, control, and inspect systemd services with inline logs
- **Permissions Auditor** — find SUID/SGID binaries, world-writable files, and home directory issues
- **Package Manager** — curated app catalog (~70+ packages) with multi-distro support and batch install
- **System Updates** — check and apply updates from your package manager and Flatpak (AUR helper detection on Arch)
- **Log Viewer** — query journalctl with filters for priority, unit, boot, time range, and grep

---

## Distro support

Auto-detects your distribution and adapts package commands accordingly.

| Distro Family | Package Manager | Tested |
|---|---|---|
| Arch / CachyOS / Manjaro / EndeavourOS | pacman (+ yay/paru for AUR) | ✅ |
| Debian / Ubuntu / Pop!_OS / Mint | apt | ○ |
| Fedora / RHEL / CentOS | dnf | ○ |
| openSUSE | zypper | ○ |
| Void Linux | xbps | ○ |
| Alpine | apk | ○ |

✅ = tested &nbsp; ○ = should work, not yet tested

---

## Building from source

**Requirements:** [Rust](https://rustup.rs/) (stable), [Node.js](https://nodejs.org/) (v18+), [Tauri v2 system dependencies](https://v2.tauri.app/start/prerequisites/)

```bash
git clone https://github.com/rA9-001/Ferrix.git
cd Ferrix
npm install
npm run tauri dev      # development
npm run tauri build    # production (output: src-tauri/target/release/)
```

---

## How it works

- **Frontend:** Svelte 5 with SvelteKit (static adapter)
- **Backend:** Rust via Tauri v2 command system
- **Privilege escalation:** `pkexec` — only triggered when an action actually needs root
- **No network calls** — reads from `/proc`, `/sys`, and local system commands

---

## Project structure

```
src/                    # Svelte frontend
├── lib/                # Feature components (13 modules)
├── routes/             # SvelteKit pages
├── app.html
└── global.css

src-tauri/              # Rust backend
├── src/
│   ├── lib.rs          # Command registration, rate limiting
│   ├── audit_log.rs    # Append-only audit log
│   ├── cleaner.rs      # System cleanup
│   ├── disk.rs         # Partition & directory analysis
│   ├── distro.rs       # Distribution detection
│   ├── firewall.rs     # UFW wrapper
│   ├── hardening.rs    # Sysctl security hardening
│   ├── hardware.rs     # Hardware info & live stats
│   ├── logs.rs         # journalctl queries
│   ├── network.rs      # Network interfaces & traffic
│   ├── optimizer.rs    # Performance tweaks
│   ├── packages.rs     # Package catalog & installer
│   ├── permissions.rs  # SUID/SGID/world-writable audit
│   ├── services.rs     # systemd management
│   ├── startup.rs      # Autostart entries
│   └── updates.rs      # System updates
├── Cargo.toml
└── tauri.conf.json
```

---

## Security

- Fully offline — no telemetry, no analytics, nothing phones home
- Root access only requested when actually needed (via `pkexec`)
- All user inputs validated against allow-list patterns before reaching system commands
- Privileged actions logged to `$XDG_STATE_HOME/ferrix/audit.log`
- Privileged commands are rate-limited
- Release builds use LTO, single codegen unit, overflow checks, and symbol stripping
- Frontend locked down with CSP and frozen prototypes

---

## Contributing

Ferrix is open source under the [MIT license](LICENSE) — fork it, modify it, use it however you want.

Direct push access to this repo is restricted. If you want to report a bug, suggest a feature, or discuss a change, [open an issue](https://github.com/rA9-001/Ferrix/issues).

---

## IDE setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
