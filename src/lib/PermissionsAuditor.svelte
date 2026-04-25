<script>
  import { invoke } from "@tauri-apps/api/core";

  let report = $state(null);
  let scanning = $state(false);
  let activeTab = $state("suid");
  let search = $state("");
  let showHelp = $state(false);

  const tabs = [
    { id: "suid", label: "SUID Binaries", icon: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:1em;height:1em;vertical-align:middle;display:inline-block"><path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.78 7.78 5.5 5.5 0 0 1 7.78-7.78zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"/></svg>`, desc: "Files that run with the owner's privileges (usually root). Expected for sudo/passwd, but unexpected SUID binaries can be a security risk." },
    { id: "sgid", label: "SGID Binaries", icon: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:1em;height:1em;vertical-align:middle;display:inline-block"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/><circle cx="12" cy="16" r="1"/></svg>`, desc: "Files that run with the group's privileges. Less dangerous than SUID but still worth reviewing." },
    { id: "world", label: "World-Writable", icon: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:1em;height:1em;vertical-align:middle;display:inline-block"><circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg>`, desc: "Files that any user on the system can modify. Could allow tampering with configs or scripts." },
    { id: "home", label: "Home Directories", icon: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:1em;height:1em;vertical-align:middle;display:inline-block"><path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg>`, desc: "Checks home folder permissions, SSH key security, and authorized_keys exposure." },
  ];

  async function runScan() {
    scanning = true;
    try {
      report = await invoke("run_permission_audit", { scanPaths: [] });
    } finally {
      scanning = false;
    }
  }

  function severityIcon(severity) {
    switch (severity) {
      case "critical": return '<span class="sev-dot" style="background:#ef4444"></span>';
      case "warning": return '<span class="sev-dot" style="background:#f59e0b"></span>';
      case "info": return '<span class="sev-dot" style="background:#4ade80"></span>';
      default: return '<span class="sev-dot" style="background:#6b7280"></span>';
    }
  }

  function severityClass(severity) {
    switch (severity) {
      case "critical": return "sev-critical";
      case "warning": return "sev-warning";
      case "info": return "sev-info";
      default: return "";
    }
  }

  let currentItems = $derived.by(() => {
    if (!report) return [];
    let items;
    switch (activeTab) {
      case "suid": items = report.suid_binaries; break;
      case "sgid": items = report.sgid_binaries; break;
      case "world": items = report.world_writable; break;
      case "home": items = report.home_dir_issues; break;
      default: items = [];
    }
    if (search.trim()) {
      const q = search.toLowerCase();
      items = items.filter(
        (e) =>
          e.path.toLowerCase().includes(q) ||
          e.description.toLowerCase().includes(q) ||
          e.owner.toLowerCase().includes(q)
      );
    }
    return items;
  });

  function tabCount(tabId) {
    if (!report) return 0;
    switch (tabId) {
      case "suid": return report.suid_binaries.length;
      case "sgid": return report.sgid_binaries.length;
      case "world": return report.world_writable.length;
      case "home": return report.home_dir_issues.length;
      default: return 0;
    }
  }

  function formatSize(bytes) {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
  }

  $effect(() => {
    runScan();
  });
</script>

<div class="permissions-auditor">
  <div class="header">
    <div class="header-left">
      <h2>File Permissions Auditor</h2>
      <p class="subtitle">Scan for SUID/SGID binaries, world-writable files, and misconfigured home directories</p>
    </div>
    <button class="scan-btn" onclick={runScan} disabled={scanning}>
      {scanning ? "Scanning..." : "Rescan"}
    </button>
  </div>

  <div class="info-banner">
    <span class="info-icon"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:1em;height:1em;vertical-align:middle;display:inline-block"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg></span>
    <span>Audits your filesystem for potential security risks. <strong>SUID/SGID binaries</strong> run with elevated privileges and can be exploited if unexpected. <strong>World-writable files</strong> can be modified by any user. <strong>Home directory</strong> checks verify that SSH keys and personal folders aren't exposed to other users.</span>
    <button class="help-toggle" onclick={() => showHelp = !showHelp}>{showHelp ? 'Hide details ▲' : 'How it works ▼'}</button>
  </div>

  {#if showHelp}
    <div class="help-panel">
      <div class="help-section">
        <h4><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:1em;height:1em;vertical-align:middle;display:inline-block"><path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.78 7.78 5.5 5.5 0 0 1 7.78-7.78zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"/></svg> SUID Binaries</h4>
        <p>Finds files with the <code>-4000</code> permission bit set. These files execute with the <em>owner's</em> privileges (usually root) regardless of who runs them. Known system binaries (<code>sudo</code>, <code>su</code>, <code>passwd</code>, <code>mount</code>, <code>pkexec</code>, etc.) are flagged as <span class="sev-info-inline">info</span>, while unexpected SUID binaries are flagged <span class="sev-critical-inline">critical</span> since they could allow privilege escalation.</p>
      </div>
      <div class="help-section">
        <h4><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:1em;height:1em;vertical-align:middle;display:inline-block"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/><circle cx="12" cy="16" r="1"/></svg> SGID Binaries</h4>
        <p>Finds files with the <code>-2000</code> permission bit set. These files execute with the <em>group's</em> privileges. All SGID binaries are flagged as <span class="sev-warning-inline">warning</span> for review.</p>
      </div>
      <div class="help-section">
        <h4><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:1em;height:1em;vertical-align:middle;display:inline-block"><circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg> World-Writable Files</h4>
        <p>Finds files with the <code>-0002</code> permission bit (anyone can write). Flagged as <span class="sev-warning-inline">warning</span> because they can be tampered with by any user on the system.</p>
      </div>
      <div class="help-section">
        <h4><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:1em;height:1em;vertical-align:middle;display:inline-block"><path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg> Home Directory Checks</h4>
        <p>Inspects each <code>/home/*</code> directory for:</p>
        <ul>
          <li>World-readable (<code>o+r</code>) home folders → <span class="sev-warning-inline">warning</span></li>
          <li>World-writable (<code>o+w</code>) home folders → <span class="sev-critical-inline">critical</span></li>
          <li><code>.ssh/</code> directory should be <code>700</code> — any deviation is <span class="sev-critical-inline">critical</span></li>
          <li><code>authorized_keys</code> should be <code>600</code> — any deviation is <span class="sev-critical-inline">critical</span></li>
          <li>Private keys (<code>id_*</code>) should be <code>600</code> — any deviation is <span class="sev-critical-inline">critical</span></li>
        </ul>
      </div>
    </div>
  {/if}

  {#if scanning && !report}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>Scanning file permissions...</p>
      <p class="scan-hint">This may take a moment</p>
    </div>
  {:else if report}
    <!-- Summary -->
    <div class="summary-row">
      <div class="summary-card" class:highlight={report.summary.suid_count > 0}>
        <span class="summary-icon"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:1em;height:1em;vertical-align:middle;display:inline-block"><path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.78 7.78 5.5 5.5 0 0 1 7.78-7.78zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"/></svg></span>
        <span class="summary-value">{report.summary.suid_count}</span>
        <span class="summary-label">SUID Binaries</span>
      </div>
      <div class="summary-card" class:highlight={report.summary.sgid_count > 0}>
        <span class="summary-icon"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:1em;height:1em;vertical-align:middle;display:inline-block"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/><circle cx="12" cy="16" r="1"/></svg></span>
        <span class="summary-value">{report.summary.sgid_count}</span>
        <span class="summary-label">SGID Binaries</span>
      </div>
      <div class="summary-card" class:warn={report.summary.world_writable_count > 0}>
        <span class="summary-icon"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:1em;height:1em;vertical-align:middle;display:inline-block"><circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg></span>
        <span class="summary-value">{report.summary.world_writable_count}</span>
        <span class="summary-label">World-Writable</span>
      </div>
      <div class="summary-card" class:warn={report.summary.home_issues_count > 0}>
        <span class="summary-icon"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:1em;height:1em;vertical-align:middle;display:inline-block"><path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg></span>
        <span class="summary-value">{report.summary.home_issues_count}</span>
        <span class="summary-label">Home Dir Issues</span>
      </div>
    </div>

    <!-- Tabs -->
    <div class="controls-row">
      <div class="tab-group">
        {#each tabs as tab}
          <button
            class="tab-btn"
            class:active={activeTab === tab.id}
            onclick={() => activeTab = tab.id}
          >
            <span>{@html tab.icon}</span>
            <span>{tab.label}</span>
            <span class="tab-count">{tabCount(tab.id)}</span>
          </button>
        {/each}
      </div>
      <input
        type="text"
        class="search-input"
        placeholder="Filter results..."
        bind:value={search}
      />
    </div>

    <div class="tab-desc">
      {tabs.find(t => t.id === activeTab)?.desc}
    </div>

    <!-- Results -->
    {#if currentItems.length === 0}
      <div class="empty-state">
        <p>
          {#if !search.trim()}
            No issues found in this category.
          {:else}
            No results matching "{search}".
          {/if}
        </p>
      </div>
    {:else}
      <div class="results-list">
        {#each currentItems as entry}
          <div class="result-card {severityClass(entry.severity)}">
            <div class="result-header">
              <span class="sev-icon">{@html severityIcon(entry.severity)}</span>
              <span class="result-path">{entry.path}</span>
              <span class="result-perms">{entry.permissions}</span>
            </div>
            <div class="result-details">
              <span class="result-desc">{entry.description}</span>
              <div class="result-meta">
                <span>Owner: <strong>{entry.owner}</strong></span>
                <span>Group: <strong>{entry.group}</strong></span>
                {#if entry.size_bytes > 0}
                  <span>Size: <strong>{formatSize(entry.size_bytes)}</strong></span>
                {/if}
              </div>
            </div>
          </div>
        {/each}
      </div>
      <div class="results-footer">
        <span>Showing {currentItems.length} items</span>
      </div>
    {/if}

    <div class="scan-info">
      <span>Scanned paths: {report.summary.scan_paths.join(", ")}</span>
    </div>
  {:else}
    <div class="empty-state">
      <p>Click Rescan to audit file permissions.</p>
    </div>
  {/if}
</div>


<style>
  .permissions-auditor {
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

  .info-banner {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 12px 16px;
    background: rgba(217, 119, 6, 0.08);
    border: 1px solid rgba(217, 119, 6, 0.2);
    border-radius: 8px;
    margin-bottom: 16px;
    font-size: 13px;
    color: #94a3b8;
    line-height: 1.5;
    flex-wrap: wrap;
  }

  .help-toggle {
    background: none;
    border: none;
    color: #d97706;
    font-size: 12px;
    cursor: pointer;
    padding: 2px 0;
    margin-left: auto;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .help-toggle:hover {
    color: #fbbf24;
  }

  .help-panel {
    background: rgba(15, 23, 42, 0.5);
    border: 1px solid rgba(217, 119, 6, 0.15);
    border-radius: 8px;
    padding: 16px 20px;
    margin-bottom: 16px;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  .help-section h4 {
    margin: 0 0 6px 0;
    font-size: 13px;
    color: #e2e8f0;
  }

  .help-section p,
  .help-section ul {
    margin: 0;
    font-size: 12px;
    color: #94a3b8;
    line-height: 1.6;
  }

  .help-section ul {
    padding-left: 18px;
  }

  .help-section code {
    background: rgba(0, 0, 0, 0.3);
    padding: 1px 5px;
    border-radius: 3px;
    font-size: 11px;
    color: #cbd5e1;
  }

  .sev-info-inline {
    color: #4ade80;
    font-weight: 600;
  }
  .sev-warning-inline {
    color: #fbbf24;
    font-weight: 600;
  }
  .sev-critical-inline {
    color: #f87171;
    font-weight: 600;
  }

  .info-icon {
    font-size: 18px;
    line-height: 1;
    flex-shrink: 0;
    margin-top: 1px;
  }

  .scan-btn {
    padding: 8px 18px;
    background: #d97706;
    color: white;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 600;
    font-size: 13px;
  }

  .scan-btn:hover {
    background: #b45309;
  }

  .scan-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 60px;
    color: #94a3b8;
  }

  .scan-hint {
    font-size: 12px;
    color: #64748b;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid #334155;
    border-top-color: #d97706;
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
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  .summary-card.highlight {
    border-color: rgba(217, 119, 6, 0.4);
  }

  .summary-card.warn {
    border-color: rgba(239, 68, 68, 0.4);
  }

  .summary-icon {
    font-size: 20px;
  }

  .summary-value {
    font-size: 28px;
    font-weight: 700;
    color: #f1f5f9;
  }

  .summary-card.warn .summary-value { color: #f87171; }
  .summary-card.highlight .summary-value { color: #fbbf24; }

  .summary-label {
    font-size: 11px;
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

  .tab-group {
    display: flex;
    gap: 4px;
    background: #1e293b;
    border-radius: 8px;
    padding: 3px;
    border: 1px solid #334155;
    flex-wrap: wrap;
  }

  .tab-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: transparent;
    border: none;
    color: #94a3b8;
    font-size: 12px;
    font-weight: 600;
    border-radius: 6px;
    cursor: pointer;
    transition: background-color 0.15s, border-color 0.15s, color 0.15s, opacity 0.15s;
    white-space: nowrap;
  }

  .tab-btn:hover {
    color: #e2e8f0;
    background: #334155;
  }

  .tab-btn.active {
    background: #d97706;
    color: white;
  }

  .tab-count {
    background: rgba(0, 0, 0, 0.2);
    padding: 1px 6px;
    border-radius: 10px;
    font-size: 10px;
  }

  .tab-btn.active .tab-count {
    background: rgba(255, 255, 255, 0.2);
  }

  .search-input {
    flex: 1;
    min-width: 180px;
    padding: 8px 14px;
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 8px;
    color: #e2e8f0;
    font-size: 13px;
    outline: none;
  }

  .search-input:focus {
    border-color: #d97706;
  }

  .search-input::placeholder {
    color: #475569;
  }

  .tab-desc {
    font-size: 12px;
    color: #64748b;
    padding: 8px 14px;
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 8px;
    margin-bottom: 16px;
    line-height: 1.5;
  }

  /* Results */
  .results-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 600px;
    overflow-y: auto;
  }

  .result-card {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 8px;
    padding: 10px 14px;
    transition: background 0.1s;
  }

  .result-card:hover {
    background: #263348;
  }

  .result-card.sev-critical {
    border-left: 3px solid #ef4444;
  }

  .result-card.sev-warning {
    border-left: 3px solid #f59e0b;
  }

  .result-card.sev-info {
    border-left: 3px solid #22c55e;
  }

  .result-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }

  .sev-icon {
    font-size: 12px;
  }

  .sev-dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    vertical-align: middle;
  }

  .result-path {
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 12px;
    color: #fbbf24;
    flex: 1;
    word-break: break-all;
  }

  .result-perms {
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 11px;
    color: #94a3b8;
    background: #0f172a;
    padding: 2px 8px;
    border-radius: 4px;
    white-space: nowrap;
  }

  .result-details {
    padding-left: 24px;
  }

  .result-desc {
    font-size: 12px;
    color: #94a3b8;
  }

  .result-meta {
    display: flex;
    gap: 16px;
    margin-top: 4px;
    font-size: 11px;
    color: #64748b;
  }

  .result-meta strong {
    color: #94a3b8;
  }

  .results-footer {
    padding: 10px 0;
    font-size: 12px;
    color: #64748b;
  }

  .empty-state {
    text-align: center;
    padding: 40px;
    color: #64748b;
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 10px;
  }

  .scan-info {
    margin-top: 12px;
    font-size: 11px;
    color: #475569;
    padding: 8px 12px;
    background: #0f172a;
    border-radius: 6px;
  }
</style>
