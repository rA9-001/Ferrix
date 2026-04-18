mod audit_log;
mod cleaner;
mod disk;
mod distro;
mod firewall;
mod hardening;
mod hardware;
mod logs;
mod network;
mod optimizer;
mod packages;
mod permissions;
mod services;
mod startup;
mod updates;

use cleaner::{CleanupCategory, CleanupResult};
use disk::{DiskOverview, SpaceAnalysis};
use distro::DistroInfo;
use firewall::{FirewallActionResponse, FirewallStatus};
use hardening::{HardeningResult, HardeningStatus};
use hardware::{HardwareInfo, SystemStats};
use optimizer::{TweakResult, TweakStatus};
use network::{InterfaceTraffic, NetworkSnapshot};
use permissions::AuditReport;
use packages::{InstallResult, PackageDef, PackageStatus};
use services::{ServiceInfo, ServiceLogs, ServiceResult};
use startup::StartupEntry;
use updates::UpdateCheckResult;
use logs::{LogQueryResult, JournalUnit};
use tauri::Emitter;

/// Default minimum interval between two invocations of the same privileged command.
/// Prevents accidental double-clicks and trivial DoS via UI scripting.
const RATE_LIMIT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(750);

/// Convenience wrapper: rate-limit + audit-log a privileged command.
/// Returns `Err` if the rate-limit is hit; the audit record is only written on success.
fn guard(command: &'static str, detail: &str) -> Result<(), String> {
    audit_log::check_rate_limit(command, RATE_LIMIT_INTERVAL)?;
    audit_log::record(command, detail);
    Ok(())
}

#[tauri::command]
async fn get_distro_info() -> DistroInfo {
    tokio::task::spawn_blocking(DistroInfo::detect)
        .await
        .unwrap()
}

#[tauri::command]
async fn scan_cleanup() -> Vec<CleanupCategory> {
    tokio::task::spawn_blocking(|| {
        let distro = DistroInfo::detect();
        cleaner::scan_cleanup_targets(&distro)
    })
    .await
    .unwrap()
}

#[tauri::command]
async fn run_cleanup(categories: Vec<String>) -> Vec<CleanupResult> {
    audit_log::record("run_cleanup", &categories.join(","));
    tokio::task::spawn_blocking(move || {
        let distro = DistroInfo::detect();
        cleaner::execute_cleanup(&categories, &distro)
    })
    .await
    .unwrap()
}

#[tauri::command]
async fn get_disk_overview() -> DiskOverview {
    tokio::task::spawn_blocking(disk::get_disk_overview)
        .await
        .unwrap()
}

#[tauri::command]
async fn analyze_space(path: String) -> SpaceAnalysis {
    tokio::task::spawn_blocking(move || disk::analyze_space(&path))
        .await
        .unwrap()
}

#[tauri::command]
async fn get_startup_entries() -> Vec<StartupEntry> {
    tokio::task::spawn_blocking(startup::get_startup_entries)
        .await
        .unwrap()
}

#[tauri::command]
async fn toggle_startup_entry(id: String, enabled: bool) -> Result<String, String> {
    guard("toggle_startup_entry", &format!("{}={}", id, enabled))?;
    tokio::task::spawn_blocking(move || startup::toggle_startup_entry(&id, enabled))
        .await
        .unwrap()
}

#[tauri::command]
async fn remove_startup_entry(id: String) -> Result<String, String> {
    guard("remove_startup_entry", &id)?;
    tokio::task::spawn_blocking(move || startup::remove_startup_entry(&id))
        .await
        .unwrap()
}

#[tauri::command]
async fn add_startup_entry(name: String, command: String, comment: String) -> Result<String, String> {
    guard("add_startup_entry", &name)?;
    tokio::task::spawn_blocking(move || startup::add_startup_entry(&name, &command, &comment))
        .await
        .unwrap()
}

#[tauri::command]
async fn get_hardware_info() -> HardwareInfo {
    tokio::task::spawn_blocking(hardware::get_hardware_info)
        .await
        .unwrap()
}

#[tauri::command]
async fn get_system_stats() -> SystemStats {
    tokio::task::spawn_blocking(hardware::get_system_stats)
        .await
        .unwrap()
}

#[tauri::command]
async fn get_tweaks() -> Vec<TweakStatus> {
    tokio::task::spawn_blocking(optimizer::get_tweaks)
        .await
        .unwrap()
}

#[tauri::command]
async fn apply_tweak(id: String) -> TweakResult {
    audit_log::record("apply_tweak", &id);
    tokio::task::spawn_blocking(move || optimizer::apply_tweak(&id))
        .await
        .unwrap()
}

#[tauri::command]
async fn restore_tweak(id: String) -> TweakResult {
    audit_log::record("restore_tweak", &id);
    tokio::task::spawn_blocking(move || optimizer::restore_tweak(&id))
        .await
        .unwrap()
}

#[tauri::command]
async fn get_hardening_status() -> Vec<HardeningStatus> {
    tokio::task::spawn_blocking(hardening::get_hardening_status)
        .await
        .unwrap()
}

#[tauri::command]
async fn apply_hardening(id: String) -> HardeningResult {
    audit_log::record("apply_hardening", &id);
    tokio::task::spawn_blocking(move || hardening::apply_hardening(&id))
        .await
        .unwrap()
}

#[tauri::command]
async fn restore_hardening(id: String) -> HardeningResult {
    audit_log::record("restore_hardening", &id);
    tokio::task::spawn_blocking(move || hardening::restore_hardening(&id))
        .await
        .unwrap()
}

#[tauri::command]
async fn get_firewall_status() -> FirewallStatus {
    tokio::task::spawn_blocking(firewall::get_firewall_status)
        .await
        .unwrap()
}

#[tauri::command]
async fn toggle_firewall(enable: bool) -> FirewallActionResponse {
    audit_log::record("toggle_firewall", &enable.to_string());
    tokio::task::spawn_blocking(move || firewall::toggle_firewall(enable))
        .await
        .unwrap()
}

#[tauri::command]
async fn set_default_policy(direction: String, policy: String) -> FirewallActionResponse {
    audit_log::record("set_default_policy", &format!("{} {}", direction, policy));
    tokio::task::spawn_blocking(move || firewall::set_default_policy(&direction, &policy))
        .await
        .unwrap()
}

#[tauri::command]
async fn add_firewall_rule(
    action: String,
    direction: String,
    port: String,
    protocol: String,
    from_ip: String,
    comment: String,
) -> FirewallActionResponse {
    audit_log::record(
        "add_firewall_rule",
        &format!("{} {} {}/{} from {}", action, direction, port, protocol, from_ip),
    );
    tokio::task::spawn_blocking(move || {
        firewall::add_rule(&action, &direction, &port, &protocol, &from_ip, &comment)
    })
    .await
    .unwrap()
}

#[tauri::command]
async fn delete_firewall_rule(number: u32) -> FirewallActionResponse {
    audit_log::record("delete_firewall_rule", &number.to_string());
    tokio::task::spawn_blocking(move || firewall::delete_rule(number))
        .await
        .unwrap()
}

#[tauri::command]
async fn set_firewall_logging(level: String) -> FirewallActionResponse {
    audit_log::record("set_firewall_logging", &level);
    tokio::task::spawn_blocking(move || firewall::set_logging(&level))
        .await
        .unwrap()
}

#[tauri::command]
async fn list_services() -> Vec<ServiceInfo> {
    tokio::task::spawn_blocking(services::list_services)
        .await
        .unwrap()
}

#[tauri::command]
async fn start_service(name: String) -> ServiceResult {
    audit_log::record("start_service", &name);
    tokio::task::spawn_blocking(move || services::start_service(&name))
        .await
        .unwrap()
}

#[tauri::command]
async fn stop_service(name: String) -> ServiceResult {
    audit_log::record("stop_service", &name);
    tokio::task::spawn_blocking(move || services::stop_service(&name))
        .await
        .unwrap()
}

#[tauri::command]
async fn restart_service(name: String) -> ServiceResult {
    audit_log::record("restart_service", &name);
    tokio::task::spawn_blocking(move || services::restart_service(&name))
        .await
        .unwrap()
}

#[tauri::command]
async fn enable_service(name: String) -> ServiceResult {
    audit_log::record("enable_service", &name);
    tokio::task::spawn_blocking(move || services::enable_service(&name))
        .await
        .unwrap()
}

#[tauri::command]
async fn disable_service(name: String) -> ServiceResult {
    audit_log::record("disable_service", &name);
    tokio::task::spawn_blocking(move || services::disable_service(&name))
        .await
        .unwrap()
}

#[tauri::command]
async fn get_service_logs(name: String, lines: u32) -> ServiceLogs {
    tokio::task::spawn_blocking(move || services::get_service_logs(&name, lines))
        .await
        .unwrap()
}

#[tauri::command]
async fn get_package_catalog() -> Vec<PackageDef> {
    packages::get_package_catalog()
}

#[tauri::command]
async fn check_packages_installed(package_manager: String) -> Vec<PackageStatus> {
    tokio::task::spawn_blocking(move || {
        let catalog = packages::get_package_catalog();
        packages::check_installed(&package_manager, &catalog)
    })
    .await
    .unwrap()
}

#[tauri::command]
async fn install_packages(app: tauri::AppHandle, package_ids: Vec<String>) -> Vec<InstallResult> {
    audit_log::record("install_packages", &package_ids.join(","));
    tokio::task::spawn_blocking(move || {
        let distro = DistroInfo::detect();
        let catalog = packages::get_package_catalog();
        let pkgs: Vec<PackageDef> = package_ids
            .iter()
            .filter_map(|id| catalog.iter().find(|p| p.id == *id).cloned())
            .collect();
        let emit = |line: &str| {
            let _ = app.emit("install-output", line.to_string());
        };
        packages::install_packages_batch(&emit, &distro.package_manager, &pkgs)
    })
    .await
    .unwrap()
}

#[tauri::command]
async fn remove_packages(app: tauri::AppHandle, package_ids: Vec<String>) -> Vec<InstallResult> {
    audit_log::record("remove_packages", &package_ids.join(","));
    tokio::task::spawn_blocking(move || {
        let distro = DistroInfo::detect();
        let catalog = packages::get_package_catalog();
        let pkgs: Vec<PackageDef> = package_ids
            .iter()
            .filter_map(|id| catalog.iter().find(|p| p.id == *id).cloned())
            .collect();
        let emit = |line: &str| {
            let _ = app.emit("remove-output", line.to_string());
        };
        packages::remove_packages_batch(&emit, &distro.package_manager, &pkgs)
    })
    .await
    .unwrap()
}

#[tauri::command]
async fn run_permission_audit(scan_paths: Vec<String>) -> AuditReport {
    tokio::task::spawn_blocking(move || permissions::run_audit(scan_paths))
        .await
        .unwrap()
}

#[tauri::command]
async fn get_network_snapshot() -> NetworkSnapshot {
    tokio::task::spawn_blocking(network::get_network_snapshot)
        .await
        .unwrap()
}

#[tauri::command]
async fn get_traffic_snapshot() -> Vec<InterfaceTraffic> {
    tokio::task::spawn_blocking(network::get_traffic_snapshot)
        .await
        .unwrap()
}

#[tauri::command]
async fn check_updates() -> UpdateCheckResult {
    tokio::task::spawn_blocking(|| {
        let distro = DistroInfo::detect();
        updates::check_updates(&distro.package_manager)
    })
    .await
    .unwrap()
}

#[tauri::command]
async fn apply_updates(app: tauri::AppHandle, update_flatpak: bool) {
    audit_log::record("apply_updates", &format!("flatpak={}", update_flatpak));
    tokio::task::spawn_blocking(move || {
        let distro = DistroInfo::detect();
        let emit = move |line: &str| {
            let _ = app.emit("update-output", line.to_string());
        };
        updates::apply_updates(&emit, &distro.package_manager, update_flatpak);
    })
    .await
    .unwrap()
}

#[tauri::command]
async fn query_logs(
    priority: String,
    unit: String,
    boot: String,
    lines: u32,
    grep: String,
    since: String,
) -> LogQueryResult {
    tokio::task::spawn_blocking(move || logs::query_logs(&priority, &unit, &boot, lines, &grep, &since))
        .await
        .unwrap()
}

#[tauri::command]
async fn get_journal_units() -> Vec<JournalUnit> {
    tokio::task::spawn_blocking(logs::get_journal_units)
        .await
        .unwrap()
}

#[tauri::command]
async fn get_boot_list() -> Vec<String> {
    tokio::task::spawn_blocking(logs::get_boot_list)
        .await
        .unwrap()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Force X11 backend (XWayland) — WebKitGTK crashes with NVIDIA on native Wayland
    std::env::set_var("GDK_BACKEND", "x11");
    // Suppress GBM buffer errors when GPU compositing isn't available via XWayland
    std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_distro_info,
            scan_cleanup,
            run_cleanup,
            get_disk_overview,
            analyze_space,
            get_startup_entries,
            toggle_startup_entry,
            remove_startup_entry,
            add_startup_entry,
            get_hardware_info,
            get_system_stats,
            get_tweaks,
            apply_tweak,
            restore_tweak,
            get_hardening_status,
            apply_hardening,
            restore_hardening,
            get_firewall_status,
            toggle_firewall,
            set_default_policy,
            add_firewall_rule,
            delete_firewall_rule,
            set_firewall_logging,
            get_network_snapshot,
            get_traffic_snapshot,
            list_services,
            start_service,
            stop_service,
            restart_service,
            enable_service,
            disable_service,
            get_service_logs,
            run_permission_audit,
            get_package_catalog,
            check_packages_installed,
            install_packages,
            remove_packages,
            check_updates,
            apply_updates,
            query_logs,
            get_journal_units,
            get_boot_list,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
