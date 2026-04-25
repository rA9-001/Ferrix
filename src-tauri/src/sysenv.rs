//! Spawning *system* binaries from inside a portable bundle (AppImage, Flatpak,
//! Snap, …) is risky: the bundle's runtime injects environment variables that
//! point at libraries / GIO modules / GSettings schemas / GTK plugins shipped
//! inside the bundle. When the spawned tool is a system binary (`pacman`,
//! `apt`, `flatpak`, `systemctl`, `ufw`, …) those variables make it load
//! mismatched shared objects and either crash silently or behave incorrectly.
//!
//! AppImage's AppRun helpfully saves the originals as `APPIMAGE_ORIGINAL_<VAR>`
//! before overwriting them. This module restores those (or unsets the
//! contaminated value entirely) before spawning the child process.
//!
//! Always use [`system_command`] instead of `Command::new` when invoking a
//! tool that lives on the host system rather than inside our own bundle.

use std::process::Command;

/// Environment variables that bundles commonly override and that confuse
/// system tools when inherited.
const CONTAMINATED_VARS: &[&str] = &[
    // Dynamic linker
    "LD_LIBRARY_PATH",
    "LD_PRELOAD",
    "LD_AUDIT",
    // Language runtimes
    "PYTHONPATH",
    "PYTHONHOME",
    "PERLLIB",
    "PERL5LIB",
    "RUBYLIB",
    "NODE_PATH",
    // XDG / GLib / GIO
    "XDG_DATA_DIRS",
    "XDG_CONFIG_DIRS",
    "GIO_MODULE_DIR",
    "GIO_EXTRA_MODULES",
    "GSETTINGS_SCHEMA_DIR",
    // GTK
    "GTK_PATH",
    "GTK_DATA_PREFIX",
    "GTK_EXE_PREFIX",
    "GTK_IM_MODULE_FILE",
    "GDK_PIXBUF_MODULE_FILE",
    "GDK_PIXBUF_MODULEDIR",
    // Qt
    "QT_PLUGIN_PATH",
    "QT_QPA_PLATFORM_PLUGIN_PATH",
    // Fontconfig
    "FONTCONFIG_PATH",
    "FONTCONFIG_FILE",
    // Misc
    "LIBVA_DRIVERS_PATH",
    "LIBGL_DRIVERS_PATH",
    "ALSA_CONFIG_PATH",
];

/// Build a [`Command`] with an environment safe for invoking host system
/// binaries from within a portable bundle.
///
/// For each variable in [`CONTAMINATED_VARS`]:
/// - If `APPIMAGE_ORIGINAL_<VAR>` exists and is non-empty, restore it.
/// - Otherwise, remove the variable entirely so the child sees the system
///   defaults.
pub fn system_command<S: AsRef<std::ffi::OsStr>>(program: S) -> Command {
    let mut cmd = Command::new(program);
    for var in CONTAMINATED_VARS {
        let original_key = format!("APPIMAGE_ORIGINAL_{var}");
        match std::env::var_os(&original_key) {
            Some(val) if !val.is_empty() => {
                cmd.env(var, val);
            }
            _ => {
                cmd.env_remove(var);
            }
        }
    }
    cmd
}
