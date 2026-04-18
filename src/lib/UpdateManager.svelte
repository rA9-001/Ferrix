<script>
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  let checkResult = $state(null);
  let checking = $state(false);
  let updating = $state(false);
  let consoleOpen = $state(false);
  let consoleLines = $state([]);
  let updateDone = $state(false);
  let consoleBodyEl = $state(null);
  let searchQuery = $state("");

  const filteredUpdates = $derived.by(() => {
    if (!checkResult || !searchQuery) return checkResult?.updates || [];
    const q = searchQuery.toLowerCase();
    return checkResult.updates.filter((u) => u.name.toLowerCase().includes(q));
  });

  const filteredFlatpak = $derived.by(() => {
    if (!checkResult || !searchQuery) return checkResult?.flatpak_updates || [];
    const q = searchQuery.toLowerCase();
    return checkResult.flatpak_updates.filter((f) => f.toLowerCase().includes(q));
  });

  $effect(() => {
    if (consoleLines.length && consoleBodyEl) {
      consoleBodyEl.scrollTop = consoleBodyEl.scrollHeight;
    }
  });

  async function checkForUpdates() {
    checking = true;
    checkResult = null;
    updateDone = false;
    try {
      checkResult = await invoke("check_updates");
    } finally {
      checking = false;
    }
  }

  async function runUpdate() {
    if (!checkResult) return;
    updating = true;
    consoleLines = [];
    consoleOpen = true;
    updateDone = false;

    const hasFlatpak = checkResult.flatpak_updates.length > 0;
    const unlisten = await listen("update-output", (event) => {
      consoleLines = [...consoleLines, event.payload];
    });

    try {
      await invoke("apply_updates", { updateFlatpak: hasFlatpak });
      updateDone = true;
      // Re-check after update
      checkResult = await invoke("check_updates");
    } finally {
      updating = false;
      unlisten();
    }
  }

  const totalCount = $derived.by(() => {
    if (!checkResult) return 0;
    return checkResult.updates.length + checkResult.flatpak_updates.length;
  });

  $effect(() => {
    checkForUpdates();
  });
</script>

<div class="update-manager">
  <header class="header">
    <div>
      <h2>System Update Manager</h2>
      <p class="subtitle">Check and apply system-wide updates</p>
    </div>
    <div class="header-actions">
      <button class="btn btn-secondary" onclick={checkForUpdates} disabled={checking || updating}>
        {checking ? "Checking..." : "Check for Updates"}
      </button>
      {#if totalCount > 0}
        <button class="btn btn-primary" onclick={runUpdate} disabled={updating || checking}>
          {updating ? "Updating..." : `Update All (${totalCount})`}
        </button>
      {/if}
    </div>
  </header>

  {#if checking && !checkResult}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>Checking for updates...</p>
      <p class="hint">Syncing package databases</p>
    </div>
  {:else if checkResult}
    <!-- Summary Banner -->
    <div class="summary-banner" class:up-to-date={totalCount === 0} class:has-updates={totalCount > 0}>
      <div class="summary-icon">{#if totalCount === 0}✓{:else}{@html '<svg style="width:1em;height:1em;vertical-align:middle" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="19" x2="12" y2="5"/><polyline points="5 12 12 5 19 12"/></svg>'}{/if}</div>
      <div class="summary-text">
        <strong>{checkResult.summary}</strong>
        {#if updateDone}
          <span class="done-badge">Update complete</span>
        {/if}
      </div>
      {#if checking}
        <div class="spinner small"></div>
      {/if}
    </div>

    {#if totalCount > 0}
      <div class="search-bar">
        <span class="search-icon">{@html '<svg style="width:1em;height:1em;vertical-align:middle" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>'}</span>
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Search packages..."
          class="search-input"
        />
        {#if searchQuery}
          <button class="search-clear" onclick={() => (searchQuery = "")}>✕</button>
        {/if}
      </div>
    {/if}

    {#if filteredUpdates.length > 0}
      <div class="update-section">
        <h3 class="section-title">
          <span class="section-icon">{@html '<svg style="width:1em;height:1em;vertical-align:middle" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><polyline points="3.27 6.96 12 12.01 20.73 6.96"/><line x1="12" y1="22.08" x2="12" y2="12"/></svg>'}</span>
          System Packages
          <span class="section-count">{filteredUpdates.length}</span>
        </h3>
        <div class="update-table">
          <div class="table-header">
            <span class="col-name">Package</span>
            <span class="col-ver">Current</span>
            <span class="col-arrow"></span>
            <span class="col-ver">New</span>
          </div>
          {#each filteredUpdates as upd}
            <div class="table-row">
              <span class="col-name">{upd.name}</span>
              <span class="col-ver old">{upd.current_version || "—"}</span>
              <span class="col-arrow">→</span>
              <span class="col-ver new">{upd.new_version}</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    {#if filteredFlatpak.length > 0}
      <div class="update-section">
        <h3 class="section-title">
          <span class="section-icon">{@html '<svg style="width:1em;height:1em;vertical-align:middle" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/><rect x="8" y="2" width="8" height="4" rx="1" ry="1"/></svg>'}</span>
          Flatpak Apps
          <span class="section-count">{filteredFlatpak.length}</span>
        </h3>
        <div class="flatpak-grid">
          {#each filteredFlatpak as fid}
            <div class="flatpak-chip">{fid}</div>
          {/each}
        </div>
      </div>
    {/if}

    {#if searchQuery && filteredUpdates.length === 0 && filteredFlatpak.length === 0}
      <div class="empty-state">
        <p>No packages matching "{searchQuery}"</p>
      </div>
    {:else if totalCount === 0 && !consoleOpen}
      <div class="up-to-date-hero">
        <span class="hero-icon"><svg style="width:48px;height:48px" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg></span>
        <p>Your system is fully up to date!</p>
      </div>
    {/if}
  {/if}

  {#if consoleOpen}
    <div class="output-panel">
      <div class="console-header">
        <h3>Update Output</h3>
        <button class="btn-close" onclick={() => (consoleOpen = false)}>✕</button>
      </div>
      <div class="console-body" bind:this={consoleBodyEl}>
        {#if consoleLines.length > 0}
          <pre class="console-live">{consoleLines.join('\n')}</pre>
        {/if}
        {#if updating}
          <div class="console-waiting">
            <div class="spinner small"></div>
            <span>Applying updates...</span>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .update-manager {
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
  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
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
  .btn-primary {
    background: rgba(56, 189, 248, 0.15);
    color: #38bdf8;
    border-color: rgba(56, 189, 248, 0.4);
  }
  .btn-primary:hover:not(:disabled) {
    background: rgba(56, 189, 248, 0.25);
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 4rem;
    color: #94a3b8;
  }
  .hint {
    font-size: 0.8rem;
    color: #64748b;
    margin-top: 0.25rem;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid rgba(56, 189, 248, 0.2);
    border-top-color: #38bdf8;
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

  /* Search bar */
  .search-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background: rgba(15, 23, 42, 0.4);
    border: 1px solid rgba(148, 163, 184, 0.1);
    border-radius: 8px;
    margin-bottom: 1rem;
  }
  .search-icon {
    font-size: 0.9rem;
    color: #64748b;
  }
  .search-input {
    flex: 1;
    background: none;
    border: none;
    color: #e2e8f0;
    font-size: 0.85rem;
    outline: none;
  }
  .search-input::placeholder {
    color: #475569;
  }
  .search-clear {
    background: none;
    border: none;
    color: #64748b;
    cursor: pointer;
    font-size: 0.85rem;
    padding: 0.2rem;
  }
  .search-clear:hover {
    color: #e2e8f0;
  }
  .empty-state {
    text-align: center;
    padding: 2rem;
    color: #64748b;
    font-size: 0.9rem;
  }

  /* Summary banner */
  .summary-banner {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem 1.25rem;
    border-radius: 10px;
    margin-bottom: 1.25rem;
  }
  .summary-banner.up-to-date {
    background: rgba(34, 197, 94, 0.08);
    border: 1px solid rgba(34, 197, 94, 0.2);
  }
  .summary-banner.has-updates {
    background: rgba(56, 189, 248, 0.08);
    border: 1px solid rgba(56, 189, 248, 0.2);
  }
  .summary-icon {
    font-size: 1.5rem;
  }
  .summary-banner.up-to-date .summary-icon {
    color: #4ade80;
  }
  .summary-banner.has-updates .summary-icon {
    color: #38bdf8;
  }
  .summary-text {
    flex: 1;
    color: #e2e8f0;
    font-size: 0.95rem;
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }
  .done-badge {
    font-size: 0.75rem;
    padding: 0.2rem 0.6rem;
    border-radius: 999px;
    background: rgba(34, 197, 94, 0.15);
    color: #4ade80;
    font-weight: 500;
  }

  /* Sections */
  .update-section {
    margin-bottom: 1.25rem;
  }
  .section-title {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 1rem;
    color: #cbd5e1;
    margin: 0 0 0.6rem 0;
  }
  .section-icon {
    font-size: 1.1rem;
  }
  .section-count {
    font-size: 0.75rem;
    padding: 0.15rem 0.5rem;
    border-radius: 999px;
    background: rgba(56, 189, 248, 0.15);
    color: #38bdf8;
  }

  /* Update table */
  .update-table {
    background: rgba(15, 23, 42, 0.4);
    border: 1px solid rgba(148, 163, 184, 0.1);
    border-radius: 8px;
    overflow: hidden;
    max-height: 45vh;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  .table-header {
    display: grid;
    grid-template-columns: 1fr 150px 30px 150px;
    padding: 0.5rem 1rem;
    background: rgba(30, 41, 59, 0.4);
    border-bottom: 1px solid rgba(148, 163, 184, 0.1);
    font-size: 0.75rem;
    color: #64748b;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .table-row {
    display: grid;
    grid-template-columns: 1fr 150px 30px 150px;
    padding: 0.45rem 1rem;
    border-bottom: 1px solid rgba(148, 163, 184, 0.05);
    font-size: 0.85rem;
    align-items: center;
  }
  .table-row:last-child {
    border-bottom: none;
  }
  .table-row:hover {
    background: rgba(56, 189, 248, 0.04);
  }
  .col-name {
    color: #e2e8f0;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .col-ver {
    font-family: monospace;
    font-size: 0.78rem;
  }
  .col-ver.old {
    color: #64748b;
  }
  .col-ver.new {
    color: #38bdf8;
    font-weight: 500;
  }
  .col-arrow {
    color: #475569;
    text-align: center;
  }

  /* Flatpak grid */
  .flatpak-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
  }
  .flatpak-chip {
    padding: 0.35rem 0.75rem;
    background: rgba(30, 41, 59, 0.5);
    border: 1px solid rgba(148, 163, 184, 0.1);
    border-radius: 6px;
    font-size: 0.78rem;
    color: #94a3b8;
    font-family: monospace;
  }

  /* Up-to-date hero */
  .up-to-date-hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 3rem;
    color: #94a3b8;
  }
  .hero-icon {
    font-size: 3rem;
    margin-bottom: 0.75rem;
  }
  .up-to-date-hero p {
    font-size: 1.1rem;
    color: #cbd5e1;
    margin: 0;
  }

  /* Console panel */
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
    flex: 1;
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
  .console-waiting {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.5rem 0;
    color: #94a3b8;
    font-size: 0.85rem;
  }
</style>
