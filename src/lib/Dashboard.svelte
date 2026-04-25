<script>
  import { invoke } from "@tauri-apps/api/core";

  let { onNavigate, active = true } = $props();

  let stats = $state(null);
  let gpu = $state(null);
  let disk = $state(null);
  let updates = $state(null);
  let updatesLoading = $state(true);
  let pollTimer = $state(null);
  let gpuTimer = $state(null);

  function formatBytes(bytes) {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
  }

  function formatRate(bytesPerSec) {
    if (bytesPerSec < 1024) return bytesPerSec.toFixed(0) + " B/s";
    if (bytesPerSec < 1048576) return (bytesPerSec / 1024).toFixed(1) + " KB/s";
    return (bytesPerSec / 1048576).toFixed(1) + " MB/s";
  }

  async function loadStats() {
    try {
      stats = await invoke("get_system_stats");
    } catch {}
  }

  async function loadGpu() {
    try {
      const hw = await invoke("get_hardware_info");
      gpu = hw.gpus?.length > 0 ? hw.gpus[0] : null;
    } catch {}
  }

  async function loadStatic() {
    try {
      disk = await invoke("get_disk_overview");
    } catch {}

    try {
      updates = await invoke("check_updates");
    } catch {}
    updatesLoading = false;
  }

  function startPolling() {
    loadStats();
    loadGpu();
    pollTimer = setInterval(loadStats, 2000);
    gpuTimer = setInterval(loadGpu, 10000);
  }

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
    if (gpuTimer) {
      clearInterval(gpuTimer);
      gpuTimer = null;
    }
  }

  $effect(() => {
    loadStatic();
  });

  // Only poll while this view is the active tab so background tabs don't
  // burn CPU on Tauri invokes every couple of seconds.
  $effect(() => {
    if (active) {
      startPolling();
      return () => stopPolling();
    }
  });

  let rootPartition = $derived(
    disk?.partitions?.find((p) => p.mount_point === "/") ?? null
  );

  let activeIface = $derived(
    stats?.net_interfaces?.find((n) => n.state === "up" && n.name !== "lo") ?? null
  );

  let totalRx = $derived(
    stats?.net_interfaces
      ?.filter((n) => n.name !== "lo")
      .reduce((sum, n) => sum + n.rx_rate, 0) ?? 0
  );

  let totalTx = $derived(
    stats?.net_interfaces
      ?.filter((n) => n.name !== "lo")
      .reduce((sum, n) => sum + n.tx_rate, 0) ?? 0
  );

  let updateCount = $derived(
    (updates?.updates?.length ?? 0) + (updates?.flatpak_updates?.length ?? 0)
  );

  let memPercent = $derived(
    stats?.memory
      ? ((stats.memory.used_bytes / stats.memory.total_bytes) * 100)
      : 0
  );

  let gpuHasStats = $derived(
    gpu && (gpu.temperature_c > 0 || gpu.vram_total_mb > 0)
  );
</script>

<div class="dashboard">
  <header class="dash-header">
    <h2>Dashboard</h2>
    <span class="subtitle">System overview</span>
  </header>

  <div class="grid">
    <!-- CPU -->
    <div class="card">
      <div class="card-icon cpu">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <rect x="4" y="4" width="16" height="16" rx="2"/>
          <rect x="9" y="9" width="6" height="6"/>
          <path d="M9 1v3M15 1v3M9 20v3M15 20v3M20 9h3M20 14h3M1 9h3M1 14h3"/>
        </svg>
      </div>
      <div class="card-body">
        <span class="card-label">CPU</span>
        <span class="card-value">{stats ? stats.cpu_usage.toFixed(1) : "—"}%</span>
        <div class="bar-track">
          <div class="bar-fill cpu-fill" style="width: {stats?.cpu_usage ?? 0}%"></div>
        </div>
        <span class="card-detail">
          Load {stats ? stats.load_average.map((l) => l.toFixed(2)).join("  ") : "—"}
        </span>
      </div>
    </div>

    <!-- GPU -->
    <div class="card">
      <div class="card-icon gpu">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <rect x="2" y="6" width="20" height="12" rx="2"/>
          <path d="M6 10h2v4H6zM10 10h2v4h-2zM14 10h4v4h-4z"/>
        </svg>
      </div>
      <div class="card-body">
        <span class="card-label">GPU</span>
        {#if gpuHasStats}
          <span class="card-value">{gpu.temperature_c}°C</span>
          {#if gpu.vram_total_mb > 0}
            <div class="bar-track">
              <div class="bar-fill gpu-fill" style="width: {(gpu.vram_used_mb / gpu.vram_total_mb) * 100}%"></div>
            </div>
            <span class="card-detail">VRAM {gpu.vram_used_mb} / {gpu.vram_total_mb} MB</span>
          {:else}
            <span class="card-detail">{gpu.name}</span>
          {/if}
        {:else if gpu}
          <span class="card-value">—</span>
          <span class="card-detail">{gpu.name}</span>
        {:else}
          <span class="card-value">—</span>
          <span class="card-detail">No GPU detected</span>
        {/if}
      </div>
    </div>

    <!-- Memory -->
    <div class="card">
      <div class="card-icon mem">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <rect x="3" y="5" width="18" height="14" rx="2"/>
          <path d="M7 9v6M11 9v6M15 9v6M19 9v6"/>
        </svg>
      </div>
      <div class="card-body">
        <span class="card-label">Memory</span>
        <span class="card-value">{memPercent.toFixed(1)}%</span>
        <div class="bar-track">
          <div class="bar-fill mem-fill" style="width: {memPercent}%"></div>
        </div>
        <span class="card-detail">
          {stats ? formatBytes(stats.memory.used_bytes) : "—"} / {stats ? formatBytes(stats.memory.total_bytes) : "—"}
        </span>
      </div>
    </div>

    <!-- Uptime -->
    <div class="card">
      <div class="card-icon uptime">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"/>
          <polyline points="12 6 12 12 16 14"/>
        </svg>
      </div>
      <div class="card-body">
        <span class="card-label">Uptime</span>
        <span class="card-value uptime-value">{stats?.uptime ?? "—"}</span>
      </div>
    </div>

    <!-- Disk -->
    <div class="card">
      <div class="card-icon disk">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <ellipse cx="12" cy="5" rx="9" ry="3"/>
          <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/>
          <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/>
        </svg>
      </div>
      <div class="card-body">
        <span class="card-label">Disk <code>/</code></span>
        {#if rootPartition}
          <span class="card-value">{rootPartition.usage_percent.toFixed(1)}%</span>
          <div class="bar-track">
            <div class="bar-fill disk-fill" class:disk-warn={rootPartition.usage_percent > 85} style="width: {rootPartition.usage_percent}%"></div>
          </div>
          <span class="card-detail">
            {formatBytes(rootPartition.used_bytes)} / {formatBytes(rootPartition.total_bytes)}
          </span>
        {:else}
          <span class="card-value">—</span>
        {/if}
      </div>
    </div>

    <!-- Network -->
    <div class="card">
      <div class="card-icon net">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 20v-6M6 20v-4M18 20v-8"/>
          <path d="M6 8l6-4 6 4"/>
        </svg>
      </div>
      <div class="card-body">
        <span class="card-label">Network</span>
        <div class="net-rates">
          <span class="rate down">↓ {formatRate(totalRx)}</span>
          <span class="rate up">↑ {formatRate(totalTx)}</span>
        </div>
        <span class="card-detail">{activeIface?.name ?? "no interface"}</span>
      </div>
    </div>

    <!-- Updates -->
    <button class="card card-btn" onclick={() => onNavigate("updates")}>
      <div class="card-icon updates" class:has-updates={updateCount > 0}>
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 5v14M5 12l7-7 7 7"/>
        </svg>
      </div>
      <div class="card-body">
        <span class="card-label">Updates</span>
        {#if updatesLoading}
          <span class="card-value checking">...</span>
          <span class="card-detail">Checking...</span>
        {:else}
          <span class="card-value" class:has-updates={updateCount > 0}>{updateCount}</span>
          <span class="card-detail">
            {updateCount === 0 ? "System is up to date" : updateCount === 1 ? "update available" : "updates available"}
          </span>
        {/if}
      </div>
    </button>

  </div>
</div>

<style>
  .dashboard {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    padding: 0;
    height: 100%;
  }

  .dash-header {
    display: flex;
    align-items: baseline;
    gap: 0.75rem;
  }
  .dash-header h2 {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary);
  }
  .subtitle {
    font-size: 0.85rem;
    color: var(--text-muted);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 1rem;
    flex: 1;
    align-content: start;
  }

  .card {
    display: flex;
    align-items: flex-start;
    gap: 1rem;
    background: var(--surface-1);
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    padding: 1.5rem;
    transition: border-color 0.15s, box-shadow 0.15s;
    min-height: 120px;
  }
  .card:hover {
    border-color: var(--border-default);
  }

  .card-btn {
    cursor: pointer;
    text-align: left;
    font: inherit;
    color: inherit;
  }
  .card-btn:hover {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent-soft), 0 2px 8px rgba(0,0,0,0.2);
  }

  .card-icon {
    flex-shrink: 0;
    width: 44px;
    height: 44px;
    border-radius: 11px;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-top: 2px;
  }
  .card-icon svg {
    width: 22px;
    height: 22px;
  }

  .card-icon.cpu  { background: rgba(99, 102, 241, 0.12); color: #818cf8; }
  .card-icon.gpu  { background: rgba(168, 85, 247, 0.12); color: #c084fc; }
  .card-icon.mem  { background: rgba(45, 212, 191, 0.12); color: #2dd4bf; }
  .card-icon.uptime { background: rgba(251, 191, 36, 0.12); color: #fbbf24; }
  .card-icon.disk { background: rgba(96, 165, 250, 0.12); color: #60a5fa; }
  .card-icon.net  { background: rgba(52, 211, 153, 0.12); color: #34d399; }
  .card-icon.updates { background: rgba(255, 107, 53, 0.12); color: var(--accent); }
  .card-icon.updates.has-updates { background: rgba(255, 107, 53, 0.2); }


  .card-body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .card-label {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }
  .card-label code {
    font-size: 0.7rem;
    background: var(--surface-3);
    padding: 1px 5px;
    border-radius: 4px;
    color: var(--text-secondary);
    text-transform: none;
    letter-spacing: 0;
  }

  .card-value {
    font-size: 1.75rem;
    font-weight: 700;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    color: var(--text-primary);
    line-height: 1.2;
  }
  .card-value.uptime-value {
    font-size: 1.1rem;
    font-weight: 600;
  }
  .card-value.has-updates { color: var(--accent); }
  .card-value.checking { color: var(--text-muted); }


  .card-detail {
    font-size: 0.78rem;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Usage bars */
  .bar-track {
    width: 100%;
    height: 7px;
    background: var(--surface-3);
    border-radius: 4px;
    overflow: hidden;
    margin: 4px 0;
  }
  .bar-fill {
    height: 100%;
    border-radius: 3px;
    transition: width 0.6s ease;
  }
  .cpu-fill { background: linear-gradient(90deg, #818cf8, #6366f1); }
  .gpu-fill { background: linear-gradient(90deg, #c084fc, #a855f7); }
  .mem-fill { background: linear-gradient(90deg, #2dd4bf, #14b8a6); }
  .disk-fill { background: linear-gradient(90deg, #60a5fa, #3b82f6); }
  .disk-fill.disk-warn { background: linear-gradient(90deg, #fbbf24, #f87171); }

  /* Network rates */
  .net-rates {
    display: flex;
    gap: 1rem;
    align-items: baseline;
  }
  .rate {
    font-size: 1.1rem;
    font-weight: 700;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    color: var(--text-primary);
  }
  .rate.down::first-letter { color: #34d399; }
  .rate.up::first-letter { color: #f87171; }
</style>
