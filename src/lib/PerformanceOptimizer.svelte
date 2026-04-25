<script>
  import { invoke } from "@tauri-apps/api/core";

  let tweaks = $state([]);
  let loading = $state(true);
  let applying = $state(new Set());
  let restoring = $state(new Set());
  let results = $state({});

  const svgAttrs = 'width="1em" height="1em" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"';
  const categoryIcons = {
    CPU: `<svg ${svgAttrs}><rect x="4" y="4" width="16" height="16" rx="2"/><rect x="9" y="9" width="6" height="6"/><path d="M9 1v3M15 1v3M9 20v3M15 20v3M20 9h3M20 14h3M1 9h3M1 14h3"/></svg>`,
    Memory: `<svg ${svgAttrs}><rect x="3" y="5" width="18" height="14" rx="2"/><path d="M7 9v6M11 9v6M15 9v6M19 9v6"/></svg>`,
    Storage: `<svg ${svgAttrs}><ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/></svg>`,
    Network: `<svg ${svgAttrs}><circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg>`,
    Kernel: `<svg ${svgAttrs}><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>`,
  };
  const fallbackIcon = `<svg ${svgAttrs}><path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/></svg>`;

  async function loadTweaks() {
    loading = true;
    tweaks = await invoke("get_tweaks");
    loading = false;
  }

  async function applyTweak(id) {
    const next = new Set(applying);
    next.add(id);
    applying = next;

    try {
      const result = await invoke("apply_tweak", { id });
      results = { ...results, [id]: result };
      // Refresh to pick up new current values
      tweaks = await invoke("get_tweaks");
    } finally {
      const done = new Set(applying);
      done.delete(id);
      applying = done;
    }
  }

  async function applyAll() {
    const pending = tweaks.filter((t) => !t.is_applied);
    for (const t of pending) {
      await applyTweak(t.id);
    }
  }

  async function restoreTweak(id) {
    const next = new Set(restoring);
    next.add(id);
    restoring = next;

    try {
      const result = await invoke("restore_tweak", { id });
      results = { ...results, [id]: result };
      tweaks = await invoke("get_tweaks");
    } finally {
      const done = new Set(restoring);
      done.delete(id);
      restoring = done;
    }
  }

  async function restoreAll() {
    const applied = tweaks.filter((t) => t.is_applied);
    for (const t of applied) {
      await restoreTweak(t.id);
    }
  }

  $effect(() => {
    loadTweaks();
  });

  let appliedCount = $derived(tweaks.filter((t) => t.is_applied).length);
  let totalCount = $derived(tweaks.length);
  let categories = $derived([...new Set(tweaks.map((t) => t.category))]);
</script>

<div class="optimizer">
  <div class="header">
    <div class="header-left">
      <h2>Performance Optimizer</h2>
      {#if tweaks.length > 0}
        <span class="score-badge" class:all-good={appliedCount === totalCount}>
          {appliedCount}/{totalCount} applied
        </span>
      {/if}
    </div>
    <div class="header-actions">
      <button class="btn btn-secondary" onclick={loadTweaks} disabled={loading}>
        <span class:spin={loading}>↻</span> Refresh
      </button>
      {#if appliedCount > 0}
        <button class="btn btn-restore" onclick={restoreAll}>
          ↩ Restore All
        </button>
      {/if}
      {#if appliedCount < totalCount}
        <button class="btn btn-primary" onclick={applyAll}>
          <svg width="1em" height="1em" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:middle"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/></svg> Apply All
        </button>
      {/if}
    </div>
  </div>

  {#if loading && tweaks.length === 0}
    <div class="loading">
      <div class="spinner"></div>
      <p>Scanning system settings...</p>
    </div>
  {:else if tweaks.length === 0}
    <div class="empty-state">
      <p>No performance tweaks available on this system.</p>
    </div>
  {:else}
    <div class="info-banner">
      <span class="info-icon"><svg width="1em" height="1em" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg></span>
      <span>These tweaks optimize your system for gaming and performance. All changes are permanent and persist across reboots. Root access is required.</span>
    </div>

    <div class="tweaks-body">
      {#each categories as category}
        <section class="tweak-section">
          <h3 class="section-title">
            <span class="section-icon">{@html categoryIcons[category] || fallbackIcon}</span>
            {category}
          </h3>

          <div class="tweak-list">
            {#each tweaks.filter((t) => t.category === category) as tweak}
              <div class="tweak-card" class:applied={tweak.is_applied}>
                <div class="tweak-info">
                  <div class="tweak-header">
                    <span class="tweak-name">{tweak.name}</span>
                    {#if tweak.is_applied}
                      <span class="status-badge applied">Applied</span>
                    {:else}
                      <span class="status-badge pending">Not optimized</span>
                    {/if}
                  </div>
                  <p class="tweak-desc">{tweak.description}</p>
                  <div class="sys-info">
                    <span class="sys-label">File</span>
                    <span class="sys-path">{tweak.sys_path}</span>
                    <span class="sys-label">Value</span>
                    <span class="sys-path">{tweak.is_applied ? tweak.current_value : tweak.recommended_value}</span>
                  </div>
                  <div class="tweak-values">
                    <span class="value-item">
                      <span class="value-label">Current:</span>
                      <span class="value-data" class:good={tweak.is_applied} class:warn={!tweak.is_applied}>{tweak.current_value}</span>
                    </span>
                    {#if !tweak.is_applied}
                      <span class="value-item">
                        <span class="value-label">Recommended:</span>
                        <span class="value-data good">{tweak.recommended_value}</span>
                      </span>
                    {/if}
                    {#if tweak.is_applied && tweak.default_value !== tweak.current_value}
                      <span class="value-item">
                        <span class="value-label">Default:</span>
                        <span class="value-data default">{tweak.default_value}</span>
                      </span>
                    {/if}
                  </div>
                </div>
                <div class="tweak-action">
                  {#if tweak.is_applied}
                    <button
                      class="btn btn-restore-small"
                      onclick={() => restoreTweak(tweak.id)}
                      disabled={restoring.has(tweak.id)}
                    >
                      {#if restoring.has(tweak.id)}
                        <span class="spin">↻</span>
                      {:else}
                        Restore
                      {/if}
                    </button>
                  {:else}
                    <button
                      class="btn btn-apply"
                      onclick={() => applyTweak(tweak.id)}
                      disabled={applying.has(tweak.id)}
                    >
                      {#if applying.has(tweak.id)}
                        <span class="spin">↻</span>
                      {:else}
                        Apply
                      {/if}
                    </button>
                  {/if}
                </div>
              </div>

              {#if results[tweak.id] && !results[tweak.id].success}
                <div class="result-error">
                  <span>✗</span> {results[tweak.id].message}
                </div>
              {/if}
            {/each}
          </div>
        </section>
      {/each}
    </div>
  {/if}
</div>

<style>
  .optimizer {
    padding: 0;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
    flex-wrap: wrap;
    gap: 12px;
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

  .header-actions {
    display: flex;
    gap: 8px;
  }

  .score-badge {
    font-size: 0.85rem;
    font-weight: 600;
    color: #f59e0b;
    background: #f59e0b18;
    padding: 3px 12px;
    border-radius: 12px;
  }

  .score-badge.all-good {
    color: #22c55e;
    background: #22c55e18;
  }

  .btn {
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    border: none;
    transition: background-color 0.15s, border-color 0.15s, color 0.15s, opacity 0.15s;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .btn-secondary {
    background: #1e293b;
    border: 1px solid #334155;
    color: #94a3b8;
  }

  .btn-secondary:hover {
    background: #334155;
    color: #e2e8f0;
  }

  .btn-primary {
    background: #7c3aed;
    color: #fff;
  }

  .btn-primary:hover {
    background: #6d28d9;
  }

  .btn-apply {
    background: #1e293b;
    border: 1px solid #7c3aed;
    color: #c4b5fd;
    padding: 6px 16px;
    border-radius: 8px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: background-color 0.15s, border-color 0.15s, color 0.15s, opacity 0.15s;
  }

  .btn-apply:hover {
    background: #7c3aed;
    color: #fff;
  }

  .btn-apply:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-restore {
    background: #1e293b;
    border: 1px solid #f59e0b;
    color: #fbbf24;
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: background-color 0.15s, border-color 0.15s, color 0.15s, opacity 0.15s;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .btn-restore:hover {
    background: #f59e0b;
    color: #0f172a;
  }

  .btn-restore-small {
    background: #1e293b;
    border: 1px solid #f59e0b;
    color: #fbbf24;
    padding: 6px 16px;
    border-radius: 8px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: background-color 0.15s, border-color 0.15s, color 0.15s, opacity 0.15s;
  }

  .btn-restore-small:hover {
    background: #f59e0b;
    color: #0f172a;
  }

  .btn-restore-small:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .spin {
    display: inline-block;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
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

  .empty-state {
    text-align: center;
    padding: 60px 0;
    color: #64748b;
    font-size: 0.95rem;
  }

  .info-banner {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 10px;
    padding: 12px 16px;
    margin-bottom: 20px;
    font-size: 0.88rem;
    color: #94a3b8;
    display: flex;
    gap: 10px;
    align-items: flex-start;
    line-height: 1.5;
  }

  .info-icon {
    font-size: 1rem;
    flex-shrink: 0;
    margin-top: 1px;
  }

  .tweaks-body {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .tweak-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .section-title {
    margin: 0;
    font-size: 0.9rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #cbd5e1;
    font-weight: 700;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .section-icon {
    font-size: 1rem;
  }

  .tweak-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .tweak-card {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 12px;
    padding: 16px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    transition: border-color 0.15s;
  }

  .tweak-card.applied {
    border-color: #22c55e30;
  }

  .tweak-info {
    flex: 1;
    min-width: 0;
  }

  .tweak-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 6px;
    flex-wrap: wrap;
  }

  .tweak-name {
    font-size: 1rem;
    font-weight: 600;
    color: #f1f5f9;
  }

  .status-badge {
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: uppercase;
    padding: 2px 8px;
    border-radius: 6px;
    letter-spacing: 0.5px;
  }

  .status-badge.applied {
    background: #22c55e18;
    color: #4ade80;
  }

  .status-badge.pending {
    background: #f59e0b18;
    color: #fbbf24;
  }

  .tweak-desc {
    font-size: 0.88rem;
    color: #94a3b8;
    margin: 0 0 8px 0;
    line-height: 1.4;
  }

  .sys-info {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }

  .sys-label {
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: uppercase;
    color: #7c3aed;
    letter-spacing: 0.5px;
    flex-shrink: 0;
  }

  .sys-path {
    font-size: 0.8rem;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    color: #c4b5fd;
    background: #7c3aed12;
    padding: 2px 8px;
    border-radius: 4px;
    word-break: break-all;
  }

  .tweak-values {
    display: flex;
    gap: 20px;
    flex-wrap: wrap;
  }

  .value-item {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .value-label {
    font-size: 0.8rem;
    color: #64748b;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .value-data {
    font-size: 0.9rem;
    font-weight: 600;
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  .value-data.good {
    color: #4ade80;
  }

  .value-data.warn {
    color: #fbbf24;
  }

  .value-data.default {
    color: #94a3b8;
  }

  .tweak-action {
    flex-shrink: 0;
  }

  .result-error {
    background: #ef444418;
    border: 1px solid #ef444440;
    border-radius: 8px;
    padding: 10px 14px;
    color: #f87171;
    font-size: 0.85rem;
    margin-top: -4px;
  }
</style>
