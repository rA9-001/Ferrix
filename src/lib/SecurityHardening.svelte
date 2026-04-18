<script>
  import { invoke } from "@tauri-apps/api/core";

  let items = $state([]);
  let loading = $state(true);
  let applying = $state(new Set());
  let restoring = $state(new Set());
  let results = $state({});

  const svgAttrs = 'width="1em" height="1em" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"';
  const categoryIcons = {
    Network: `<svg ${svgAttrs}><circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg>`,
    Kernel: `<svg ${svgAttrs}><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>`,
    Filesystem: `<svg ${svgAttrs}><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>`,
  };
  const fallbackIcon = `<svg ${svgAttrs}><path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/></svg>`;

  async function loadStatus() {
    loading = true;
    items = await invoke("get_hardening_status");
    loading = false;
  }

  async function applyItem(id) {
    const next = new Set(applying);
    next.add(id);
    applying = next;

    try {
      const result = await invoke("apply_hardening", { id });
      results = { ...results, [id]: result };
      items = await invoke("get_hardening_status");
    } finally {
      const done = new Set(applying);
      done.delete(id);
      applying = done;
    }
  }

  async function restoreItem(id) {
    const next = new Set(restoring);
    next.add(id);
    restoring = next;

    try {
      const result = await invoke("restore_hardening", { id });
      results = { ...results, [id]: result };
      items = await invoke("get_hardening_status");
    } finally {
      const done = new Set(restoring);
      done.delete(id);
      restoring = done;
    }
  }

  async function applyAll() {
    const pending = items.filter((i) => !i.is_applied);
    for (const i of pending) {
      await applyItem(i.id);
    }
  }

  async function restoreAll() {
    const applied = items.filter((i) => i.is_applied);
    for (const i of applied) {
      await restoreItem(i.id);
    }
  }

  $effect(() => {
    loadStatus();
  });

  let appliedCount = $derived(items.filter((i) => i.is_applied).length);
  let totalCount = $derived(items.length);
  let categories = $derived([...new Set(items.map((i) => i.category))]);
</script>

<div class="hardening">
  <div class="header">
    <div class="header-left">
      <h2>Security Hardening</h2>
      {#if items.length > 0}
        <span class="score-badge" class:all-good={appliedCount === totalCount}>
          {appliedCount}/{totalCount} hardened
        </span>
      {/if}
    </div>
    <div class="header-actions">
      <button class="btn btn-secondary" onclick={loadStatus} disabled={loading}>
        <span class:spin={loading}>↻</span> Refresh
      </button>
      {#if appliedCount > 0}
        <button class="btn btn-restore" onclick={restoreAll}>
          ↩ Restore All
        </button>
      {/if}
      {#if appliedCount < totalCount}
        <button class="btn btn-primary" onclick={applyAll}>
          <svg width="1em" height="1em" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:middle"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg> Harden All
        </button>
      {/if}
    </div>
  </div>

  {#if loading && items.length === 0}
    <div class="loading">
      <div class="spinner"></div>
      <p>Scanning security settings...</p>
    </div>
  {:else if items.length === 0}
    <div class="empty-state">
      <p>No hardening options available on this system.</p>
    </div>
  {:else}
    <div class="info-banner">
      <span class="info-icon"><svg width="1em" height="1em" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg></span>
      <span>Hardens your system based on CIS Benchmarks and KSPP guidelines. Changes are applied at runtime and persisted across reboots. Root access is required.</span>
    </div>

    <div class="tweaks-body">
      {#each categories as category}
        <section class="tweak-section">
          <h3 class="section-title">
            <span class="section-icon">{@html categoryIcons[category] || fallbackIcon}</span>
            {category}
          </h3>

          <div class="tweak-list">
            {#each items.filter((i) => i.category === category) as item}
              <div class="tweak-card" class:applied={item.is_applied}>
                <div class="tweak-info">
                  <div class="tweak-header">
                    <span class="tweak-name">{item.name}</span>
                    {#if item.is_applied}
                      <span class="status-badge applied">Hardened</span>
                    {:else}
                      <span class="status-badge pending">Exposed</span>
                    {/if}
                  </div>
                  <p class="tweak-desc">{item.description}</p>
                  <div class="sysctl-info">
                    <span class="sysctl-label">File:</span>
                    <span class="sysctl-path">/proc/sys/{item.sysctl_path.replaceAll(".", "/")}</span>
                    <span class="sysctl-label">Command:</span>
                    <span class="sysctl-path">sysctl -w {item.sysctl_path}={item.is_applied ? item.current_value : item.recommended_value}</span>
                  </div>
                  <div class="tweak-values">
                    <span class="value-item">
                      <span class="value-label">Current:</span>
                      <span class="value-data" class:good={item.is_applied} class:warn={!item.is_applied}>{item.current_value}</span>
                    </span>
                    {#if !item.is_applied}
                      <span class="value-item">
                        <span class="value-label">Recommended:</span>
                        <span class="value-data good">{item.recommended_value}</span>
                      </span>
                    {/if}
                    <span class="value-item">
                      <span class="value-label">Default:</span>
                      <span class="value-data default">{item.default_value}</span>
                    </span>
                  </div>
                </div>
                <div class="tweak-action">
                  {#if item.is_applied}
                    <button
                      class="btn btn-restore-small"
                      onclick={() => restoreItem(item.id)}
                      disabled={restoring.has(item.id)}
                    >
                      {#if restoring.has(item.id)}
                        <span class="spin">↻</span>
                      {:else}
                        Restore
                      {/if}
                    </button>
                  {:else}
                    <button
                      class="btn btn-apply"
                      onclick={() => applyItem(item.id)}
                      disabled={applying.has(item.id)}
                    >
                      {#if applying.has(item.id)}
                        <span class="spin">↻</span>
                      {:else}
                        Harden
                      {/if}
                    </button>
                  {/if}
                </div>
              </div>

              {#if results[item.id] && !results[item.id].success}
                <div class="result-error">
                  <span>✗</span> {results[item.id].message}
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
  .hardening {
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
    transition: all 0.15s;
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
    background: #059669;
    color: #fff;
  }

  .btn-primary:hover {
    background: #047857;
  }

  .btn-apply {
    background: #1e293b;
    border: 1px solid #059669;
    color: #6ee7b7;
    padding: 6px 16px;
    border-radius: 8px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
  }

  .btn-apply:hover {
    background: #059669;
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
    transition: all 0.15s;
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
    transition: all 0.15s;
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
    border-top-color: #059669;
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
    background: #ef444418;
    color: #f87171;
  }

  .tweak-desc {
    font-size: 0.88rem;
    color: #94a3b8;
    margin: 0 0 6px 0;
    line-height: 1.4;
  }

  .sysctl-info {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 8px;
    margin-bottom: 8px;
  }

  .sysctl-label {
    font-size: 0.78rem;
    color: #64748b;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .sysctl-path {
    font-size: 0.8rem;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    color: #64748b;
    background: #0f172a;
    padding: 3px 8px;
    border-radius: 4px;
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
    color: #f87171;
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
