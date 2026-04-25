<script>
  import { invoke } from "@tauri-apps/api/core";

  let entries = $state([]);
  let loading = $state(true);
  let error = $state(null);
  let actionMessage = $state(null);
  let togglingIds = $state(new Set());

  // Add new entry form
  let showAddForm = $state(false);
  let newName = $state("");
  let newCommand = $state("");
  let newComment = $state("");
  let adding = $state(false);

  async function loadEntries() {
    loading = true;
    error = null;
    try {
      entries = await invoke("get_startup_entries");
    } catch (e) {
      error = String(e);
    }
    loading = false;
  }

  async function toggleEntry(entry) {
    togglingIds = new Set([...togglingIds, entry.id]);
    actionMessage = null;
    try {
      const msg = await invoke("toggle_startup_entry", {
        id: entry.id,
        enabled: !entry.enabled,
      });
      actionMessage = { type: "success", text: msg };
      await loadEntries();
    } catch (e) {
      actionMessage = { type: "error", text: String(e) };
    }
    const next = new Set(togglingIds);
    next.delete(entry.id);
    togglingIds = next;
  }

  async function removeEntry(entry) {
    actionMessage = null;
    try {
      const msg = await invoke("remove_startup_entry", { id: entry.id });
      actionMessage = { type: "success", text: msg };
      await loadEntries();
    } catch (e) {
      actionMessage = { type: "error", text: String(e) };
    }
  }

  async function addEntry() {
    if (!newName.trim() || !newCommand.trim()) return;
    adding = true;
    actionMessage = null;
    try {
      const msg = await invoke("add_startup_entry", {
        name: newName.trim(),
        command: newCommand.trim(),
        comment: newComment.trim(),
      });
      actionMessage = { type: "success", text: msg };
      newName = "";
      newCommand = "";
      newComment = "";
      showAddForm = false;
      await loadEntries();
    } catch (e) {
      actionMessage = { type: "error", text: String(e) };
    }
    adding = false;
  }

  function getSourceBadge(entry) {
    if (entry.entry_type === "systemd") return { label: "systemd", color: "#6366f1" };
    if (entry.source === "system") return { label: "system", color: "#f59e0b" };
    return { label: "user", color: "#22c55e" };
  }

  const smSvgAttrs = 'viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:1em;height:1em;vertical-align:middle;display:inline-block"';
  const svgGear = `<svg ${smSvgAttrs}><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>`;
  const svgMonitor = `<svg ${smSvgAttrs}><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>`;
  const svgRocket = `<svg ${smSvgAttrs}><path d="M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09z"/><path d="M12 15l-3-3a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.35 22.35 0 0 1-4 2z"/><path d="M9 12H4s.55-3.03 2-4c1.62-1.08 5 0 5 0"/><path d="M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5"/></svg>`;

  function getEntryIcon(entry) {
    if (entry.entry_type === "systemd") return svgGear;
    if (entry.icon) return svgMonitor;
    return svgRocket;
  }

  $effect(() => {
    loadEntries();
  });
</script>

<div class="startup-manager">
  <div class="header">
    <div class="header-left">
      <h2>Startup Manager</h2>
      <span class="entry-count">{entries.length} entries</span>
    </div>
    <div class="header-actions">
      <button class="refresh-btn" onclick={loadEntries} disabled={loading}>
        <span class:spin={loading}>↻</span>
      </button>
      <button
        class="add-btn"
        onclick={() => (showAddForm = !showAddForm)}
      >
        {showAddForm ? "✕ Cancel" : "+ Add Entry"}
      </button>
    </div>
  </div>

  {#if actionMessage}
    <div class="action-message {actionMessage.type}">
      {actionMessage.text}
      <button class="dismiss-btn" onclick={() => (actionMessage = null)}>✕</button>
    </div>
  {/if}

  {#if showAddForm}
    <div class="add-form">
      <h3>Add Startup Entry</h3>
      <div class="form-grid">
        <label>
          <span>Name</span>
          <input
            type="text"
            bind:value={newName}
            placeholder="My Application"
          />
        </label>
        <label>
          <span>Command</span>
          <input
            type="text"
            bind:value={newCommand}
            placeholder="/usr/bin/myapp --start-minimized"
          />
        </label>
        <label>
          <span>Comment <span class="optional">(optional)</span></span>
          <input
            type="text"
            bind:value={newComment}
            placeholder="Starts my application on login"
          />
        </label>
      </div>
      <button
        class="submit-btn"
        onclick={addEntry}
        disabled={adding || !newName.trim() || !newCommand.trim()}
      >
        {adding ? "Adding..." : "Add to Startup"}
      </button>
    </div>
  {/if}

  {#if loading && entries.length === 0}
    <div class="loading">
      <div class="spinner"></div>
      <p>Scanning startup entries...</p>
    </div>
  {:else if error}
    <div class="error-box">
      <strong>Error:</strong> {error}
    </div>
  {:else if entries.length === 0}
    <div class="empty">
      <p>No startup entries found.</p>
      <p class="hint">Click "+ Add Entry" to add an application to startup.</p>
    </div>
  {:else}
    <div class="entries-list">
      {#each entries as entry (entry.id)}
        {@const badge = getSourceBadge(entry)}
        <div class="entry-card" class:disabled={!entry.enabled}>
          <div class="entry-icon">{@html getEntryIcon(entry)}</div>
          <div class="entry-info">
            <div class="entry-header">
              <span class="entry-name">{entry.name}</span>
              <span class="badge" style="background: {badge.color}20; color: {badge.color}; border: 1px solid {badge.color}40">
                {badge.label}
              </span>
            </div>
            {#if entry.comment}
              <div class="entry-comment">{entry.comment}</div>
            {/if}
            <div class="entry-command" title={entry.command}>
              {entry.command}
            </div>
          </div>
          <div class="entry-actions">
            <!-- svelte-ignore a11y_consider_explicit_label -->
            <button
              class="toggle-btn"
              class:active={entry.enabled}
              onclick={() => toggleEntry(entry)}
              disabled={togglingIds.has(entry.id)}
              title={entry.enabled ? "Disable" : "Enable"}
            >
              <span class="toggle-track">
                <span class="toggle-thumb"></span>
              </span>
            </button>
            {#if entry.source === "user" && entry.entry_type === "desktop"}
              <button
                class="remove-btn"
                onclick={() => removeEntry(entry)}
                title="Remove entry"
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:1em;height:1em;vertical-align:middle;display:inline-block"><path d="M3 6h18M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2"/><path d="M5 6v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V6"/><line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/></svg>
              </button>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .startup-manager {
    padding: 0;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
  }

  .header-left {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }

  .header h2 {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 700;
    color: #e2e8f0;
  }

  .entry-count {
    font-size: 0.85rem;
    color: #64748b;
    background: #1e293b;
    padding: 2px 10px;
    border-radius: 12px;
  }

  .header-actions {
    display: flex;
    gap: 8px;
  }

  .refresh-btn {
    background: #1e293b;
    border: 1px solid #334155;
    color: #94a3b8;
    width: 36px;
    height: 36px;
    border-radius: 8px;
    cursor: pointer;
    font-size: 1.1rem;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s;
  }

  .refresh-btn:hover {
    background: #334155;
    color: #e2e8f0;
  }

  .spin {
    display: inline-block;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .add-btn {
    background: #7c3aed;
    border: none;
    color: #fff;
    padding: 8px 16px;
    border-radius: 8px;
    cursor: pointer;
    font-size: 0.85rem;
    font-weight: 600;
    transition: all 0.15s;
  }

  .add-btn:hover {
    background: #6d28d9;
  }

  .action-message {
    padding: 10px 16px;
    border-radius: 8px;
    margin-bottom: 16px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.9rem;
  }

  .action-message.success {
    background: #22c55e18;
    border: 1px solid #22c55e40;
    color: #4ade80;
  }

  .action-message.error {
    background: #ef444418;
    border: 1px solid #ef444440;
    color: #f87171;
  }

  .dismiss-btn {
    background: none;
    border: none;
    color: inherit;
    cursor: pointer;
    opacity: 0.6;
    font-size: 0.9rem;
    padding: 0 4px;
  }

  .dismiss-btn:hover {
    opacity: 1;
  }

  .add-form {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 12px;
    padding: 20px;
    margin-bottom: 20px;
  }

  .add-form h3 {
    margin: 0 0 16px;
    font-size: 1.05rem;
    color: #e2e8f0;
  }

  .form-grid {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 16px;
  }

  .form-grid label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .form-grid label span {
    font-size: 0.8rem;
    color: #94a3b8;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .optional {
    text-transform: none !important;
    font-weight: 400 !important;
    opacity: 0.6;
  }

  .form-grid input {
    background: #0f172a;
    border: 1px solid #334155;
    color: #e2e8f0;
    padding: 10px 12px;
    border-radius: 8px;
    font-size: 0.9rem;
    font-family: inherit;
    outline: none;
    transition: border-color 0.15s;
  }

  .form-grid input:focus {
    border-color: #7c3aed;
  }

  .form-grid input::placeholder {
    color: #475569;
  }

  .submit-btn {
    background: #7c3aed;
    border: none;
    color: #fff;
    padding: 10px 24px;
    border-radius: 8px;
    cursor: pointer;
    font-size: 0.9rem;
    font-weight: 600;
    transition: all 0.15s;
  }

  .submit-btn:hover:not(:disabled) {
    background: #6d28d9;
  }

  .submit-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
    padding: 60px 0;
    color: #94a3b8;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid #334155;
    border-top-color: #7c3aed;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .error-box {
    background: #ef444418;
    border: 1px solid #ef444440;
    border-radius: 8px;
    padding: 16px;
    color: #f87171;
    font-size: 0.9rem;
  }

  .empty {
    text-align: center;
    padding: 60px 0;
    color: #64748b;
  }

  .empty .hint {
    font-size: 0.85rem;
    margin-top: 8px;
    opacity: 0.7;
  }

  .entries-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .entry-card {
    display: flex;
    align-items: center;
    gap: 14px;
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 10px;
    padding: 14px 16px;
    transition: all 0.15s;
    content-visibility: auto;
    contain-intrinsic-size: auto 80px;
  }

  .entry-card:hover {
    border-color: #475569;
    background: #1e293bee;
  }

  .entry-card.disabled {
    opacity: 0.55;
  }

  .entry-card.disabled:hover {
    opacity: 0.7;
  }

  .entry-icon {
    font-size: 1.4rem;
    flex-shrink: 0;
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #0f172a;
    border-radius: 8px;
  }

  .entry-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .entry-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .entry-name {
    font-weight: 600;
    color: #e2e8f0;
    font-size: 0.95rem;
  }

  .badge {
    font-size: 0.65rem;
    padding: 1px 8px;
    border-radius: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    flex-shrink: 0;
  }

  .entry-comment {
    color: #94a3b8;
    font-size: 0.8rem;
    line-height: 1.4;
  }

  .entry-command {
    color: #64748b;
    font-size: 0.75rem;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .entry-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .toggle-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: 4px;
    transition: opacity 0.15s;
  }

  .toggle-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .toggle-track {
    display: block;
    width: 40px;
    height: 22px;
    background: #334155;
    border-radius: 11px;
    position: relative;
    transition: background 0.2s;
  }

  .toggle-btn.active .toggle-track {
    background: #7c3aed;
  }

  .toggle-thumb {
    display: block;
    width: 16px;
    height: 16px;
    background: #e2e8f0;
    border-radius: 50%;
    position: absolute;
    top: 3px;
    left: 3px;
    transition: transform 0.2s;
  }

  .toggle-btn.active .toggle-thumb {
    transform: translateX(18px);
  }

  .remove-btn {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 1rem;
    padding: 4px;
    opacity: 0.4;
    transition: opacity 0.15s;
  }

  .remove-btn:hover {
    opacity: 1;
  }
</style>
