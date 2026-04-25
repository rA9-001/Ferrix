<script>
  import { invoke } from "@tauri-apps/api/core";

  let entries = $state([]);
  let loading = $state(false);
  let units = $state([]);
  let boots = $state([]);

  // Filters
  let filterPriority = $state("");
  let filterUnit = $state("");
  let filterBoot = $state("");
  let filterGrep = $state("");
  let filterSince = $state("");
  let filterLines = $state(200);

  let expandedEntry = $state(null);
  let unitSearch = $state("");
  let visibleCount = $state(100);

  const filteredUnits = $derived.by(() => {
    if (!unitSearch) return units;
    const q = unitSearch.toLowerCase();
    return units.filter((u) => u.name.toLowerCase().includes(q));
  });

  const priorityColors = {
    emerg: { bg: "rgba(239,68,68,0.2)", fg: "#f87171", label: "EMRG" },
    alert: { bg: "rgba(239,68,68,0.15)", fg: "#fb923c", label: "ALRT" },
    crit: { bg: "rgba(239,68,68,0.12)", fg: "#f87171", label: "CRIT" },
    err: { bg: "rgba(248,113,113,0.1)", fg: "#f87171", label: "ERR" },
    warning: { bg: "rgba(250,204,21,0.1)", fg: "#fbbf24", label: "WARN" },
    notice: { bg: "rgba(56,189,248,0.08)", fg: "#38bdf8", label: "NOTE" },
    info: { bg: "rgba(148,163,184,0.06)", fg: "#94a3b8", label: "INFO" },
    debug: { bg: "rgba(148,163,184,0.04)", fg: "#64748b", label: "DBG" },
  };

  async function loadMeta() {
    const [u, b] = await Promise.all([
      invoke("get_journal_units"),
      invoke("get_boot_list"),
    ]);
    units = u;
    boots = b;
  }

  async function queryLogs() {
    loading = true;
    expandedEntry = null;
    visibleCount = 100;
    try {
      const result = await invoke("query_logs", {
        priority: filterPriority,
        unit: filterUnit,
        boot: filterBoot,
        lines: filterLines,
        grep: filterGrep,
        since: filterSince,
      });
      entries = result.entries;
    } finally {
      loading = false;
    }
  }

  function resetFilters() {
    filterPriority = "";
    filterUnit = "";
    filterBoot = "";
    filterGrep = "";
    filterSince = "";
    filterLines = 200;
    queryLogs();
  }

  let metaLoaded = $state(false);

  $effect(() => {
    loadMeta().then(() => {
      metaLoaded = true;
      queryLogs();
    });
  });

  // Auto-apply filters when any select/dropdown changes
  $effect(() => {
    // Track all filter values to trigger re-query
    filterPriority; filterUnit; filterBoot;
    if (metaLoaded) queryLogs();
  });
</script>

<div class="log-viewer">
  <header class="header">
    <div>
      <h2>Log Viewer</h2>
      <p class="subtitle">Browse system journal logs</p>
    </div>
    <div class="header-actions">
      <button class="btn btn-muted" onclick={resetFilters} disabled={loading}>Reset</button>
      <button class="btn btn-accent" onclick={queryLogs} disabled={loading}>
        {loading ? "Loading..." : "Refresh"}
      </button>
    </div>
  </header>

  <!-- Filters -->
  <div class="filters">
    <div class="filter-group">
      <label for="log-priority">Priority</label>
      <select id="log-priority" bind:value={filterPriority}>
        <option value="">All</option>
        <option value="0">Emergency</option>
        <option value="1">Alert</option>
        <option value="2">Critical</option>
        <option value="3">Error</option>
        <option value="4">Warning</option>
        <option value="5">Notice</option>
        <option value="6">Info</option>
        <option value="7">Debug</option>
      </select>
    </div>

    <div class="filter-group">
      <label for="log-unit">Unit</label>
      <select id="log-unit" bind:value={filterUnit}>
        <option value="">All units</option>
        {#each filteredUnits as u}
          <option value={u.name}>{u.name}</option>
        {/each}
      </select>
    </div>

    <div class="filter-group">
      <label for="log-boot">Boot</label>
      <select id="log-boot" bind:value={filterBoot}>
        <option value="">Current</option>
        {#each boots as b}
          <option value={b}>{b}</option>
        {/each}
      </select>
    </div>

    <div class="filter-group">
      <label for="log-search">Search</label>
      <input
        id="log-search"
        type="text"
        bind:value={filterGrep}
        placeholder="grep pattern..."
        maxlength="200"
      />
    </div>

    <div class="filter-group">
      <label for="log-since">Since</label>
      <input
        id="log-since"
        type="text"
        bind:value={filterSince}
        placeholder="e.g. -1h, today"
        maxlength="30"
      />
    </div>

    <div class="filter-group filter-lines">
      <label for="log-lines">Lines</label>
      <input
        id="log-lines"
        type="number"
        bind:value={filterLines}
        min="10"
        max="5000"
      />
    </div>
  </div>

  <!-- Results -->
  <div class="results-bar">
    <span class="result-count">{entries.length} entries</span>
  </div>

  {#if loading}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>Querying journal...</p>
    </div>
  {:else if entries.length === 0}
    <div class="empty-state">
      <span class="empty-icon"><svg style="width:48px;height:48px" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="8" y1="13" x2="16" y2="13"/><line x1="8" y1="17" x2="16" y2="17"/></svg></span>
      <p>No log entries match your filters</p>
    </div>
  {:else}
    <div class="log-list">
      {#each entries.slice(0, visibleCount) as entry, idx}
        {@const pc = priorityColors[entry.priority] || priorityColors.info}
        <div class="log-entry-wrapper" class:expanded={expandedEntry === idx}>
          <button
            class="log-entry"
            style="border-left: 3px solid {pc.fg};"
            onclick={() => (expandedEntry = expandedEntry === idx ? null : idx)}
          >
            <div class="entry-main">
              <span class="entry-badge" style="background:{pc.bg}; color:{pc.fg};">{pc.label}</span>
              <span class="entry-time">{entry.timestamp}</span>
              <span class="entry-unit">{entry.unit || "—"}</span>
              <span class="entry-msg">{entry.message}</span>
            </div>
          </button>
          {#if expandedEntry === idx}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="entry-detail" onmousedown={(e) => e.stopPropagation()}>
              <div class="detail-row"><span class="dl">Timestamp</span><span class="dv">{entry.timestamp}</span></div>
              <div class="detail-row"><span class="dl">Priority</span><span class="dv" style="color:{pc.fg};">{entry.priority}</span></div>
              <div class="detail-row"><span class="dl">Unit</span><span class="dv">{entry.unit || "—"}</span></div>
              <div class="detail-row"><span class="dl">Message</span><span class="dv msg">{entry.message}</span></div>
            </div>
          {/if}
        </div>
      {/each}
      {#if visibleCount < entries.length}
        <button class="btn-show-more" onclick={() => (visibleCount += 200)}>
          Show more ({entries.length - visibleCount} remaining)
        </button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .log-viewer {
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
  .btn-muted {
    background: rgba(148, 163, 184, 0.08);
    color: #64748b;
    border-color: rgba(148, 163, 184, 0.15);
  }
  .btn-muted:hover:not(:disabled) {
    background: rgba(148, 163, 184, 0.15);
    color: #94a3b8;
  }
  .btn-accent {
    background: rgba(168, 85, 247, 0.15);
    color: #c084fc;
    border-color: rgba(168, 85, 247, 0.35);
  }
  .btn-accent:hover:not(:disabled) {
    background: rgba(168, 85, 247, 0.25);
  }

  /* Filters */
  .filters {
    display: flex;
    gap: 0.6rem;
    flex-wrap: wrap;
    margin-bottom: 0.75rem;
    padding: 0.75rem;
    background: rgba(15, 23, 42, 0.35);
    border: 1px solid rgba(148, 163, 184, 0.08);
    border-radius: 10px;
  }
  .filter-group {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    flex: 1;
    min-width: 120px;
  }
  .filter-lines {
    max-width: 90px;
    flex: 0;
  }
  .filter-group label {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #64748b;
    font-weight: 600;
  }
  .filter-group select,
  .filter-group input {
    background: rgba(30, 41, 59, 0.6);
    border: 1px solid rgba(148, 163, 184, 0.12);
    border-radius: 6px;
    color: #e2e8f0;
    padding: 0.4rem 0.6rem;
    font-size: 0.8rem;
  }
  .filter-group select {
    appearance: none;
    -webkit-appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' fill='%2394a3b8' viewBox='0 0 16 16'%3E%3Cpath d='M8 11L3 6h10z'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.5rem center;
    padding-right: 1.5rem;
    cursor: pointer;
  }
  .filter-group select option {
    background: #1e293b;
    color: #e2e8f0;
  }
  .filter-group select:focus,
  .filter-group input:focus {
    outline: none;
    border-color: rgba(168, 85, 247, 0.4);
  }

  .results-bar {
    display: flex;
    align-items: center;
    margin-bottom: 0.5rem;
  }
  .result-count {
    font-size: 0.78rem;
    color: #64748b;
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 4rem;
    color: #94a3b8;
  }
  .spinner {
    width: 28px;
    height: 28px;
    border: 3px solid rgba(168, 85, 247, 0.2);
    border-top-color: #c084fc;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin-bottom: 0.75rem;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 3rem;
    color: #94a3b8;
  }
  .empty-icon {
    font-size: 2.5rem;
    margin-bottom: 0.5rem;
  }

  /* Log list */
  .log-list {
    overflow-y: auto;
    overscroll-behavior: contain;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .btn-show-more {
    margin: 8px auto;
    padding: 8px 24px;
    background: rgba(255, 107, 53, 0.1);
    color: #ff6b35;
    border: 1px solid rgba(255, 107, 53, 0.25);
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.82rem;
    font-weight: 500;
    transition: background 0.15s;
  }
  .btn-show-more:hover {
    background: rgba(255, 107, 53, 0.2);
  }
  .log-entry-wrapper {
    background: rgba(15, 23, 42, 0.3);
    border-radius: 4px;
    transition: background 0.15s;
    content-visibility: auto;
    contain-intrinsic-size: auto 32px;
  }
  .log-entry-wrapper:hover {
    background: rgba(30, 41, 59, 0.5);
  }
  .log-entry-wrapper.expanded {
    background: rgba(30, 41, 59, 0.6);
  }
  .log-entry {
    background: none;
    border: none;
    border-radius: 4px;
    padding: 0.4rem 0.6rem 0.4rem 0.75rem;
    text-align: left;
    cursor: pointer;
    width: 100%;
    color: inherit;
    font-family: inherit;
  }
  .entry-main {
    display: grid;
    grid-template-columns: 48px 160px 180px 1fr;
    gap: 0.5rem;
    align-items: center;
    font-size: 0.78rem;
  }
  .entry-badge {
    font-size: 0.68rem;
    font-weight: 700;
    padding: 0.15rem 0.35rem;
    border-radius: 4px;
    text-align: center;
    font-family: monospace;
  }
  .entry-time {
    color: #64748b;
    font-family: monospace;
    font-size: 0.75rem;
  }
  .entry-unit {
    color: #c084fc;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.78rem;
  }
  .entry-msg {
    color: #cbd5e1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .entry-detail {
    margin-top: 0;
    padding: 0.5rem 0.6rem 0.5rem 0.75rem;
    border-top: 1px solid rgba(148, 163, 184, 0.08);
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    user-select: text;
    -webkit-user-select: text;
    cursor: text;
  }
  .detail-row {
    display: flex;
    gap: 0.75rem;
    font-size: 0.78rem;
  }
  .dl {
    color: #64748b;
    width: 80px;
    flex-shrink: 0;
    font-weight: 600;
  }
  .dv {
    color: #e2e8f0;
    word-break: break-word;
  }
  .dv.msg {
    white-space: pre-wrap;
    font-family: monospace;
    font-size: 0.75rem;
  }
</style>
