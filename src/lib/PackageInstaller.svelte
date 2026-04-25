<script>
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import ConfirmDialog from "./ConfirmDialog.svelte";

  let confirmRemoveOpen = $state(false);

  let catalog = $state([]);
  let statuses = $state({});
  let loading = $state(true);
  let selected = $state(new Set());
  let removeSelected = $state(new Set());
  let installing = $state(false);
  let removing = $state(false);
  let lastAction = $state("install");
  let installResults = $state([]);
  let consoleOpen = $state(false);
  let consoleLines = $state([]);
  let search = $state("");
  let activeCategory = $state("All");
  let distroInfo = $state(null);
  let consoleBodyEl = $state(null);

  const categories = $derived.by(() => {
    const cats = [...new Set(catalog.map((p) => p.category))];
    return ["All", ...cats];
  });

  const filtered = $derived.by(() => {
    let items = catalog;
    if (activeCategory !== "All") {
      items = items.filter((p) => p.category === activeCategory);
    }
    if (search.trim()) {
      const q = search.toLowerCase();
      items = items.filter(
        (p) =>
          p.name.toLowerCase().includes(q) ||
          p.description.toLowerCase().includes(q) ||
          p.category.toLowerCase().includes(q)
      );
    }
    return items;
  });

  const groupedFiltered = $derived.by(() => {
    const groups = {};
    for (const pkg of filtered) {
      if (!groups[pkg.category]) groups[pkg.category] = [];
      groups[pkg.category].push(pkg);
    }
    return groups;
  });

  const selectedCount = $derived(selected.size);
  const removeCount = $derived(removeSelected.size);

  const selectedPackages = $derived.by(() => {
    return catalog.filter((p) => selected.has(p.id));
  });

  const removePackages = $derived.by(() => {
    return catalog.filter((p) => removeSelected.has(p.id));
  });

  $effect(() => {
    if (consoleLines.length && consoleBodyEl) {
      consoleBodyEl.scrollTop = consoleBodyEl.scrollHeight;
    }
  });

  async function load() {
    loading = true;
    try {
      distroInfo = await invoke("get_distro_info");
      catalog = await invoke("get_package_catalog");
      const statusList = await invoke("check_packages_installed", {
        packageManager: distroInfo.package_manager,
      });
      const map = {};
      for (const s of statusList) {
        map[s.id] = s.installed;
      }
      statuses = map;
    } finally {
      loading = false;
    }
  }

  function toggleSelect(id) {
    const next = new Set(selected);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    selected = next;
  }

  function toggleRemoveSelect(id) {
    const next = new Set(removeSelected);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    removeSelected = next;
  }

  function selectAllVisible() {
    const next = new Set(selected);
    for (const pkg of filtered) {
      if (!statuses[pkg.id]) {
        next.add(pkg.id);
      }
    }
    selected = next;
  }

  function deselectAll() {
    selected = new Set();
    removeSelected = new Set();
  }

  async function installSelected() {
    if (selected.size === 0) return;
    installing = true;
    lastAction = "install";
    installResults = [];
    consoleLines = [];
    consoleOpen = true;

    const unlisten = await listen("install-output", (event) => {
      consoleLines = [...consoleLines, event.payload];
    });

    try {
      const ids = Array.from(selected);
      const results = await invoke("install_packages", { packageIds: ids });
      installResults = results;
      const statusList = await invoke("check_packages_installed", {
        packageManager: distroInfo.package_manager,
      });
      const map = {};
      for (const s of statusList) {
        map[s.id] = s.installed;
      }
      statuses = map;
      const next = new Set(selected);
      for (const r of results) {
        if (r.success) next.delete(r.package_id);
      }
      selected = next;
    } finally {
      installing = false;
      unlisten();
    }
  }

  async function removeInstalledPackages() {
    if (removeSelected.size === 0) return;
    confirmRemoveOpen = true;
  }

  async function performRemove() {
    removing = true;
    lastAction = "remove";
    installResults = [];
    consoleLines = [];
    consoleOpen = true;

    const unlisten = await listen("remove-output", (event) => {
      consoleLines = [...consoleLines, event.payload];
    });

    try {
      const ids = Array.from(removeSelected);
      const results = await invoke("remove_packages", { packageIds: ids });
      installResults = results;
      const statusList = await invoke("check_packages_installed", {
        packageManager: distroInfo.package_manager,
      });
      const map = {};
      for (const s of statusList) {
        map[s.id] = s.installed;
      }
      statuses = map;
      const next = new Set(removeSelected);
      for (const r of results) {
        if (r.success) next.delete(r.package_id);
      }
      removeSelected = next;
    } finally {
      removing = false;
      unlisten();
    }
  }

  function getInstallCmd(pkg) {
    if (!distroInfo) return null;
    const pm = distroInfo.package_manager;
    const native = pkg.packages[pm];
    if (native) {
      const cmdMap = {
        pacman: `pkexec pacman -S --noconfirm ${native}`,
        apt: `pkexec apt-get install -y ${native}`,
        dnf: `pkexec dnf install -y ${native}`,
        zypper: `pkexec zypper install -y ${native}`,
        xbps: `pkexec xbps-install -y ${native}`,
        apk: `pkexec apk add ${native}`,
      };
      return { method: pm, cmd: cmdMap[pm] || null, name: native };
    }
    if (pkg.packages.flatpak) {
      return { method: "flatpak", cmd: `flatpak install -y flathub ${pkg.packages.flatpak}`, name: pkg.packages.flatpak };
    }
    return null;
  }

  function getRemoveCmd(pkg) {
    if (!distroInfo) return null;
    const pm = distroInfo.package_manager;
    const native = pkg.packages[pm];
    if (native) {
      const cmdMap = {
        pacman: `pkexec pacman -Rs --noconfirm ${native}`,
        apt: `pkexec apt-get remove -y ${native}`,
        dnf: `pkexec dnf remove -y ${native}`,
        zypper: `pkexec zypper remove -y ${native}`,
        xbps: `pkexec xbps-remove -y ${native}`,
        apk: `pkexec apk del ${native}`,
      };
      return { method: pm, cmd: cmdMap[pm] || null, name: native };
    }
    if (pkg.packages.flatpak) {
      return { method: "flatpak", cmd: `flatpak uninstall -y ${pkg.packages.flatpak}`, name: pkg.packages.flatpak };
    }
    return null;
  }

  function getPkgSource(pkg) {
    if (!distroInfo) return null;
    const pm = distroInfo.package_manager;
    const native = pkg.packages[pm];
    if (native) return `${pm}: ${native}`;
    if (pkg.packages.flatpak) return `flatpak: ${pkg.packages.flatpak}`;
    return null;
  }

  $effect(() => {
    load();
  });
</script>

<div class="pkg-installer" class:console-visible={consoleOpen}>
  <header class="header">
    <div>
      <h2>Package Manager</h2>
      <p class="subtitle">Browse, install, and remove applications for your system</p>
    </div>
    <div class="header-actions">
      <button class="btn btn-secondary" onclick={load} disabled={loading}>
        {loading ? "Loading..." : "Refresh"}
      </button>
    </div>
  </header>

  {#if distroInfo}
    <div class="info-banner">
      <span class="info-icon">ℹ</span>
      <span>
        Detected <strong>{distroInfo.name}</strong> using <strong>{distroInfo.package_manager}</strong>.
        Packages not available natively will be installed via Flatpak when possible.
      </span>
    </div>
  {/if}

  <div class="main-layout">
    <!-- Left: package browser -->
    <div class="browser-panel">
      <div class="controls-row">
        <div class="search-box">
          <span class="search-icon">{@html '<svg style="width:1em;height:1em;vertical-align:middle" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>'}</span>
          <input type="text" placeholder="Search packages..." bind:value={search} />
        </div>
        <div class="selection-actions">
          <button class="btn-sm" onclick={selectAllVisible}>Select all visible</button>
          <button class="btn-sm" onclick={deselectAll}>Clear selection</button>
        </div>
      </div>

      <div class="category-tabs">
        {#each categories as cat}
          <button
            class="cat-tab"
            class:active={activeCategory === cat}
            onclick={() => (activeCategory = cat)}
          >
            {cat}
          </button>
        {/each}
      </div>

      {#if loading}
        <div class="loading">
          <div class="spinner"></div>
          <p>Loading package catalog...</p>
        </div>
      {:else}
        <div class="package-grid-container">
          {#each Object.entries(groupedFiltered) as [category, pkgs]}
            <div class="category-section">
              <h3 class="category-title">{category}</h3>
              <div class="package-grid">
                {#each pkgs as pkg}
                  {@const isInstalled = statuses[pkg.id]}
                  {@const isSelected = selected.has(pkg.id)}
                  {@const isRemoveSelected = removeSelected.has(pkg.id)}
                  {@const pkgSource = getPkgSource(pkg)}
                  <button
                    class="package-card"
                    class:installed={isInstalled && !isRemoveSelected}
                    class:selected={isSelected}
                    class:remove-selected={isRemoveSelected}
                    onclick={() => isInstalled ? toggleRemoveSelect(pkg.id) : toggleSelect(pkg.id)}
                    disabled={installing || removing}
                  >
                    <div class="pkg-icon">{pkg.icon}</div>
                    <div class="pkg-info">
                      <span class="pkg-name">{pkg.name}</span>
                      <span class="pkg-desc">{pkg.description}</span>
                      {#if pkgSource}
                        <span class="pkg-source">{pkgSource}</span>
                      {/if}
                    </div>
                    <div class="pkg-status">
                      {#if isRemoveSelected}
                        <span class="badge remove">✕ Remove</span>
                      {:else if isInstalled}
                        <span class="badge installed">✓ Installed</span>
                      {:else if isSelected}
                        <span class="badge selected">Selected</span>
                      {/if}
                    </div>
                  </button>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Right: selection sidebar -->
    <aside class="selection-sidebar">
      <div class="sidebar-header">
        <h3>Selected ({selectedCount + removeCount})</h3>
      </div>
      {#if selectedCount === 0 && removeCount === 0}
        <div class="sidebar-empty">
          <p>Click packages to select them for installation or removal.</p>
        </div>
      {:else}
        <div class="sidebar-list">
          {#if selectedCount > 0}
            <div class="sidebar-section-label install-label">Install ({selectedCount})</div>
            {#each selectedPackages as pkg}
              {@const info = getInstallCmd(pkg)}
              <div class="sidebar-item">
                <div class="sidebar-item-top">
                  <span class="sidebar-icon">{pkg.icon}</span>
                  <span class="sidebar-name">{pkg.name}</span>
                  <button class="sidebar-remove" onclick={() => toggleSelect(pkg.id)}>✕</button>
                </div>
                {#if info}
                  <code class="sidebar-cmd">{info.cmd}</code>
                {/if}
              </div>
            {/each}
          {/if}
          {#if removeCount > 0}
            <div class="sidebar-section-label remove-label">Remove ({removeCount})</div>
            {#each removePackages as pkg}
              {@const info = getRemoveCmd(pkg)}
              <div class="sidebar-item sidebar-item-remove">
                <div class="sidebar-item-top">
                  <span class="sidebar-icon">{pkg.icon}</span>
                  <span class="sidebar-name">{pkg.name}</span>
                  <button class="sidebar-remove" onclick={() => toggleRemoveSelect(pkg.id)}>✕</button>
                </div>
                {#if info}
                  <code class="sidebar-cmd">{info.cmd}</code>
                {/if}
              </div>
            {/each}
          {/if}
        </div>
        <div class="sidebar-footer">
          {#if selectedCount > 0}
            <button class="btn btn-install" onclick={installSelected} disabled={installing || removing}>
              {installing ? "Installing..." : `Install ${selectedCount} package${selectedCount > 1 ? "s" : ""}`}
            </button>
          {/if}
          {#if removeCount > 0}
            <button class="btn btn-remove" onclick={removeInstalledPackages} disabled={installing || removing}>
              {removing ? "Removing..." : `Remove ${removeCount} package${removeCount > 1 ? "s" : ""}`}
            </button>
          {/if}
        </div>
      {/if}
    </aside>
  </div>

<ConfirmDialog
  bind:open={confirmRemoveOpen}
  title={`Remove ${removeCount} package${removeCount > 1 ? "s" : ""}?`}
  message={`The following will be uninstalled via pkexec:\n\n${removePackages.map((p) => p.name).join(", ")}`}
  confirmLabel="Remove"
  destructive
  onconfirm={performRemove}
/>

  <!-- Bottom: output console -->
  {#if consoleOpen}
    <div class="output-panel">
      <div class="console-header">
        <h3>{lastAction === "remove" ? "Removal" : "Installation"} Output</h3>
        <button class="btn-close" onclick={() => (consoleOpen = false)}>✕</button>
      </div>
      <div class="console-body" bind:this={consoleBodyEl}>
        {#if consoleLines.length > 0}
          <pre class="console-live">{consoleLines.join('\n')}</pre>
        {/if}
        {#if installing || removing}
          <div class="console-waiting">
            <div class="spinner small"></div>
            <span>{installing ? "Installing" : "Removing"} packages...</span>
          </div>
        {:else if installResults.length > 0}
          <div class="console-summary">
            {#each installResults as result}
              <div class="console-entry" class:success={result.success} class:fail={!result.success}>
                <div class="entry-header">
                  <span class="entry-icon">{result.success ? "✓" : "✗"}</span>
                  <strong>{result.package_id}</strong>
                  <span class="entry-method">via {result.method}</span>
                </div>
                <pre class="entry-output">{result.output}</pre>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .pkg-installer {
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 1rem;
  }

  .header h2 {
    font-size: 1.5rem;
    color: #f1f5f9;
    margin: 0 0 0.25rem 0;
  }

  .subtitle {
    color: #94a3b8;
    font-size: 0.85rem;
    margin: 0;
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .btn {
    padding: 0.5rem 1rem;
    border-radius: 8px;
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
    border: 1px solid transparent;
    transition: all 0.2s;
  }

  .btn-secondary {
    background: rgba(148, 163, 184, 0.1);
    color: #94a3b8;
    border-color: rgba(148, 163, 184, 0.2);
  }
  .btn-secondary:hover:not(:disabled) {
    background: rgba(148, 163, 184, 0.2);
    color: #cbd5e1;
  }

  .btn-install {
    background: rgba(236, 72, 153, 0.15);
    color: #f472b6;
    border: 1px solid rgba(236, 72, 153, 0.4);
    width: 100%;
  }
  .btn-install:hover:not(:disabled) {
    background: rgba(236, 72, 153, 0.25);
  }
  .btn-install:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-remove {
    background: rgba(239, 68, 68, 0.15);
    color: #f87171;
    border: 1px solid rgba(239, 68, 68, 0.4);
    width: 100%;
  }
  .btn-remove:hover:not(:disabled) {
    background: rgba(239, 68, 68, 0.25);
  }
  .btn-remove:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .sidebar-section-label {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 0.4rem 0.55rem 0.15rem;
  }
  .sidebar-section-label.install-label {
    color: #f472b6;
  }
  .sidebar-section-label.remove-label {
    color: #f87171;
  }

  .sidebar-item-remove {
    border-color: rgba(239, 68, 68, 0.15);
  }

  .info-banner {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    background: rgba(236, 72, 153, 0.06);
    border: 1px solid rgba(236, 72, 153, 0.15);
    border-radius: 8px;
    margin-bottom: 1rem;
    color: #94a3b8;
    font-size: 0.85rem;
  }
  .info-icon {
    font-size: 1.1rem;
    color: #f472b6;
  }
  .info-banner strong {
    color: #f472b6;
  }

  /* Main split layout */
  .main-layout {
    display: flex;
    gap: 1rem;
    flex: 1;
    min-height: 0;
  }

  .browser-panel {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  .selection-sidebar {
    width: 260px;
    flex-shrink: 0;
    background: rgba(15, 23, 42, 0.4);
    border: 1px solid rgba(148, 163, 184, 0.1);
    border-radius: 10px;
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 220px);
  }

  .sidebar-header {
    padding: 0.6rem 0.85rem;
    border-bottom: 1px solid rgba(148, 163, 184, 0.08);
  }
  .sidebar-header h3 {
    margin: 0;
    font-size: 0.95rem;
    color: #cbd5e1;
  }

  .sidebar-empty {
    padding: 1.5rem 0.85rem;
    text-align: center;
  }
  .sidebar-empty p {
    color: #64748b;
    font-size: 0.85rem;
    margin: 0;
  }

  .sidebar-list {
    flex: 1;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 0.4rem;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .sidebar-item {
    padding: 0.45rem 0.55rem;
    background: rgba(30, 41, 59, 0.4);
    border: 1px solid rgba(148, 163, 184, 0.08);
    border-radius: 6px;
  }

  .sidebar-item-top {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .sidebar-icon {
    font-size: 0.9rem;
  }

  .sidebar-name {
    flex: 1;
    font-size: 0.9rem;
    color: #e2e8f0;
    font-weight: 500;
  }

  .sidebar-remove {
    background: none;
    border: none;
    color: #64748b;
    font-size: 0.75rem;
    cursor: pointer;
    padding: 0 0.2rem;
    line-height: 1;
  }
  .sidebar-remove:hover {
    color: #f87171;
  }

  .sidebar-cmd {
    display: block;
    margin-top: 0.25rem;
    font-size: 0.7rem;
    color: #64748b;
    background: rgba(0, 0, 0, 0.2);
    padding: 0.2rem 0.4rem;
    border-radius: 4px;
    word-break: break-all;
    font-family: monospace;
  }

  /* Controls */
  .controls-row {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    margin-bottom: 0.75rem;
    flex-wrap: wrap;
  }

  .search-box {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    background: rgba(30, 41, 59, 0.5);
    border: 1px solid rgba(148, 163, 184, 0.15);
    border-radius: 8px;
    padding: 0.4rem 0.75rem;
    flex: 1;
    min-width: 200px;
  }
  .search-icon {
    font-size: 0.85rem;
  }
  .search-box input {
    background: none;
    border: none;
    color: #e2e8f0;
    font-size: 0.95rem;
    flex: 1;
    outline: none;
  }

  .selection-actions {
    display: flex;
    gap: 0.35rem;
  }

  .btn-sm {
    padding: 0.35rem 0.6rem;
    font-size: 0.85rem;
    cursor: pointer;
    background: rgba(148, 163, 184, 0.08);
    border: 1px solid rgba(148, 163, 184, 0.15);
    border-radius: 6px;
    color: #94a3b8;
    transition: all 0.2s;
  }
  .btn-sm:hover {
    background: rgba(148, 163, 184, 0.15);
    color: #cbd5e1;
  }

  .category-tabs {
    display: flex;
    gap: 0.25rem;
    flex-wrap: wrap;
    margin-bottom: 1rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid rgba(148, 163, 184, 0.1);
  }

  .cat-tab {
    padding: 0.35rem 0.7rem;
    font-size: 0.85rem;
    background: rgba(148, 163, 184, 0.06);
    border: 1px solid rgba(148, 163, 184, 0.1);
    border-radius: 6px;
    color: #94a3b8;
    cursor: pointer;
    transition: all 0.2s;
  }
  .cat-tab:hover {
    background: rgba(236, 72, 153, 0.1);
    color: #f472b6;
  }
  .cat-tab.active {
    background: rgba(236, 72, 153, 0.15);
    border-color: rgba(236, 72, 153, 0.4);
    color: #f472b6;
  }

  .loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 3rem;
    color: #94a3b8;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid rgba(236, 72, 153, 0.2);
    border-top-color: #ec4899;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin-bottom: 0.75rem;
  }
  .spinner.small {
    width: 18px;
    height: 18px;
    border-width: 2px;
    margin-bottom: 0;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* Package grid */
  .package-grid-container {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .category-section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .category-title {
    font-size: 1.05rem;
    color: #cbd5e1;
    margin: 0;
    padding-left: 0.25rem;
  }

  .package-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 0.5rem;
  }

  .package-card {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    padding: 0.6rem 0.75rem;
    background: rgba(30, 41, 59, 0.4);
    border: 1px solid rgba(148, 163, 184, 0.1);
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s;
    text-align: left;
    color: inherit;
    font: inherit;
    /* Skip painting off-screen cards while scrolling. */
    content-visibility: auto;
    contain-intrinsic-size: auto 56px;
  }
  .package-card:hover:not(:disabled) {
    background: rgba(236, 72, 153, 0.06);
    border-color: rgba(236, 72, 153, 0.25);
  }
  .package-card.selected {
    background: rgba(236, 72, 153, 0.1);
    border-color: rgba(236, 72, 153, 0.4);
  }
  .package-card.installed {
    opacity: 0.55;
    cursor: pointer;
  }
  .package-card.installed:hover:not(:disabled) {
    opacity: 0.75;
    border-color: rgba(239, 68, 68, 0.3);
    background: rgba(239, 68, 68, 0.06);
  }

  .package-card.remove-selected {
    opacity: 1;
    background: rgba(239, 68, 68, 0.1);
    border-color: rgba(239, 68, 68, 0.4);
  }

  .pkg-icon {
    font-size: 1.4rem;
    flex-shrink: 0;
    width: 2rem;
    text-align: center;
  }

  .pkg-info {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }

  .pkg-name {
    color: #e2e8f0;
    font-size: 1rem;
    font-weight: 500;
  }

  .pkg-desc {
    color: #94a3b8;
    font-size: 0.82rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .pkg-source {
    color: #64748b;
    font-size: 0.75rem;
    font-family: monospace;
  }

  .pkg-status {
    flex-shrink: 0;
  }

  .badge {
    font-size: 0.75rem;
    padding: 0.2rem 0.55rem;
    border-radius: 999px;
    font-weight: 500;
  }
  .badge.installed {
    background: rgba(34, 197, 94, 0.15);
    color: #4ade80;
  }
  .badge.selected {
    background: rgba(236, 72, 153, 0.15);
    color: #f472b6;
  }
  .badge.remove {
    background: rgba(239, 68, 68, 0.15);
    color: #f87171;
  }

  /* Bottom output panel */
  .output-panel {
    margin-top: 1rem;
    background: rgba(15, 23, 42, 0.6);
    border: 1px solid rgba(148, 163, 184, 0.12);
    border-radius: 10px;
    overflow: hidden;
    max-height: 45vh;
    display: flex;
    flex-direction: column;
  }

  .console-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.6rem 1rem;
    background: rgba(30, 41, 59, 0.3);
    border-bottom: 1px solid rgba(148, 163, 184, 0.08);
    flex-shrink: 0;
  }
  .console-header h3 {
    margin: 0;
    font-size: 0.85rem;
    color: #cbd5e1;
  }

  .btn-close {
    background: none;
    border: none;
    color: #64748b;
    font-size: 1rem;
    cursor: pointer;
  }
  .btn-close:hover {
    color: #e2e8f0;
  }

  .console-body {
    padding: 0.75rem 1rem;
    overflow-y: auto;
    overscroll-behavior: contain;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    flex: 1;
  }

  .console-waiting {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 1rem;
    color: #94a3b8;
    font-size: 0.85rem;
  }

  .console-entry {
    border-radius: 6px;
    padding: 0.5rem 0.75rem;
  }
  .console-entry.success {
    background: rgba(34, 197, 94, 0.06);
    border: 1px solid rgba(34, 197, 94, 0.15);
  }
  .console-entry.fail {
    background: rgba(239, 68, 68, 0.06);
    border: 1px solid rgba(239, 68, 68, 0.15);
  }

  .entry-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.35rem;
    font-size: 0.8rem;
    color: #e2e8f0;
  }

  .entry-icon {
    font-size: 0.9rem;
  }
  .console-entry.success .entry-icon {
    color: #4ade80;
  }
  .console-entry.fail .entry-icon {
    color: #f87171;
  }

  .entry-method {
    color: #64748b;
    font-size: 0.7rem;
  }

  .entry-output {
    margin: 0;
    font-size: 0.7rem;
    color: #94a3b8;
    white-space: pre-wrap;
    word-break: break-all;
    max-height: 150px;
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  .console-live {
    margin: 0;
    font-size: 0.78rem;
    color: #a5f3fc;
    white-space: pre-wrap;
    word-break: break-all;
    font-family: monospace;
    line-height: 1.5;
  }

  .console-summary {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .sidebar-footer {
    padding: 0.6rem;
    border-top: 1px solid rgba(148, 163, 184, 0.08);
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
</style>
