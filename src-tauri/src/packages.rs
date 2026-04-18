use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub icon: String,
    pub packages: PackageNames,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageNames {
    pub pacman: Option<String>,
    pub apt: Option<String>,
    pub dnf: Option<String>,
    pub zypper: Option<String>,
    pub xbps: Option<String>,
    pub apk: Option<String>,
    pub flatpak: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    pub package_id: String,
    pub success: bool,
    pub output: String,
    pub method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageStatus {
    pub id: String,
    pub installed: bool,
}

pub fn get_package_catalog() -> Vec<PackageDef> {
    vec![
        // Web Browsers
        pkg("firefox", "Firefox", "Open-source web browser by Mozilla", "Web Browsers", "🦊",
            pn(Some("firefox"), Some("firefox"), Some("firefox"), Some("firefox"), Some("firefox"), Some("firefox"), None)),
        pkg("chromium", "Chromium", "Open-source browser by Google", "Web Browsers", "🌐",
            pn(Some("chromium"), Some("chromium-browser"), Some("chromium"), Some("chromium"), Some("chromium"), Some("chromium"), None)),
        pkg("brave", "Brave", "Privacy-focused browser with ad blocker", "Web Browsers", "🦁",
            pn(Some("brave-browser"), None, None, None, None, None, Some("com.brave.Browser"))),
        pkg("librewolf", "LibreWolf", "Privacy-focused Firefox fork", "Web Browsers", "🐺",
            pn(Some("librewolf"), None, None, None, None, None, Some("io.gitlab.librewolf-community"))),
        pkg("tor-browser", "Tor Browser", "Browse anonymously via the Tor network", "Web Browsers", "🧅",
            pn(Some("torbrowser-launcher"), Some("torbrowser-launcher"), None, None, Some("torbrowser-launcher"), None, Some("com.github.nickvision.torbrowser"))),

        // Communication
        pkg("discord", "Discord", "Voice, video, and text chat platform", "Communication", "💬",
            pn(Some("discord"), None, None, None, None, None, Some("com.discordapp.Discord"))),
        pkg("telegram", "Telegram", "Fast cloud-based messaging app", "Communication", "📨",
            pn(Some("telegram-desktop"), Some("telegram-desktop"), None, None, Some("telegram-desktop"), None, Some("org.telegram.desktop"))),
        pkg("signal", "Signal", "Encrypted messaging and calls", "Communication", "🔒",
            pn(Some("signal-desktop"), None, None, None, None, None, Some("org.signal.Signal"))),
        pkg("thunderbird", "Thunderbird", "Email client by Mozilla", "Communication", "📧",
            pn(Some("thunderbird"), Some("thunderbird"), Some("thunderbird"), Some("MozillaThunderbird"), Some("thunderbird"), None, None)),
        pkg("slack", "Slack", "Team communication platform", "Communication", "💼",
            pn(Some("slack-desktop"), None, None, None, None, None, Some("com.slack.Slack"))),
        pkg("element", "Element", "Matrix-based decentralized chat", "Communication", "🟢",
            pn(Some("element-desktop"), None, None, None, None, None, Some("im.riot.Riot"))),

        // Media
        pkg("vlc", "VLC", "Versatile media player", "Media", "🎬",
            pn(Some("vlc"), Some("vlc"), Some("vlc"), Some("vlc"), Some("vlc"), Some("vlc"), None)),
        pkg("mpv", "mpv", "Lightweight and powerful media player", "Media", "▶️",
            pn(Some("mpv"), Some("mpv"), Some("mpv"), Some("mpv"), Some("mpv"), Some("mpv"), None)),
        pkg("obs", "OBS Studio", "Video recording and live streaming", "Media", "📹",
            pn(Some("obs-studio"), Some("obs-studio"), None, None, None, None, Some("com.obsproject.Studio"))),
        pkg("spotify", "Spotify", "Music streaming service", "Media", "🎵",
            pn(Some("spotify-launcher"), None, None, None, None, None, Some("com.spotify.Client"))),
        pkg("audacity", "Audacity", "Audio editor and recorder", "Media", "🎙️",
            pn(Some("audacity"), Some("audacity"), Some("audacity"), Some("audacity"), Some("audacity"), None, None)),
        pkg("kdenlive", "Kdenlive", "Video editor by KDE", "Media", "🎞️",
            pn(Some("kdenlive"), Some("kdenlive"), None, None, None, None, Some("org.kde.kdenlive"))),
        pkg("handbrake", "HandBrake", "Video transcoder", "Media", "🔄",
            pn(Some("handbrake"), Some("handbrake"), None, None, Some("HandBrake"), None, Some("fr.handbrake.ghb"))),

        // Gaming
        pkg("steam", "Steam", "Gaming platform by Valve", "Gaming", "🎮",
            pn(Some("steam"), Some("steam-installer"), None, None, None, None, Some("com.valvesoftware.Steam"))),
        pkg("lutris", "Lutris", "Open gaming platform for Linux", "Gaming", "🕹️",
            pn(Some("lutris"), Some("lutris"), Some("lutris"), None, None, None, Some("net.lutris.Lutris"))),
        pkg("heroic", "Heroic", "Epic/GOG/Amazon game launcher", "Gaming", "⚔️",
            pn(Some("heroic-games-launcher"), None, None, None, None, None, Some("com.heroicgameslauncher.hgl"))),
        pkg("mangohud", "MangoHUD", "FPS and performance overlay", "Gaming", "📊",
            pn(Some("mangohud"), Some("mangohud"), Some("mangohud"), None, None, None, None)),
        pkg("gamemode", "GameMode", "Optimize system for gaming", "Gaming", "🚀",
            pn(Some("gamemode"), Some("gamemode"), Some("gamemode"), None, None, None, None)),

        // Dev: Editors
        pkg("vscode", "VS Code", "Popular code editor by Microsoft", "Dev: Editors", "📝",
            pn(Some("visual-studio-code-bin"), None, None, None, None, None, Some("com.visualstudio.code"))),
        pkg("vscodium", "VSCodium", "VS Code without telemetry", "Dev: Editors", "📝",
            pn(Some("vscodium"), None, None, None, None, None, Some("com.vscodium.codium"))),
        pkg("neovim", "Neovim", "Hyperextensible terminal-based editor", "Dev: Editors", "📟",
            pn(Some("neovim"), Some("neovim"), Some("neovim"), Some("neovim"), Some("neovim"), Some("neovim"), None)),
        pkg("vim", "Vim", "Classic terminal text editor", "Dev: Editors", "📟",
            pn(Some("vim"), Some("vim"), Some("vim"), Some("vim"), Some("vim"), Some("vim"), None)),
        pkg("sublime", "Sublime Text", "Fast and elegant code editor", "Dev: Editors", "🖊️",
            pn(None, None, None, None, None, None, Some("com.sublimetext.three"))),
        pkg("kate", "Kate", "Advanced text editor by KDE", "Dev: Editors", "📄",
            pn(Some("kate"), Some("kate"), Some("kate"), Some("kate"), None, None, None)),

        // Dev: Tools
        pkg("git", "Git", "Distributed version control system", "Dev: Tools", "🔀",
            pn(Some("git"), Some("git"), Some("git"), Some("git"), Some("git"), Some("git"), None)),
        pkg("docker", "Docker", "Container platform", "Dev: Tools", "🐳",
            pn(Some("docker"), Some("docker.io"), Some("docker-ce"), Some("docker"), Some("docker"), Some("docker"), None)),
        pkg("virtualbox", "VirtualBox", "x86 virtualization", "Dev: Tools", "📦",
            pn(Some("virtualbox"), Some("virtualbox"), None, None, None, None, None)),
        pkg("wireshark", "Wireshark", "Network protocol analyzer", "Dev: Tools", "🦈",
            pn(Some("wireshark-qt"), Some("wireshark"), Some("wireshark"), Some("wireshark"), Some("wireshark"), None, None)),
        pkg("meld", "Meld", "Visual diff and merge tool", "Dev: Tools", "🔍",
            pn(Some("meld"), Some("meld"), Some("meld"), Some("meld"), Some("meld"), None, None)),

        // Dev: Languages
        pkg("python", "Python 3", "Popular programming language", "Dev: Languages", "🐍",
            pn(Some("python"), Some("python3"), Some("python3"), Some("python3"), Some("python3"), Some("python3"), None)),
        pkg("nodejs", "Node.js", "JavaScript runtime", "Dev: Languages", "🟩",
            pn(Some("nodejs"), Some("nodejs"), Some("nodejs"), Some("nodejs"), Some("nodejs"), Some("nodejs"), None)),
        pkg("rust", "Rust", "Systems programming language", "Dev: Languages", "🦀",
            pn(Some("rust"), Some("rustc"), Some("rust"), Some("rust"), Some("rust"), Some("rust"), None)),
        pkg("go", "Go", "Programming language by Google", "Dev: Languages", "🐹",
            pn(Some("go"), Some("golang"), Some("golang"), Some("go"), Some("go"), Some("go"), None)),
        pkg("java", "Java (OpenJDK)", "General-purpose programming language", "Dev: Languages", "☕",
            pn(Some("jdk-openjdk"), Some("default-jdk"), Some("java-latest-openjdk"), Some("java-17-openjdk"), Some("openjdk17"), Some("openjdk17"), None)),
        pkg("lua", "Lua", "Lightweight scripting language", "Dev: Languages", "🌙",
            pn(Some("lua"), Some("lua5.4"), Some("lua"), Some("lua54"), Some("lua"), Some("lua"), None)),
        pkg("ruby", "Ruby", "Dynamic object-oriented language", "Dev: Languages", "💎",
            pn(Some("ruby"), Some("ruby"), Some("ruby"), Some("ruby"), Some("ruby"), Some("ruby"), None)),
        pkg("php", "PHP", "Server-side scripting language", "Dev: Languages", "🐘",
            pn(Some("php"), Some("php"), Some("php"), Some("php8"), Some("php"), Some("php"), None)),
        pkg("zig", "Zig", "Systems language with no hidden control flow", "Dev: Languages", "⚡",
            pn(Some("zig"), None, None, None, None, None, None)),
        pkg("kotlin", "Kotlin", "Modern JVM language by JetBrains", "Dev: Languages", "🟣",
            pn(Some("kotlin"), None, None, None, None, None, None)),

        // Creative
        pkg("gimp", "GIMP", "Image editor", "Creative", "🎨",
            pn(Some("gimp"), Some("gimp"), Some("gimp"), Some("gimp"), Some("gimp"), Some("gimp"), None)),
        pkg("inkscape", "Inkscape", "Vector graphics editor", "Creative", "✏️",
            pn(Some("inkscape"), Some("inkscape"), Some("inkscape"), Some("inkscape"), Some("inkscape"), None, None)),
        pkg("blender", "Blender", "3D creation suite", "Creative", "🧊",
            pn(Some("blender"), Some("blender"), Some("blender"), Some("blender"), Some("blender"), None, None)),
        pkg("krita", "Krita", "Digital painting app", "Creative", "🖌️",
            pn(Some("krita"), Some("krita"), Some("krita"), None, None, None, Some("org.kde.krita"))),

        // Office
        pkg("libreoffice", "LibreOffice", "Full office suite", "Office", "📊",
            pn(Some("libreoffice-fresh"), Some("libreoffice"), Some("libreoffice"), Some("libreoffice"), Some("libreoffice"), Some("libreoffice"), None)),
        pkg("obsidian", "Obsidian", "Markdown knowledge base", "Office", "💎",
            pn(Some("obsidian"), None, None, None, None, None, Some("md.obsidian.Obsidian"))),
        pkg("okular", "Okular", "Document viewer by KDE", "Office", "📖",
            pn(Some("okular"), Some("okular"), Some("okular"), Some("okular"), None, None, None)),
        pkg("calibre", "Calibre", "E-book management", "Office", "📚",
            pn(Some("calibre"), Some("calibre"), Some("calibre"), Some("calibre"), Some("calibre"), None, None)),

        // Security
        pkg("keepassxc", "KeePassXC", "Offline password manager", "Security", "🔑",
            pn(Some("keepassxc"), Some("keepassxc"), Some("keepassxc"), None, None, None, Some("org.keepassxc.KeePassXC"))),
        pkg("veracrypt", "VeraCrypt", "Disk encryption", "Security", "🔐",
            pn(Some("veracrypt"), None, None, None, None, None, Some("com.veracrypt.VeraCrypt"))),
        pkg("clamav", "ClamAV", "Open-source antivirus", "Security", "🛡️",
            pn(Some("clamav"), Some("clamav"), Some("clamav"), Some("clamav"), Some("clamav"), Some("clamav"), None)),

        // File Sharing
        pkg("syncthing", "Syncthing", "Decentralized file synchronization", "File Sharing", "🔄",
            pn(Some("syncthing"), Some("syncthing"), Some("syncthing"), Some("syncthing"), Some("syncthing"), Some("syncthing"), None)),
        pkg("qbittorrent", "qBittorrent", "BitTorrent client", "File Sharing", "📥",
            pn(Some("qbittorrent"), Some("qbittorrent"), Some("qbittorrent"), Some("qbittorrent"), Some("qbittorrent"), None, None)),
        pkg("filezilla", "FileZilla", "FTP/SFTP client", "File Sharing", "📂",
            pn(Some("filezilla"), Some("filezilla"), Some("filezilla"), Some("filezilla"), Some("filezilla"), None, None)),

        // System
        pkg("gparted", "GParted", "Partition editor", "System", "💽",
            pn(Some("gparted"), Some("gparted"), Some("gparted"), Some("gparted"), Some("gparted"), None, None)),
        pkg("timeshift", "Timeshift", "System restore tool", "System", "⏪",
            pn(Some("timeshift"), Some("timeshift"), None, None, None, None, None)),
        pkg("htop", "htop", "Interactive process viewer", "System", "📊",
            pn(Some("htop"), Some("htop"), Some("htop"), Some("htop"), Some("htop"), Some("htop"), None)),
        pkg("btop", "btop", "Resource monitor", "System", "📈",
            pn(Some("btop"), Some("btop"), Some("btop"), None, Some("btop"), None, None)),
        pkg("flatpak", "Flatpak", "Universal app packaging", "System", "📦",
            pn(Some("flatpak"), Some("flatpak"), Some("flatpak"), Some("flatpak"), Some("flatpak"), None, None)),

        // Terminal
        pkg("alacritty", "Alacritty", "GPU-accelerated terminal", "Terminal", "🖥️",
            pn(Some("alacritty"), Some("alacritty"), Some("alacritty"), None, Some("alacritty"), None, None)),
        pkg("kitty", "Kitty", "Fast GPU-based terminal", "Terminal", "🐱",
            pn(Some("kitty"), Some("kitty"), Some("kitty"), None, Some("kitty"), None, None)),
        pkg("fish", "Fish", "Friendly interactive shell", "Terminal", "🐟",
            pn(Some("fish"), Some("fish"), Some("fish"), Some("fish"), Some("fish"), Some("fish"), None)),
        pkg("zsh", "Zsh", "Z shell with plugins", "Terminal", "💲",
            pn(Some("zsh"), Some("zsh"), Some("zsh"), Some("zsh"), Some("zsh"), Some("zsh"), None)),
        pkg("tmux", "tmux", "Terminal multiplexer", "Terminal", "🪟",
            pn(Some("tmux"), Some("tmux"), Some("tmux"), Some("tmux"), Some("tmux"), Some("tmux"), None)),

        // CLI Tools
        pkg("fastfetch", "fastfetch", "System info tool", "CLI Tools", "ℹ️",
            pn(Some("fastfetch"), Some("fastfetch"), Some("fastfetch"), None, Some("fastfetch"), None, None)),
        pkg("fzf", "fzf", "Fuzzy finder", "CLI Tools", "🔎",
            pn(Some("fzf"), Some("fzf"), Some("fzf"), None, Some("fzf"), Some("fzf"), None)),
        pkg("ripgrep", "ripgrep", "Fast recursive grep", "CLI Tools", "🔍",
            pn(Some("ripgrep"), Some("ripgrep"), Some("ripgrep"), None, Some("ripgrep"), Some("ripgrep"), None)),
        pkg("bat", "bat", "cat with syntax highlighting", "CLI Tools", "🦇",
            pn(Some("bat"), Some("bat"), Some("bat"), None, Some("bat"), Some("bat"), None)),
        pkg("eza", "eza", "Modern replacement for ls", "CLI Tools", "📁",
            pn(Some("eza"), None, Some("eza"), None, Some("eza"), None, None)),
    ]
}

fn pkg(id: &str, name: &str, description: &str, category: &str, icon: &str, packages: PackageNames) -> PackageDef {
    PackageDef {
        id: id.to_string(),
        name: name.to_string(),
        description: description.to_string(),
        category: category.to_string(),
        icon: icon.to_string(),
        packages,
    }
}

fn pn(
    pacman: Option<&str>,
    apt: Option<&str>,
    dnf: Option<&str>,
    zypper: Option<&str>,
    xbps: Option<&str>,
    apk: Option<&str>,
    flatpak: Option<&str>,
) -> PackageNames {
    PackageNames {
        pacman: pacman.map(|s| s.to_string()),
        apt: apt.map(|s| s.to_string()),
        dnf: dnf.map(|s| s.to_string()),
        zypper: zypper.map(|s| s.to_string()),
        xbps: xbps.map(|s| s.to_string()),
        apk: apk.map(|s| s.to_string()),
        flatpak: flatpak.map(|s| s.to_string()),
    }
}

pub fn check_installed(package_manager: &str, catalog: &[PackageDef]) -> Vec<PackageStatus> {
    catalog
        .iter()
        .map(|pkg| {
            let installed = is_installed(package_manager, pkg);
            PackageStatus {
                id: pkg.id.clone(),
                installed,
            }
        })
        .collect()
}

fn is_installed(pm: &str, pkg: &PackageDef) -> bool {
    // Check native package first
    let native_name = match pm {
        "pacman" => pkg.packages.pacman.as_deref(),
        "apt" => pkg.packages.apt.as_deref(),
        "dnf" => pkg.packages.dnf.as_deref(),
        "zypper" => pkg.packages.zypper.as_deref(),
        "xbps" => pkg.packages.xbps.as_deref(),
        "apk" => pkg.packages.apk.as_deref(),
        _ => None,
    };

    if let Some(name) = native_name {
        let result = match pm {
            "pacman" => Command::new("pacman").args(["-Qi", name]).output(),
            "apt" => Command::new("dpkg").args(["-s", name]).output(),
            "dnf" => Command::new("rpm").args(["-q", name]).output(),
            "zypper" => Command::new("rpm").args(["-q", name]).output(),
            "xbps" => Command::new("xbps-query").args([name]).output(),
            "apk" => Command::new("apk").args(["info", "-e", name]).output(),
            _ => return false,
        };
        if let Ok(out) = result {
            if out.status.success() {
                return true;
            }
        }
    }

    // Check flatpak
    if let Some(flatpak_id) = pkg.packages.flatpak.as_deref() {
        if validate_flatpak_id(flatpak_id) {
            if let Ok(out) = Command::new("flatpak").args(["info", flatpak_id]).output() {
                if out.status.success() {
                    return true;
                }
            }
        }
    }

    false
}

fn get_native_name<'a>(pm: &str, pkg: &'a PackageDef) -> Option<&'a str> {
    match pm {
        "pacman" => pkg.packages.pacman.as_deref(),
        "apt" => pkg.packages.apt.as_deref(),
        "dnf" => pkg.packages.dnf.as_deref(),
        "zypper" => pkg.packages.zypper.as_deref(),
        "xbps" => pkg.packages.xbps.as_deref(),
        "apk" => pkg.packages.apk.as_deref(),
        _ => None,
    }
}

/// Strict allow-list for native package names.
/// Accepts: ascii alphanumerics, `-`, `_`, `.`, `+` (for `g++`, `c++` etc.).
/// Rejects: empty, leading `-` (would be parsed as a CLI flag), and oversize input.
fn validate_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 128 || name.starts_with('-') {
        return false;
    }
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '+')
}

/// Strict allow-list for Flatpak application IDs (reverse-DNS form,
/// e.g. `org.mozilla.firefox`). Disallows `+`, must contain at least one `.`,
/// and forbids leading `-` / `.`.
fn validate_flatpak_id(id: &str) -> bool {
    if id.is_empty() || id.len() > 255 {
        return false;
    }
    if id.starts_with('-') || id.starts_with('.') || id.ends_with('.') {
        return false;
    }
    if !id.contains('.') {
        return false;
    }
    id.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

fn run_command_streaming<F: Fn(&str)>(
    emit_line: &F,
    program: &str,
    args: &[String],
) -> Result<(bool, String), String> {
    let mut child = Command::new(program)
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

fn get_install_args(pm: &str) -> Vec<String> {
    match pm {
        "pacman" => vec!["pacman", "-S", "--noconfirm"],
        "apt" => vec!["apt-get", "install", "-y"],
        "dnf" => vec!["dnf", "install", "-y"],
        "zypper" => vec!["zypper", "install", "-y"],
        "xbps" => vec!["xbps-install", "-y"],
        "apk" => vec!["apk", "add"],
        _ => vec![],
    }
    .into_iter()
    .map(String::from)
    .collect()
}

fn get_remove_args(pm: &str) -> Vec<String> {
    match pm {
        "pacman" => vec!["pacman", "-Rs", "--noconfirm"],
        "apt" => vec!["apt-get", "remove", "-y"],
        "dnf" => vec!["dnf", "remove", "-y"],
        "zypper" => vec!["zypper", "remove", "-y"],
        "xbps" => vec!["xbps-remove", "-y"],
        "apk" => vec!["apk", "del"],
        _ => vec![],
    }
    .into_iter()
    .map(String::from)
    .collect()
}

pub fn install_packages_batch<F: Fn(&str)>(
    emit_line: &F,
    package_manager: &str,
    packages: &[PackageDef],
) -> Vec<InstallResult> {
    let mut results = Vec::new();
    let mut native: Vec<(&PackageDef, String)> = Vec::new();
    let mut flatpak_only: Vec<&PackageDef> = Vec::new();

    for pkg in packages {
        if let Some(name) = get_native_name(package_manager, pkg) {
            if validate_name(name) {
                native.push((pkg, name.to_string()));
            } else {
                results.push(InstallResult {
                    package_id: pkg.id.clone(),
                    success: false,
                    output: "Invalid package name".into(),
                    method: "none".into(),
                });
            }
        } else if let Some(fid) = pkg.packages.flatpak.as_deref() {
            if validate_flatpak_id(fid) {
                flatpak_only.push(pkg);
            } else {
                results.push(InstallResult {
                    package_id: pkg.id.clone(),
                    success: false,
                    output: "Invalid flatpak ID".into(),
                    method: "none".into(),
                });
            }
        } else {
            results.push(InstallResult {
                package_id: pkg.id.clone(),
                success: false,
                output: format!("No package available via {} or flatpak", package_manager),
                method: "none".into(),
            });
        }
    }

    if !native.is_empty() {
        let base_args = get_install_args(package_manager);
        if !base_args.is_empty() {
            let mut cmd_args = base_args;
            for (_, name) in &native {
                cmd_args.push(name.clone());
            }
            emit_line(&format!("$ pkexec {}", cmd_args.join(" ")));
            match run_command_streaming(emit_line, "pkexec", &cmd_args) {
                Ok((_, combined)) => {
                    for (pkg, name) in &native {
                        let now_installed = is_installed(package_manager, pkg);
                        results.push(InstallResult {
                            package_id: pkg.id.clone(),
                            success: now_installed,
                            output: if now_installed {
                                format!("{} installed successfully", name)
                            } else {
                                combined.clone()
                            },
                            method: package_manager.into(),
                        });
                    }
                }
                Err(e) => {
                    for (pkg, _) in &native {
                        results.push(InstallResult {
                            package_id: pkg.id.clone(),
                            success: false,
                            output: e.clone(),
                            method: package_manager.into(),
                        });
                    }
                }
            }
        }
    }

    for pkg in flatpak_only {
        let fid = pkg.packages.flatpak.as_deref().unwrap();
        emit_line(&format!("$ flatpak install -y flathub {}", fid));
        match run_command_streaming(
            emit_line,
            "flatpak",
            &[
                "install".into(),
                "-y".into(),
                "flathub".into(),
                fid.into(),
            ],
        ) {
            Ok((success, output)) => results.push(InstallResult {
                package_id: pkg.id.clone(),
                success,
                output,
                method: "flatpak".into(),
            }),
            Err(e) => results.push(InstallResult {
                package_id: pkg.id.clone(),
                success: false,
                output: e,
                method: "flatpak".into(),
            }),
        }
    }

    results
}

pub fn remove_packages_batch<F: Fn(&str)>(
    emit_line: &F,
    package_manager: &str,
    packages: &[PackageDef],
) -> Vec<InstallResult> {
    let mut results = Vec::new();
    let mut native: Vec<(&PackageDef, String)> = Vec::new();
    let mut flatpak_only: Vec<&PackageDef> = Vec::new();

    for pkg in packages {
        if let Some(name) = get_native_name(package_manager, pkg) {
            if validate_name(name) {
                let check = match package_manager {
                    "pacman" => Command::new("pacman").args(["-Qi", name]).output(),
                    "apt" => Command::new("dpkg").args(["-s", name]).output(),
                    "dnf" | "zypper" => Command::new("rpm").args(["-q", name]).output(),
                    "xbps" => Command::new("xbps-query").args([name]).output(),
                    "apk" => Command::new("apk").args(["info", "-e", name]).output(),
                    _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "unsupported")),
                };
                if check.map(|o| o.status.success()).unwrap_or(false) {
                    native.push((pkg, name.to_string()));
                    continue;
                }
            }
        }
        if let Some(fid) = pkg.packages.flatpak.as_deref() {
            if validate_flatpak_id(fid) {
                if let Ok(out) = Command::new("flatpak").args(["info", fid]).output() {
                    if out.status.success() {
                        flatpak_only.push(pkg);
                        continue;
                    }
                }
            }
        }
        results.push(InstallResult {
            package_id: pkg.id.clone(),
            success: false,
            output: "Package not installed".into(),
            method: "none".into(),
        });
    }

    if !native.is_empty() {
        let base_args = get_remove_args(package_manager);
        if !base_args.is_empty() {
            // Try batch removal first
            let mut cmd_args = base_args;
            for (_, name) in &native {
                cmd_args.push(name.clone());
            }
            emit_line(&format!("$ pkexec {}", cmd_args.join(" ")));
            let _ = run_command_streaming(emit_line, "pkexec", &cmd_args);

            // Check which packages were actually removed, retry individually for any that weren't
            let mut retry: Vec<(&PackageDef, String)> = Vec::new();
            for (pkg, name) in &native {
                if !is_installed(package_manager, pkg) {
                    results.push(InstallResult {
                        package_id: pkg.id.clone(),
                        success: true,
                        output: format!("{} removed successfully", name),
                        method: package_manager.into(),
                    });
                } else {
                    retry.push((pkg, name.clone()));
                }
            }

            // Retry failed packages individually
            for (pkg, name) in retry {
                let mut individual_args = get_remove_args(package_manager);
                individual_args.push(name.clone());
                emit_line(&format!("Retrying: $ pkexec {}", individual_args.join(" ")));
                let _ = run_command_streaming(emit_line, "pkexec", &individual_args);
                let still_installed = is_installed(package_manager, pkg);
                results.push(InstallResult {
                    package_id: pkg.id.clone(),
                    success: !still_installed,
                    output: if !still_installed {
                        format!("{} removed successfully", name)
                    } else {
                        format!("Failed to remove {} — it may have dependents", name)
                    },
                    method: package_manager.into(),
                });
            }
        }
    }

    for pkg in flatpak_only {
        let fid = pkg.packages.flatpak.as_deref().unwrap();
        emit_line(&format!("$ flatpak uninstall -y {}", fid));
        match run_command_streaming(
            emit_line,
            "flatpak",
            &["uninstall".into(), "-y".into(), fid.into()],
        ) {
            Ok((success, output)) => results.push(InstallResult {
                package_id: pkg.id.clone(),
                success,
                output,
                method: "flatpak".into(),
            }),
            Err(e) => results.push(InstallResult {
                package_id: pkg.id.clone(),
                success: false,
                output: e,
                method: "flatpak".into(),
            }),
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_name_accepts_normal_packages() {
        for ok in ["firefox", "g++", "lib32-glibc", "python3.11", "a", "x_y-z.0+"] {
            assert!(validate_name(ok), "should accept {ok}");
        }
    }

    #[test]
    fn validate_name_rejects_injection() {
        let big = "a".repeat(200);
        for bad in [
            "", "-rf", "--force", "foo;rm -rf /", "foo bar", "foo$(id)",
            "foo`id`", "foo|cat", "foo&pwd", "foo\nbar", "foo\0bar",
            "foo/bar", "../etc", big.as_str(),
        ] {
            assert!(!validate_name(bad), "should reject {:?}", bad);
        }
    }

    #[test]
    fn validate_flatpak_id_accepts_real_ids() {
        for ok in ["org.mozilla.firefox", "com.spotify.Client", "io.github.user.app"] {
            assert!(validate_flatpak_id(ok), "should accept {ok}");
        }
    }

    #[test]
    fn validate_flatpak_id_rejects_bad_input() {
        let big = "a.".repeat(200);
        for bad in [
            "", "firefox", "-org.x.y", ".org.x", "org.x.",
            "org.x;rm", "org.x y", "org.x$y", "org.x+y", "org/x",
            "org.x\nfoo", "org.x\0", big.as_str(),
        ] {
            assert!(!validate_flatpak_id(bad), "should reject {:?}", bad);
        }
    }
}
