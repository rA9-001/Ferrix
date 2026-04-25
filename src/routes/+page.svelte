<script>
  import { invoke } from "@tauri-apps/api/core";
  import DiskUsage from "$lib/DiskUsage.svelte";
  import StartupManager from "$lib/StartupManager.svelte";
  import HardwareInfo from "$lib/HardwareInfo.svelte";
  import PerformanceOptimizer from "$lib/PerformanceOptimizer.svelte";
  import SecurityHardening from "$lib/SecurityHardening.svelte";
  import FirewallManager from "$lib/FirewallManager.svelte";
  import NetworkMonitor from "$lib/NetworkMonitor.svelte";
  import ServiceManager from "$lib/ServiceManager.svelte";
  import PermissionsAuditor from "$lib/PermissionsAuditor.svelte";
  import PackageInstaller from "$lib/PackageInstaller.svelte";
  import UpdateManager from "$lib/UpdateManager.svelte";
  import LogViewer from "$lib/LogViewer.svelte";
  import Dashboard from "$lib/Dashboard.svelte";
  import ConfirmDialog from "$lib/ConfirmDialog.svelte";

  let confirmCleanupOpen = $state(false);

  let activeView = $state("dashboard");
  // Tabs are mounted on first visit and kept alive afterwards so switching
  // back is instant and scroll positions are preserved. Heavy pollers
  // (Dashboard, HardwareInfo, NetworkMonitor) receive `active` and pause
  // their intervals while hidden.
  let activated = $state(new Set(["dashboard"]));
  function switchTo(view) {
    if (!activated.has(view)) {
      activated = new Set([...activated, view]);
    }
    activeView = view;
  }
  let distroInfo = $state(null);
  let categories = $state([]);
  let scanning = $state(false);
  let cleaning = $state(false);
  let cleanupResults = $state(null);
  let selectedCategories = $state(new Set());
  let consoleOpen = $state(false);
  let consoleFilter = $state("all"); // "all" | "removed" | "error"
  let expandedCategories = $state(new Set());

  function toggleExpand(name) {
    const next = new Set(expandedCategories);
    if (next.has(name)) {
      next.delete(name);
    } else {
      next.add(name);
    }
    expandedCategories = next;
  }

  async function loadDistroInfo() {
    distroInfo = await invoke("get_distro_info");
  }

  async function scanSystem() {
    scanning = true;
    cleanupResults = null;
    try {
      categories = await invoke("scan_cleanup");
      selectedCategories = new Set(
        categories.filter((c) => c.items.length > 0 && c.total_size > 0).map((c) => c.name)
      );
    } finally {
      scanning = false;
    }
  }

  async function runCleanup() {
    if (selectedCategories.size === 0) return;
    confirmCleanupOpen = true;
  }

  async function performCleanup() {
    cleaning = true;
    try {
      cleanupResults = await invoke("run_cleanup", {
        categories: Array.from(selectedCategories),
      });
      consoleOpen = true;
      consoleFilter = "all";
      categories = await invoke("scan_cleanup");
    } catch (e) {
      cleanupResults = [{
        category: "Error",
        freed_bytes: 0,
        removed_count: 0,
        errors: [String(e)],
        log: [{ path: "", size: 0, status: "error", message: String(e), category: "Error" }],
      }];
      consoleOpen = true;
      consoleFilter = "error";
    } finally {
      cleaning = false;
    }
  }

  function toggleCategory(name) {
    const next = new Set(selectedCategories);
    if (next.has(name)) {
      next.delete(name);
    } else {
      next.add(name);
    }
    selectedCategories = next;
  }

  function toggleAll() {
    if (selectedCategories.size === nonEmptyCategories.length) {
      selectedCategories = new Set();
    } else {
      selectedCategories = new Set(nonEmptyCategories.map((c) => c.name));
    }
  }

  function formatBytes(bytes) {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
  }

  $effect(() => {
    loadDistroInfo();
  });

  let nonEmptyCategories = $derived(
    categories.filter((c) => c.items.length > 0 && c.total_size > 0)
  );

  let totalSize = $derived(
    nonEmptyCategories.reduce((sum, cat) => sum + cat.total_size, 0)
  );

  let selectedSize = $derived(
    nonEmptyCategories
      .filter((c) => selectedCategories.has(c.name))
      .reduce((sum, cat) => sum + cat.total_size, 0)
  );

  let allLogEntries = $derived(
    cleanupResults
      ? cleanupResults.flatMap((r) =>
          r.log.map((entry) => ({ ...entry, category: r.category }))
        )
      : []
  );

  let filteredLogEntries = $derived(
    consoleFilter === "all"
      ? allLogEntries
      : allLogEntries.filter((e) => e.status === consoleFilter)
  );

  let removedCount = $derived(allLogEntries.filter((e) => e.status === "removed").length);
  let errorCount = $derived(allLogEntries.filter((e) => e.status === "error").length);
  let totalFreed = $derived(
    cleanupResults ? cleanupResults.reduce((s, r) => s + r.freed_bytes, 0) : 0
  );
</script>

<main>
  <div class="app-layout">
    <aside class="sidebar">
      <div class="sidebar-header">
        <div class="brand">
          <img class="brand-logo" src="/favicon.png" alt="Ferrix" />
          <h1>Ferrix</h1>
        </div>
      </div>
      <nav>
        <button class="nav-item" class:active={activeView === "dashboard"} onclick={() => switchTo("dashboard")}>
          <span class="nav-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/></svg></span>
          <span class="nav-label">Dashboard</span>
        </button>
        <div class="nav-section">
          <div class="nav-section-label">Maintenance</div>
          <button class="nav-item" class:active={activeView === "cleanup"} onclick={() => switchTo("cleanup")}>
            <span class="nav-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2"/><path d="M5 6v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V6"/><line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/></svg></span>
            <span class="nav-label">System Cleanup</span>
          </button>
          <button class="nav-item" class:active={activeView === "disk"} onclick={() => switchTo("disk")}>
            <span class="nav-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="3"/><line x1="12" y1="2" x2="12" y2="5"/></svg></span>
            <span class="nav-label">Disk Usage</span>
          </button>
          <button class="nav-item" class:active={activeView === "updates"} onclick={() => switchTo("updates")}>
            <span class="nav-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 19V5"/><polyline points="5 12 12 5 19 12"/><line x1="5" y1="19" x2="19" y2="19"/></svg></span>
            <span class="nav-label">Updates</span>
          </button>
          <button class="nav-item" class:active={activeView === "packages"} onclick={() => switchTo("packages")}>
            <span class="nav-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><polyline points="3.27 6.96 12 12.01 20.73 6.96"/><line x1="12" y1="22.08" x2="12" y2="12"/></svg></span>
            <span class="nav-label">Packages</span>
          </button>
        </div>

        <div class="nav-section">
          <div class="nav-section-label">System</div>
          <button class="nav-item" class:active={activeView === "hardware"} onclick={() => switchTo("hardware")}>
            <span class="nav-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg></span>
            <span class="nav-label">System Monitor</span>
          </button>
          <button class="nav-item" class:active={activeView === "optimizer"} onclick={() => switchTo("optimizer")}>
            <span class="nav-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/></svg></span>
            <span class="nav-label">Performance</span>
          </button>
          <button class="nav-item" class:active={activeView === "services"} onclick={() => switchTo("services")}>
            <span class="nav-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg></span>
            <span class="nav-label">Services</span>
          </button>
          <button class="nav-item" class:active={activeView === "startup"} onclick={() => switchTo("startup")}>
            <span class="nav-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M18.36 6.64a9 9 0 1 1-12.73 0"/><line x1="12" y1="2" x2="12" y2="12"/></svg></span>
            <span class="nav-label">Startup</span>
          </button>
          <button class="nav-item" class:active={activeView === "logs"} onclick={() => switchTo("logs")}>
            <span class="nav-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="8" y1="13" x2="16" y2="13"/><line x1="8" y1="17" x2="16" y2="17"/></svg></span>
            <span class="nav-label">Logs</span>
          </button>
        </div>

        <div class="nav-section">
          <div class="nav-section-label">Network</div>
          <button class="nav-item" class:active={activeView === "network"} onclick={() => switchTo("network")}>
            <span class="nav-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg></span>
            <span class="nav-label">Network</span>
          </button>
          <button class="nav-item" class:active={activeView === "firewall"} onclick={() => switchTo("firewall")}>
            <span class="nav-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg></span>
            <span class="nav-label">Firewall</span>
          </button>
        </div>

        <div class="nav-section">
          <div class="nav-section-label">Security</div>
          <button class="nav-item" class:active={activeView === "hardening"} onclick={() => switchTo("hardening")}>
            <span class="nav-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg></span>
            <span class="nav-label">Hardening</span>
          </button>
          <button class="nav-item" class:active={activeView === "permissions"} onclick={() => switchTo("permissions")}>
            <span class="nav-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg></span>
            <span class="nav-label">Permissions</span>
          </button>
        </div>
      </nav>
      {#if distroInfo}
        <div class="distro-info">
          <div class="distro-status">
            <span class="status-dot"></span>
            <span class="distro-name">{distroInfo.name}</span>
          </div>
          <p class="distro-detail">
            <span class="distro-detail-label">pkg</span>
            <code>{distroInfo.package_manager}</code>
          </p>
        </div>
      {/if}
    </aside>

    <section class="content">
      <div class="view-pane" hidden={activeView !== "dashboard"}>
        {#if activated.has("dashboard")}
          <Dashboard onNavigate={(view) => switchTo(view)} active={activeView === "dashboard"} />
        {/if}
      </div>
      <div class="view-pane" hidden={activeView !== "disk"}>
        {#if activated.has("disk")}<DiskUsage />{/if}
      </div>
      <div class="view-pane" hidden={activeView !== "startup"}>
        {#if activated.has("startup")}<StartupManager />{/if}
      </div>
      <div class="view-pane" hidden={activeView !== "hardware"}>
        {#if activated.has("hardware")}<HardwareInfo active={activeView === "hardware"} />{/if}
      </div>
      <div class="view-pane" hidden={activeView !== "optimizer"}>
        {#if activated.has("optimizer")}<PerformanceOptimizer />{/if}
      </div>
      <div class="view-pane" hidden={activeView !== "hardening"}>
        {#if activated.has("hardening")}<SecurityHardening />{/if}
      </div>
      <div class="view-pane" hidden={activeView !== "firewall"}>
        {#if activated.has("firewall")}<FirewallManager />{/if}
      </div>
      <div class="view-pane" hidden={activeView !== "network"}>
        {#if activated.has("network")}<NetworkMonitor active={activeView === "network"} />{/if}
      </div>
      <div class="view-pane" hidden={activeView !== "services"}>
        {#if activated.has("services")}<ServiceManager />{/if}
      </div>
      <div class="view-pane" hidden={activeView !== "permissions"}>
        {#if activated.has("permissions")}<PermissionsAuditor />{/if}
      </div>
      <div class="view-pane" hidden={activeView !== "packages"}>
        {#if activated.has("packages")}<PackageInstaller />{/if}
      </div>
      <div class="view-pane" hidden={activeView !== "updates"}>
        {#if activated.has("updates")}<UpdateManager />{/if}
      </div>
      <div class="view-pane" hidden={activeView !== "logs"}>
        {#if activated.has("logs")}<LogViewer />{/if}
      </div>
      <div class="view-pane" hidden={activeView !== "cleanup"}>
      <header class="content-header">
        <div>
          <h2>System Cleanup</h2>
          <p class="subtitle">Scan and remove unused files, caches, and logs</p>
        </div>
        <div class="header-actions">
          <button class="btn btn-secondary" onclick={scanSystem} disabled={scanning}>
            {scanning ? "Scanning..." : "Scan System"}
          </button>
          {#if nonEmptyCategories.length > 0}
            <button
              class="btn btn-primary"
              onclick={runCleanup}
              disabled={cleaning || selectedCategories.size === 0}
            >
              {cleaning ? "Cleaning..." : `Clean ${formatBytes(selectedSize)}`}
            </button>
          {/if}
        </div>
      </header>

      {#if cleanupResults}
        <div class="console-wrapper">
        <div class="results-banner" class:console-visible={consoleOpen}>
          <span class="results-icon">{@html '<svg style="width:1.2em;height:1.2em;vertical-align:middle" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>'}</span>
          <div class="results-text">
            <strong>Cleanup complete!</strong>
            <p>
              {removedCount} items removed — {formatBytes(totalFreed)} freed
              {#if errorCount > 0}
                <span class="warning"> · {errorCount} errors</span>
              {/if}
            </p>
          </div>
          <button class="btn btn-console" onclick={() => consoleOpen = !consoleOpen}>
            {consoleOpen ? "Hide" : "Show"} Console
            <span class="console-count">{allLogEntries.length}</span>
          </button>
        </div>

        {#if consoleOpen}
          <div class="console-panel">
            <div class="console-toolbar">
              <div class="console-filters">
                <button
                  class="filter-btn"
                  class:active={consoleFilter === "all"}
                  onclick={() => consoleFilter = "all"}
                >
                  All <span class="filter-count">{allLogEntries.length}</span>
                </button>
                <button
                  class="filter-btn"
                  class:active={consoleFilter === "removed"}
                  onclick={() => consoleFilter = "removed"}
                >
                  Removed <span class="filter-count">{removedCount}</span>
                </button>
                <button
                  class="filter-btn"
                  class:active={consoleFilter === "error"}
                  onclick={() => consoleFilter = "error"}
                >
                  Errors <span class="filter-count">{errorCount}</span>
                </button>
              </div>
              <span class="console-summary">{formatBytes(totalFreed)} freed</span>
            </div>
            <div class="console-log">
              {#each filteredLogEntries as entry}
                <div class="log-entry" class:log-error={entry.status === "error"}>
                  <span class="log-icon">{entry.status === "removed" ? "✓" : "✗"}</span>
                  <span class="log-category">{entry.category}</span>
                  <span class="log-path" title={entry.path}>{entry.path}</span>
                  {#if entry.size > 0}
                    <span class="log-size">{formatBytes(entry.size)}</span>
                  {/if}
                  {#if entry.status === "error"}
                    <span class="log-message">{entry.message}</span>
                  {/if}
                </div>
              {/each}
              {#if filteredLogEntries.length === 0}
                <div class="log-empty">No entries match this filter</div>
              {/if}
            </div>
          </div>
        {/if}
        </div>
      {/if}

      {#if categories.length === 0 && !scanning}
        <div class="empty-state">
          <div class="empty-icon">{@html '<svg style="width:48px;height:48px" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>'}</div>
          <h3>No scan results yet</h3>
          <p>Click "Scan System" to find cleanable files</p>
        </div>
      {:else if nonEmptyCategories.length === 0 && !scanning && categories.length > 0}
        <div class="empty-state">
          <div class="empty-icon">{@html '<svg style="width:48px;height:48px" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2l2.4 7.2L22 12l-7.6 2.8L12 22l-2.4-7.2L2 12l7.6-2.8z"/></svg>'}</div>
          <h3>Your system is clean!</h3>
          <p>No unused files or caches were found</p>
        </div>
      {/if}

      {#if scanning}
        <div class="empty-state">
          <div class="spinner"></div>
          <h3>Scanning your system...</h3>
          <p>This may take a moment</p>
        </div>
      {/if}

      {#if nonEmptyCategories.length > 0 && !scanning}
        <div class="summary-bar">
          <div class="summary-stat">
            <span class="stat-value">{formatBytes(totalSize)}</span>
            <span class="stat-label">Total found</span>
          </div>
          <div class="summary-stat">
            <span class="stat-value">{nonEmptyCategories.length}</span>
            <span class="stat-label">Categories</span>
          </div>
          <div class="summary-stat">
            <span class="stat-value">{formatBytes(selectedSize)}</span>
            <span class="stat-label">Selected</span>
          </div>
          <button class="btn-link" onclick={toggleAll}>
            {selectedCategories.size === nonEmptyCategories.length ? "Deselect All" : "Select All"}
          </button>
        </div>

        <div class="categories">
          {#each nonEmptyCategories as cat}
            <div
              class="category-card"
              class:selected={selectedCategories.has(cat.name)}
              role="checkbox"
              aria-checked={selectedCategories.has(cat.name)}
              tabindex="0"
              onclick={(e) => {
                if (!e.target.closest('.expand-btn')) toggleCategory(cat.name);
              }}
              onkeydown={(e) => {
                if (e.key === ' ' || e.key === 'Enter') { e.preventDefault(); toggleCategory(cat.name); }
              }}
            >
              <div class="category-header">
                <div class="checkbox-label">
                  <input
                    type="checkbox"
                    checked={selectedCategories.has(cat.name)}
                    onclick={(e) => e.stopPropagation()}
                    onchange={() => toggleCategory(cat.name)}
                  />
                  <span class="category-name">{cat.name}</span>
                </div>
                <span class="category-size">{formatBytes(cat.total_size)}</span>
              </div>
              <p class="category-desc">{cat.description}</p>
              <div class="category-footer">
                <span class="category-meta">{cat.items.length} items</span>
                {#if cat.items.length > 0}
                  <button class="expand-btn" onclick={(e) => { e.stopPropagation(); toggleExpand(cat.name); }}>
                    {expandedCategories.has(cat.name) ? "Hide details ▲" : "Show details ▼"}
                  </button>
                {/if}
              </div>
            </div>

            {#if expandedCategories.has(cat.name)}
              <div class="details-panel">
                <div class="details-header">
                  <span class="details-title">{cat.name}</span>
                  <span class="details-count">{cat.items.length} items · {formatBytes(cat.total_size)}</span>
                  <button class="expand-btn" onclick={() => toggleExpand(cat.name)}>Close ✕</button>
                </div>
                <div class="item-list">
                  {#each cat.items.slice(0, 50) as item}
                    <div class="item-row">
                      <span class="item-icon">{@html item.item_type === "directory" ? '<svg style="width:1em;height:1em;vertical-align:middle" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>' : '<svg style="width:1em;height:1em;vertical-align:middle" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>'}</span>
                      <span class="item-path" title={item.path}>{item.path}</span>
                      <span class="item-size">{formatBytes(item.size)}</span>
                    </div>
                  {/each}
                  {#if cat.items.length > 50}
                    <div class="item-more">...and {cat.items.length - 50} more items</div>
                  {/if}
                </div>
              </div>
            {/if}
          {/each}
        </div>
      {/if}
      </div>
    </section>
  </div>
</main>

<ConfirmDialog
  bind:open={confirmCleanupOpen}
  title="Run system cleanup?"
  message={`${selectedCategories.size} categor${selectedCategories.size === 1 ? "y" : "ies"} will be cleaned. Root-owned items (package cache, journal, old logs) will require pkexec authentication. This cannot be undone.`}
  confirmLabel="Clean"
  destructive
  onconfirm={performCleanup}
/>

<style>
  main { height: 100vh; width: 100vw; }
  .app-layout { display: flex; height: 100vh; }

  /* ================ SIDEBAR ================ */
  .sidebar {
    width: 252px;
    background: linear-gradient(180deg, var(--surface-1) 0%, var(--bg-canvas) 100%);
    border-right: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    position: relative;
  }
  .sidebar::after {
    /* subtle ember glow at top */
    content: "";
    position: absolute;
    top: 0; left: 0; right: 0;
    height: 200px;
    background: radial-gradient(ellipse 70% 100% at 30% 0%, var(--accent-soft), transparent 70%);
    pointer-events: none;
  }
  .sidebar > * { position: relative; z-index: 1; }

  .sidebar-header {
    padding: 22px 20px 18px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .brand-logo {
    width: 52px;
    height: 52px;
    border-radius: 6px;
    object-fit: contain;
    flex-shrink: 0;
  }
  .sidebar-header h1 {
    font-family: var(--font-display);
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: -0.01em;
  }

  nav {
    padding: 14px 10px;
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .nav-section { display: flex; flex-direction: column; gap: 1px; }
  .nav-section-label {
    padding: 0 12px 8px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-muted);
  }
  .nav-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 9px 12px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-secondary);
    font-size: var(--text-base);
    font-family: inherit;
    cursor: pointer;
    transition: background var(--duration) var(--ease), color var(--duration) var(--ease);
    text-align: left;
    position: relative;
  }
  .nav-item:hover { background: var(--surface-hover); color: var(--text-primary); }
  .nav-item.active {
    background: var(--accent-soft);
    color: var(--text-primary);
    font-weight: 500;
  }
  .nav-item.active::before {
    content: "";
    position: absolute;
    left: -10px;
    top: 50%;
    transform: translateY(-50%);
    width: 3px;
    height: 18px;
    background: var(--accent);
    border-radius: 0 3px 3px 0;
    box-shadow: 0 0 12px var(--accent-glow);
  }
  .nav-icon {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .nav-icon :global(svg) {
    width: 18px;
    height: 18px;
  }
  .nav-item.active .nav-icon { color: var(--accent); }
  .nav-label { flex: 1; }

  .distro-info {
    padding: 14px 18px 18px;
    border-top: 1px solid var(--border-subtle);
    background: var(--bg-canvas);
  }
  .distro-status {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }
  .status-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--success);
    box-shadow: 0 0 8px var(--success);
    flex-shrink: 0;
  }
  .distro-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .distro-detail {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--text-muted);
    margin-left: 15px;
  }
  .distro-detail-label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-faint);
  }
  .distro-detail code {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--teal);
    background: var(--teal-soft);
    padding: 1px 6px;
    border-radius: 4px;
  }

  /* ================ CONTENT ================ */
  .content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg-base);
    position: relative;
  }
  .view-pane {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    overscroll-behavior: contain;
    /* Tell the compositor each pane is an isolated subtree so paints
       and layouts don't escape and so off-screen areas can be skipped. */
    contain: layout paint style;
    padding: 32px 40px;
  }
  .view-pane[hidden] { display: none !important; }
  .content-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 28px;
    gap: 16px;
  }
  .content-header h2 {
    font-family: var(--font-display);
    font-size: var(--text-2xl);
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: -0.02em;
  }
  .subtitle { color: var(--text-muted); font-size: var(--text-base); margin-top: 6px; }
  .header-actions { display: flex; gap: 10px; flex-shrink: 0; }

  /* ================ BUTTONS ================ */
  .btn {
    padding: 10px 20px;
    border: none;
    border-radius: var(--radius-sm);
    font-size: var(--text-base);
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    transition: all var(--duration) var(--ease);
    white-space: nowrap;
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .btn-primary {
    background: linear-gradient(180deg, var(--accent-hover) 0%, var(--accent) 100%);
    color: #fff;
    box-shadow: 0 1px 0 rgba(255, 255, 255, 0.15) inset, 0 4px 14px rgba(255, 107, 53, 0.3);
  }
  .btn-primary:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 1px 0 rgba(255, 255, 255, 0.2) inset, 0 6px 20px rgba(255, 107, 53, 0.4);
  }
  .btn-primary:active:not(:disabled) { transform: translateY(0); }
  .btn-secondary {
    background: var(--surface-2);
    color: var(--text-primary);
    border: 1px solid var(--border-default);
  }
  .btn-secondary:hover:not(:disabled) {
    background: var(--surface-3);
    border-color: var(--border-strong);
  }
  .btn-link {
    background: none;
    border: none;
    color: var(--accent);
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
    font-family: inherit;
    padding: 4px 8px;
    margin-left: auto;
    transition: color var(--duration) var(--ease);
  }
  .btn-link:hover { color: var(--accent-hover); }

  /* ================ RESULTS BANNER + CONSOLE ================ */
  .console-wrapper { margin-bottom: 22px; }
  .results-banner {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px 20px;
    background: var(--success-bg);
    border: 1px solid var(--success-border);
    border-radius: var(--radius-md);
  }
  .results-banner.console-visible {
    border-radius: var(--radius-md) var(--radius-md) 0 0;
    border-bottom-color: var(--border-subtle);
  }
  .results-icon { font-size: 22px; line-height: 1; }
  .results-text { flex: 1; }
  .results-banner strong { color: var(--success); font-size: 14px; font-weight: 600; }
  .results-banner p { font-size: 13px; color: var(--text-secondary); margin-top: 4px; }
  .warning { color: var(--warning); }

  .btn-console {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    border: 1px solid var(--success-border);
    border-radius: var(--radius-sm);
    background: rgba(74, 222, 128, 0.08);
    color: var(--success);
    font-size: 13px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    transition: all var(--duration) var(--ease);
    white-space: nowrap;
  }
  .btn-console:hover { background: rgba(74, 222, 128, 0.15); }
  .console-count {
    background: rgba(74, 222, 128, 0.2);
    color: var(--success);
    font-size: 11px;
    font-weight: 600;
    padding: 1px 7px;
    border-radius: var(--radius-full);
    font-variant-numeric: tabular-nums;
  }

  .console-panel {
    border: 1px solid var(--border-subtle);
    border-top: none;
    border-radius: 0 0 var(--radius-md) var(--radius-md);
    background: var(--surface-1);
    margin-bottom: 22px;
    overflow: hidden;
  }
  .console-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--surface-2);
  }
  .console-filters { display: flex; gap: 4px; }
  .filter-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    transition: all var(--duration) var(--ease);
  }
  .filter-btn:hover { color: var(--text-secondary); background: var(--surface-1); }
  .filter-btn.active { background: var(--surface-3); color: var(--text-primary); border-color: var(--border-default); }
  .filter-count {
    font-size: 11px;
    color: var(--text-faint);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .filter-btn.active .filter-count { color: var(--text-secondary); }
  .console-summary {
    font-size: 12px;
    color: var(--success);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .console-log {
    max-height: 320px;
    overflow-y: auto;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.6;
  }
  .log-entry {
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding: 5px 16px;
    border-bottom: 1px solid var(--border-subtle);
    color: var(--text-secondary);
    transition: background 0.1s;
  }
  .log-entry:hover { background: var(--surface-2); }
  .log-entry:last-child { border-bottom: none; }
  .log-icon {
    flex-shrink: 0;
    width: 16px;
    text-align: center;
    font-weight: 700;
    color: var(--success);
  }
  .log-error .log-icon { color: var(--error); }
  .log-category {
    flex-shrink: 0;
    color: var(--accent);
    font-weight: 600;
    min-width: 120px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .log-path {
    flex: 1;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }
  .log-size {
    flex-shrink: 0;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    min-width: 70px;
    text-align: right;
  }
  .log-message {
    flex-shrink: 0;
    color: var(--error);
    font-size: 11px;
    max-width: 250px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .log-empty {
    padding: 24px;
    text-align: center;
    color: var(--text-faint);
    font-style: italic;
  }

  /* ================ EMPTY STATE ================ */
  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    min-height: 300px;
  }
  .empty-icon { font-size: 56px; margin-bottom: 20px; line-height: 1; opacity: 0.65; }
  .empty-state h3 {
    font-family: var(--font-display);
    font-size: 18px;
    color: var(--text-secondary);
    margin-bottom: 8px;
    font-weight: 600;
  }
  .empty-state p { font-size: 14px; }

  .spinner {
    width: 44px;
    height: 44px;
    border: 3px solid var(--surface-3);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin-bottom: 20px;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  /* ================ SUMMARY BAR ================ */
  .summary-bar {
    display: flex;
    align-items: center;
    gap: 36px;
    padding: 18px 24px;
    background: var(--surface-1);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    margin-bottom: 22px;
    box-shadow: var(--inset-line);
  }
  .summary-stat { display: flex; flex-direction: column; }
  .stat-value {
    font-family: var(--font-display);
    font-size: 22px;
    font-weight: 600;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.01em;
  }
  .stat-label {
    font-size: 11px;
    color: var(--text-muted);
    margin-top: 3px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-weight: 500;
  }

  /* ================ CATEGORY CARDS ================ */
  .categories {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 14px;
  }
  .category-card {
    padding: 18px;
    background: var(--surface-1);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    transition: border-color var(--duration) var(--ease), background var(--duration) var(--ease), transform var(--duration) var(--ease);
    cursor: pointer;
    user-select: none;
    -webkit-user-select: none;
    box-shadow: var(--inset-line);
  }
  .category-card:hover {
    border-color: var(--border-strong);
    background: var(--surface-2);
  }
  .category-card.selected {
    border-color: var(--accent);
    background: linear-gradient(180deg, var(--accent-soft) 0%, var(--surface-1) 100%);
    box-shadow: var(--inset-line), 0 0 0 1px var(--accent-soft);
  }
  .category-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 10px;
  }
  .checkbox-label { display: flex; align-items: center; gap: 10px; cursor: pointer; pointer-events: none; }
  .checkbox-label input[type="checkbox"] {
    width: 16px;
    height: 16px;
    cursor: pointer;
  }
  .category-name {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .category-size {
    font-family: var(--font-display);
    font-size: 16px;
    font-weight: 600;
    color: var(--accent);
    font-variant-numeric: tabular-nums;
  }
  .category-desc {
    font-size: 12px;
    color: var(--text-muted);
    margin-bottom: 10px;
    line-height: 1.55;
  }
  .category-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .category-meta {
    font-size: 11px;
    color: var(--text-faint);
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .expand-btn {
    background: none;
    border: none;
    color: var(--accent);
    font-size: 11px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    transition: all var(--duration) var(--ease);
  }
  .expand-btn:hover { color: var(--accent-hover); background: var(--accent-soft); }

  .details-panel {
    grid-column: 1 / -1;
    background: var(--surface-1);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    overflow: hidden;
  }
  .details-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 18px;
    background: var(--surface-2);
    border-bottom: 1px solid var(--border-subtle);
  }
  .details-title {
    font-family: var(--font-display);
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .details-count {
    font-size: 12px;
    color: var(--text-muted);
    flex: 1;
  }

  .item-list {
    margin: 0;
    padding: 6px 18px 10px;
    max-height: 260px;
    overflow-y: auto;
    font-family: var(--font-mono);
    font-size: 11px;
  }
  .item-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 0;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border-subtle);
  }
  .item-row:last-child { border-bottom: none; }
  .item-icon { flex-shrink: 0; font-size: 12px; line-height: 1; }
  .item-path {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-primary);
    direction: rtl;
    text-align: left;
  }
  .item-size {
    flex-shrink: 0;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    min-width: 60px;
    text-align: right;
  }
  .item-more {
    padding: 8px 0 4px;
    text-align: center;
    color: var(--text-faint);
    font-style: italic;
    font-size: 11px;
  }
</style>
