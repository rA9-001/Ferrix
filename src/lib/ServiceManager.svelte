<script>
  import { invoke } from "@tauri-apps/api/core";

  let services = $state([]);
  let loading = $state(true);
  let search = $state("");
  let filter = $state("all"); // "all" | "active" | "inactive" | "failed"
  let actionInProgress = $state(new Set());
  let message = $state(null);
  let messageType = $state("success");

  // Logs panel
  let logsService = $state(null);
  let logsContent = $state([]);
  let logsLoading = $state(false);

  // Sort
  let sortCol = $state("name");
  let sortAsc = $state(true);

  async function loadServices() {
    loading = true;
    try {
      services = await invoke("list_services");
    } finally {
      loading = false;
    }
  }

  async function serviceAction(action, name) {
    const next = new Set(actionInProgress);
    next.add(`${action}-${name}`);
    actionInProgress = next;
    message = null;

    try {
      const result = await invoke(`${action}_service`, { name });
      message = result.message;
      messageType = result.success ? "success" : "error";
      services = await invoke("list_services");
    } finally {
      const done = new Set(actionInProgress);
      done.delete(`${action}-${name}`);
      actionInProgress = done;
    }
  }

  async function showLogs(name) {
    if (logsService === name) {
      logsService = null;
      return;
    }
    logsService = name;
    logsLoading = true;
    try {
      const result = await invoke("get_service_logs", { name, lines: 50 });
      logsContent = result.lines;
    } finally {
      logsLoading = false;
    }
  }

  function toggleSort(col) {
    if (sortCol === col) {
      sortAsc = !sortAsc;
    } else {
      sortCol = col;
      sortAsc = true;
    }
  }

  function sortIndicator(col) {
    if (sortCol !== col) return "↕";
    return sortAsc ? "↑" : "↓";
  }

  function statusBadgeClass(state) {
    switch (state) {
      case "active": return "badge-active";
      case "inactive": return "badge-inactive";
      case "failed": return "badge-failed";
      case "activating":
      case "deactivating":
        return "badge-transitioning";
      default: return "badge-unknown";
    }
  }

  function enabledBadgeClass(state) {
    switch (state) {
      case "enabled": return "badge-enabled";
      case "disabled": return "badge-disabled";
      case "static": return "badge-static";
      case "masked": return "badge-masked";
      default: return "badge-unknown";
    }
  }

  let filteredServices = $derived.by(() => {
    let list = services;

    // Filter
    if (filter === "active") list = list.filter((s) => s.active_state === "active");
    else if (filter === "inactive") list = list.filter((s) => s.active_state === "inactive");
    else if (filter === "failed") list = list.filter((s) => s.active_state === "failed");

    // Search
    if (search.trim()) {
      const q = search.toLowerCase();
      list = list.filter(
        (s) =>
          s.name.toLowerCase().includes(q) ||
          s.description.toLowerCase().includes(q)
      );
    }

    // Sort
    list = [...list].sort((a, b) => {
      let va = a[sortCol] ?? "";
      let vb = b[sortCol] ?? "";
      if (typeof va === "string") va = va.toLowerCase();
      if (typeof vb === "string") vb = vb.toLowerCase();
      if (va < vb) return sortAsc ? -1 : 1;
      if (va > vb) return sortAsc ? 1 : -1;
      return 0;
    });

    return list;
  });

  let activeSummary = $derived.by(() => {
    const active = services.filter((s) => s.active_state === "active").length;
    const failed = services.filter((s) => s.active_state === "failed").length;
    const inactive = services.filter((s) => s.active_state === "inactive").length;
    return { active, failed, inactive, total: services.length };
  });

  $effect(() => {
    loadServices();
  });
</script>

<div class="service-manager">
  <div class="header">
    <div class="header-left">
      <h2>Service Manager</h2>
      <p class="subtitle">Manage systemd services — start, stop, enable, disable, and view logs</p>
    </div>
    <button class="refresh-btn" onclick={loadServices} disabled={loading}>
      {loading ? "Loading..." : "Refresh"}
    </button>
  </div>

  {#if message}
    <div class="message-banner" class:success={messageType === "success"} class:error={messageType === "error"}>
      <span>{message}</span>
      <button class="dismiss-btn" onclick={() => message = null}>×</button>
    </div>
  {/if}

  {#if loading && services.length === 0}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>Loading services...</p>
    </div>
  {:else}
    <!-- Summary cards -->
    <div class="summary-row">
      <div class="summary-card">
        <span class="summary-value">{activeSummary.total}</span>
        <span class="summary-label">Total</span>
      </div>
      <div class="summary-card active">
        <span class="summary-value">{activeSummary.active}</span>
        <span class="summary-label">Active</span>
      </div>
      <div class="summary-card inactive">
        <span class="summary-value">{activeSummary.inactive}</span>
        <span class="summary-label">Inactive</span>
      </div>
      <div class="summary-card failed">
        <span class="summary-value">{activeSummary.failed}</span>
        <span class="summary-label">Failed</span>
      </div>
    </div>

    <!-- Controls -->
    <div class="controls-row">
      <input
        type="text"
        class="search-input"
        placeholder="Search services..."
        bind:value={search}
      />
      <div class="filter-tabs">
        {#each [["all", "All"], ["active", "Active"], ["inactive", "Inactive"], ["failed", "Failed"]] as [val, label]}
          <button
            class="filter-tab"
            class:active={filter === val}
            onclick={() => filter = val}
          >{label}</button>
        {/each}
      </div>
    </div>

    <!-- Table -->
    <div class="table-wrapper">
      <table class="service-table">
        <thead>
          <tr>
            <th class="col-name" onclick={() => toggleSort("name")}>
              Name <span class="sort-arrow">{sortIndicator("name")}</span>
            </th>
            <th class="col-desc">Description</th>
            <th class="col-status" onclick={() => toggleSort("active_state")}>
              Status <span class="sort-arrow">{sortIndicator("active_state")}</span>
            </th>
            <th class="col-enabled" onclick={() => toggleSort("enabled")}>
              Boot <span class="sort-arrow">{sortIndicator("enabled")}</span>
            </th>
            <th class="col-actions">Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each filteredServices as svc}
            <tr class:failed-row={svc.active_state === "failed"} class="svc-row">
              <td class="col-name">
                <span class="svc-name">{svc.name}</span>
              </td>
              <td class="col-desc">
                <span class="svc-desc">{svc.description || "—"}</span>
              </td>
              <td class="col-status">
                <span class="status-badge {statusBadgeClass(svc.active_state)}">
                  {svc.active_state}
                  {#if svc.sub_state && svc.sub_state !== svc.active_state}
                    ({svc.sub_state})
                  {/if}
                </span>
              </td>
              <td class="col-enabled">
                <span class="enabled-badge {enabledBadgeClass(svc.enabled)}">{svc.enabled}</span>
              </td>
              <td class="col-actions">
                <div class="action-btns">
                  {#if svc.active_state === "active" || svc.active_state === "activating"}
                    <button
                      class="action-btn stop"
                      disabled={actionInProgress.has(`stop_service-${svc.name}`)}
                      onclick={() => serviceAction("stop_service", svc.name)}
                      title="Stop"
                    ><svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><rect x="4" y="4" width="16" height="16" rx="2"/></svg></button>
                    <button
                      class="action-btn restart"
                      disabled={actionInProgress.has(`restart_service-${svc.name}`)}
                      onclick={() => serviceAction("restart_service", svc.name)}
                      title="Restart"
                    ><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M21 2v6h-6"/><path d="M3 12a9 9 0 0 1 15-6.7L21 8"/><path d="M3 22v-6h6"/><path d="M21 12a9 9 0 0 1-15 6.7L3 16"/></svg></button>
                  {:else}
                    <button
                      class="action-btn start"
                      disabled={actionInProgress.has(`start_service-${svc.name}`)}
                      onclick={() => serviceAction("start_service", svc.name)}
                      title="Start"
                    ><svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M6 4l15 8-15 8V4z"/></svg></button>
                  {/if}
                  {#if svc.enabled === "enabled"}
                    <button
                      class="action-btn disable"
                      disabled={actionInProgress.has(`disable_service-${svc.name}`)}
                      onclick={() => serviceAction("disable_service", svc.name)}
                      title="Disable at boot"
                    ><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><circle cx="12" cy="12" r="10"/><line x1="4" y1="4" x2="20" y2="20"/></svg></button>
                  {:else if svc.enabled === "disabled"}
                    <button
                      class="action-btn enable"
                      disabled={actionInProgress.has(`enable_service-${svc.name}`)}
                      onclick={() => serviceAction("enable_service", svc.name)}
                      title="Enable at boot"
                    ><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg></button>
                  {/if}
                  <button
                    class="action-btn logs"
                    class:active={logsService === svc.name}
                    onclick={() => showLogs(svc.name)}
                    title="View logs"
                  ><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="8" y1="13" x2="16" y2="13"/><line x1="8" y1="17" x2="16" y2="17"/></svg></button>
                </div>
              </td>
            </tr>
            {#if logsService === svc.name}
              <tr class="logs-row">
                <td colspan="5">
                  <div class="logs-panel">
                    <div class="logs-header">
                      <span>Logs for <strong>{svc.name}</strong></span>
                      <button class="dismiss-btn" onclick={() => logsService = null}>×</button>
                    </div>
                    {#if logsLoading}
                      <div class="logs-loading">Loading logs...</div>
                    {:else if logsContent.length === 0}
                      <div class="logs-empty">No log entries found.</div>
                    {:else}
                      <pre class="logs-content">{logsContent.join("\n")}</pre>
                    {/if}
                  </div>
                </td>
              </tr>
            {/if}
          {/each}
        </tbody>
      </table>
    </div>

    <div class="table-footer">
      <span class="row-count">Showing {filteredServices.length} of {services.length} services</span>
    </div>
  {/if}
</div>

<style>
  .service-manager {
    padding: 0;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
  }

  .header-left h2 {
    margin: 0;
    font-size: 22px;
    font-weight: 700;
    color: #f1f5f9;
  }

  .subtitle {
    margin: 4px 0 0;
    font-size: 13px;
    color: #94a3b8;
  }

  .refresh-btn {
    padding: 8px 18px;
    background: #ea580c;
    color: white;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 600;
    font-size: 13px;
  }

  .refresh-btn:hover {
    background: #c2410c;
  }

  .refresh-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .message-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    border-radius: 8px;
    margin-bottom: 16px;
    font-size: 13px;
  }

  .message-banner.success {
    background: rgba(34, 197, 94, 0.15);
    border: 1px solid rgba(34, 197, 94, 0.3);
    color: #4ade80;
  }

  .message-banner.error {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: #f87171;
  }

  .dismiss-btn {
    background: none;
    border: none;
    color: inherit;
    font-size: 18px;
    cursor: pointer;
    padding: 0 4px;
    opacity: 0.7;
  }

  .dismiss-btn:hover {
    opacity: 1;
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 60px;
    color: #94a3b8;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid #334155;
    border-top-color: #ea580c;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* Summary */
  .summary-row {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
    margin-bottom: 16px;
  }

  .summary-card {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 10px;
    padding: 16px;
    text-align: center;
  }

  .summary-card.active { border-color: rgba(34, 197, 94, 0.3); }
  .summary-card.inactive { border-color: rgba(148, 163, 184, 0.3); }
  .summary-card.failed { border-color: rgba(239, 68, 68, 0.3); }

  .summary-value {
    display: block;
    font-size: 28px;
    font-weight: 700;
    color: #f1f5f9;
  }

  .summary-card.active .summary-value { color: #4ade80; }
  .summary-card.inactive .summary-value { color: #94a3b8; }
  .summary-card.failed .summary-value { color: #f87171; }

  .summary-label {
    font-size: 12px;
    color: #64748b;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  /* Controls */
  .controls-row {
    display: flex;
    gap: 12px;
    align-items: center;
    margin-bottom: 16px;
    flex-wrap: wrap;
  }

  .search-input {
    flex: 1;
    min-width: 200px;
    padding: 8px 14px;
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 8px;
    color: #e2e8f0;
    font-size: 13px;
    outline: none;
  }

  .search-input:focus {
    border-color: #ea580c;
  }

  .search-input::placeholder {
    color: #475569;
  }

  .filter-tabs {
    display: flex;
    gap: 4px;
    background: #1e293b;
    border-radius: 8px;
    padding: 3px;
    border: 1px solid #334155;
  }

  .filter-tab {
    padding: 6px 14px;
    background: transparent;
    border: none;
    color: #94a3b8;
    font-size: 12px;
    font-weight: 600;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s;
  }

  .filter-tab:hover {
    color: #e2e8f0;
    background: #334155;
  }

  .filter-tab.active {
    background: #ea580c;
    color: white;
  }

  /* Table */
  .table-wrapper {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 10px;
    overflow-x: auto;
  }

  .service-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  .service-table thead th {
    padding: 10px 14px;
    text-align: left;
    color: #94a3b8;
    font-weight: 600;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    border-bottom: 1px solid #334155;
    white-space: nowrap;
    cursor: pointer;
    user-select: none;
  }

  .service-table thead th:hover {
    color: #e2e8f0;
  }

  .col-actions {
    cursor: default !important;
  }

  .sort-arrow {
    color: #475569;
    margin-left: 4px;
  }

  .service-table tbody td {
    padding: 8px 14px;
    border-bottom: 1px solid #1e293b;
    color: #e2e8f0;
    vertical-align: middle;
  }

  .service-table tbody tr {
    background: #1e293b;
    transition: background 0.1s;
  }

  .service-table tbody tr:hover {
    background: #263348;
  }

  .failed-row {
    background: rgba(239, 68, 68, 0.05) !important;
  }

  /* Skip rendering off-screen rows. systemd often returns 200+ services
     and each row contains 4-5 inline SVGs — without this, WebKitGTK
     paints them all on every scroll frame. `contain-intrinsic-size`
     reserves space so the scrollbar doesn't jump. */
  .svc-row {
    content-visibility: auto;
    contain-intrinsic-size: auto 44px;
  }

  .svc-name {
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 12px;
    color: #fb923c;
  }

  .svc-desc {
    color: #94a3b8;
    font-size: 12px;
    max-width: 320px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: inline-block;
  }

  .col-name { min-width: 180px; }
  .col-desc { min-width: 200px; }
  .col-status { min-width: 100px; }
  .col-enabled { min-width: 80px; }
  .col-actions { min-width: 140px; }

  /* Badges */
  .status-badge, .enabled-badge {
    display: inline-block;
    padding: 2px 10px;
    border-radius: 12px;
    font-size: 11px;
    font-weight: 600;
    text-transform: capitalize;
  }

  .badge-active { background: rgba(34, 197, 94, 0.15); color: #4ade80; }
  .badge-inactive { background: rgba(148, 163, 184, 0.15); color: #94a3b8; }
  .badge-failed { background: rgba(239, 68, 68, 0.15); color: #f87171; }
  .badge-transitioning { background: rgba(250, 204, 21, 0.15); color: #fbbf24; }
  .badge-unknown { background: rgba(100, 116, 139, 0.15); color: #64748b; }

  .badge-enabled { background: rgba(34, 197, 94, 0.15); color: #4ade80; }
  .badge-disabled { background: rgba(148, 163, 184, 0.15); color: #94a3b8; }
  .badge-static { background: rgba(96, 165, 250, 0.15); color: #60a5fa; }
  .badge-masked { background: rgba(239, 68, 68, 0.15); color: #f87171; }

  /* Action buttons */
  .action-btns {
    display: flex;
    gap: 4px;
  }

  .action-btn {
    width: 30px;
    height: 30px;
    border: 1px solid #334155;
    border-radius: 8px;
    background: #0f172a;
    cursor: pointer;
    font-size: 13px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s;
    padding: 0;
    color: #94a3b8;
  }

  .action-btn:hover {
    background: #334155;
    border-color: #475569;
  }

  .action-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .action-btn.start {
    color: #4ade80;
  }

  .action-btn.start:hover { border-color: #22c55e; background: rgba(34, 197, 94, 0.15); }
  .action-btn.stop:hover { border-color: #ef4444; }
  .action-btn.restart:hover { border-color: #3b82f6; }
  .action-btn.enable:hover { border-color: #22c55e; }
  .action-btn.disable:hover { border-color: #ef4444; }
  .action-btn.logs:hover, .action-btn.logs.active { border-color: #ea580c; background: rgba(234, 88, 12, 0.15); }

  /* Logs */
  .logs-row td {
    padding: 0 !important;
    border-bottom: 1px solid #334155 !important;
  }

  .logs-panel {
    background: #0f172a;
    border-top: 1px solid #334155;
  }

  .logs-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 14px;
    color: #94a3b8;
    font-size: 12px;
    border-bottom: 1px solid #1e293b;
  }

  .logs-loading, .logs-empty {
    padding: 16px 14px;
    color: #64748b;
    font-size: 12px;
  }

  .logs-content {
    margin: 0;
    padding: 12px 14px;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 11px;
    color: #cbd5e1;
    line-height: 1.6;
    max-height: 300px;
    overflow-y: auto;
    overscroll-behavior: contain;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .table-footer {
    padding: 10px 14px;
    font-size: 12px;
    color: #64748b;
  }
</style>
