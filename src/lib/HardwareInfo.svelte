<script>
  import { invoke } from "@tauri-apps/api/core";

  let { active = true } = $props();

  let hwInfo = $state(null);
  let loading = $state(true);
  let error = $state(null);

  // Live monitoring state
  let stats = $state(null);
  let pollTimer = $state(null);
  const HISTORY_LEN = 60;
  let cpuHistory = $state([]);
  let memHistory = $state([]);
  let netRxHistory = $state([]);
  let netTxHistory = $state([]);

  const RATE_OPTIONS = [
    { label: "0.5s", value: 500 },
    { label: "1s", value: 1000 },
    { label: "1.5s", value: 1500 },
    { label: "2s", value: 2000 },
    { label: "3s", value: 3000 },
    { label: "5s", value: 5000 },
    { label: "10s", value: 10000 },
  ];
  let pollRate = $state(1500);

  const sections = [
    { id: "system", label: "System", icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" width="1em" height="1em"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>` },
    { id: "cpu", label: "CPU", icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" width="1em" height="1em"><rect x="4" y="4" width="16" height="16" rx="2"/><rect x="9" y="9" width="6" height="6"/><path d="M9 1v3M15 1v3M9 20v3M15 20v3M20 9h3M20 14h3M1 9h3M1 14h3"/></svg>` },
    { id: "memory", label: "Memory", icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" width="1em" height="1em"><rect x="3" y="5" width="18" height="14" rx="2"/><path d="M7 9v6M11 9v6M15 9v6M19 9v6"/></svg>` },
    { id: "gpu", label: "GPU", icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" width="1em" height="1em"><rect x="2" y="6" width="20" height="12" rx="2"/><path d="M6 10h2v4H6zM10 10h2v4h-2zM14 10h4v4h-4z"/></svg>` },
    { id: "storage", label: "Storage", icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" width="1em" height="1em"><ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/></svg>` },
    { id: "network", label: "Network", icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" width="1em" height="1em"><circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg>` },
    { id: "sensors", label: "Sensors", icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" width="1em" height="1em"><path d="M14 14.76V3.5a2.5 2.5 0 0 0-5 0v11.26a4.5 4.5 0 1 0 5 0z"/></svg>` },
    { id: "processes", label: "Processes", icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" width="1em" height="1em"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>` },
  ];

  let visibleSections = $state(new Set());

  let baseSensorData = $derived(
    stats?.sensors?.length ? stats.sensors : hwInfo?.sensors ?? []
  );

  let sensorData = $derived.by(() => {
    const base = baseSensorData;
    const gpus = stats?.gpus ?? hwInfo?.gpus ?? [];
    const gpuReadings = gpus
      .filter((g) => g.temperature_c > 0)
      .map((g, i) => ({
        label: gpus.length > 1 ? `GPU ${i} (${g.name || "Unknown"})` : (g.name || "GPU"),
        value: g.temperature_c,
        unit: "°C",
        high: 80,
        critical: 95,
      }));
    if (gpuReadings.length === 0) return base;
    return [...base, { name: "GPU Temperature", readings: gpuReadings }];
  });

  function formatBytes(bytes) {
    if (!bytes || bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
  }

  function pct(used, total) {
    if (!total) return 0;
    return Math.round((used / total) * 100);
  }

  function tempColor(val, high, critical) {
    if (critical && val >= critical) return "#ef4444";
    if (high && val >= high) return "#f59e0b";
    if (val > 80) return "#f59e0b";
    if (val > 60) return "#eab308";
    return "#22c55e";
  }

  function usageColor(pct) {
    if (pct > 90) return "#ef4444";
    if (pct > 75) return "#f59e0b";
    if (pct > 50) return "#7c3aed";
    return "#22c55e";
  }

  function formatRate(bytesPerSec) {
    if (bytesPerSec < 1024) return bytesPerSec.toFixed(0) + " B/s";
    if (bytesPerSec < 1024 * 1024) return (bytesPerSec / 1024).toFixed(1) + " KB/s";
    return (bytesPerSec / 1024 / 1024).toFixed(1) + " MB/s";
  }

  function sparklinePath(data, width, height) {
    if (data.length < 2) return "";
    const max = Math.max(...data, 1);
    const step = width / (HISTORY_LEN - 1);
    const points = data.map((v, i) => {
      const x = (i + (HISTORY_LEN - data.length)) * step;
      const y = height - (v / max) * height;
      return `${x},${y}`;
    });
    return "M" + points.join("L");
  }

  function sparklineAreaPath(data, width, height) {
    if (data.length < 2) return "";
    const max = Math.max(...data, 1);
    const step = width / (HISTORY_LEN - 1);
    const offset = (HISTORY_LEN - data.length) * step;
    const points = data.map((v, i) => {
      const x = offset + i * step;
      const y = height - (v / max) * height;
      return `${x},${y}`;
    });
    const lastX = offset + (data.length - 1) * step;
    const firstX = offset;
    return `M${firstX},${height} L${points.join("L")} L${lastX},${height} Z`;
  }

  async function loadHardware() {
    loading = true;
    error = null;
    try {
      hwInfo = await invoke("get_hardware_info");
    } catch (e) {
      error = String(e);
    }
    loading = false;
  }

  async function pollStats() {
    try {
      stats = await invoke("get_system_stats");
      // Append to histories
      cpuHistory = [...cpuHistory, stats.cpu_usage].slice(-HISTORY_LEN);
      const memPct = pct(stats.memory.used_bytes, stats.memory.total_bytes);
      memHistory = [...memHistory, memPct].slice(-HISTORY_LEN);

      // Sum all rx/tx rates
      const totalRx = stats.net_interfaces.reduce((s, n) => s + n.rx_rate, 0);
      const totalTx = stats.net_interfaces.reduce((s, n) => s + n.tx_rate, 0);
      netRxHistory = [...netRxHistory, totalRx].slice(-HISTORY_LEN);
      netTxHistory = [...netTxHistory, totalTx].slice(-HISTORY_LEN);
    } catch (e) {
      // Silently ignore polling errors
    }
  }

  function startPolling() {
    if (pollTimer) return;
    pollStats(); // immediate first call
    pollTimer = setInterval(pollStats, pollRate);
  }

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  }

  function setPollRate(ms) {
    pollRate = ms;
    stopPolling();
    startPolling();
  }

  $effect(() => {
    loadHardware();
  });

  // Only poll system stats while this view is the active tab.
  $effect(() => {
    if (active) {
      startPolling();
      return () => stopPolling();
    }
  });
</script>

<div class="hardware-info">
  <div class="header">
    <div class="header-left">
      <h2>System Monitor</h2>
      {#if hwInfo}
        <span class="host-badge">{hwInfo.host.hostname}</span>
      {/if}
      {#if stats}
        <span class="live-dot"></span>
      {/if}
    </div>
    <div class="header-right">
      <div class="rate-selector">
        <span class="rate-label">Refresh</span>
        <div class="rate-options">
          {#each RATE_OPTIONS as opt}
            <button
              class="rate-btn"
              class:active={pollRate === opt.value}
              onclick={() => setPollRate(opt.value)}
            >{opt.label}</button>
          {/each}
        </div>
      </div>
      <button class="refresh-btn" onclick={loadHardware} disabled={loading}>
        <span class:spin={loading}>↻</span>
      </button>
    </div>
  </div>

  {#if loading && !hwInfo}
    <div class="loading">
      <div class="spinner"></div>
      <p>Scanning hardware...</p>
    </div>
  {:else if error}
    <div class="error-box"><strong>Error:</strong> {error}</div>
  {:else if hwInfo}
    <!-- Filter chips -->
    <div class="filter-chips">
      {#each sections as sec}
        <button
          class="chip"
          class:active={visibleSections.has(sec.id)}
          onclick={() => {
            const next = new Set(visibleSections);
            if (next.has(sec.id)) next.delete(sec.id);
            else next.add(sec.id);
            visibleSections = next;
          }}
        >
          <span class="chip-icon">{@html sec.icon}</span>
          {sec.label}
        </button>
      {/each}
    </div>

    <!-- Unified scrollable panel -->
    <div class="panel-body">

      <!-- SYSTEM -->
      {#if visibleSections.size === 0 || visibleSections.has("system")}
        <section class="panel-section">
          <h3 class="section-title"><span class="section-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" width="1em" height="1em"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg></span> System</h3>
          <div class="section-card">
            <div class="detail-grid">
              <div class="detail-item"><span class="detail-label">Hostname</span><span class="detail-value">{hwInfo.host.hostname}</span></div>
              <div class="detail-item"><span class="detail-label">Board</span><span class="detail-value">{hwInfo.host.board_vendor} {hwInfo.host.board_name}</span></div>
              <div class="detail-item"><span class="detail-label">BIOS</span><span class="detail-value">{hwInfo.host.bios_vendor} {hwInfo.host.bios_version}</span></div>
              <div class="detail-item"><span class="detail-label">Kernel</span><span class="detail-value">{hwInfo.host.kernel}</span></div>
              <div class="detail-item"><span class="detail-label">Uptime</span><span class="detail-value">{stats?.uptime || hwInfo.host.uptime}</span></div>
              {#if stats?.load_average}
                <div class="detail-item"><span class="detail-label">Load Average</span><span class="detail-value">{stats.load_average[0].toFixed(2)} / {stats.load_average[1].toFixed(2)} / {stats.load_average[2].toFixed(2)}</span></div>
              {/if}
            </div>
          </div>
        </section>
      {/if}

      <!-- CPU -->
      {#if visibleSections.size === 0 || visibleSections.has("cpu")}
        <section class="panel-section">
          <h3 class="section-title"><span class="section-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" width="1em" height="1em"><rect x="4" y="4" width="16" height="16" rx="2"/><rect x="9" y="9" width="6" height="6"/><path d="M9 1v3M15 1v3M9 20v3M15 20v3M20 9h3M20 14h3M1 9h3M1 14h3"/></svg></span> CPU</h3>

          <!-- Live sparkline + specs side by side -->
          <div class="section-split">
            {#if stats}
              <div class="section-card flex-1">
                <div class="gauge-header">
                  <span class="gauge-label">Usage</span>
                  <span class="gauge-value" style="color: {usageColor(stats.cpu_usage)}">{stats.cpu_usage.toFixed(1)}%</span>
                </div>
                <svg class="sparkline" viewBox="0 0 200 50" preserveAspectRatio="none">
                  <path d={sparklineAreaPath(cpuHistory, 200, 50)} fill="#7c3aed18" />
                  <path d={sparklinePath(cpuHistory, 200, 50)} fill="none" stroke="#7c3aed" stroke-width="1.5" />
                </svg>
                {#if stats.load_average}
                  <div class="gauge-sub">Load: {stats.load_average[0].toFixed(2)} / {stats.load_average[1].toFixed(2)} / {stats.load_average[2].toFixed(2)}</div>
                {/if}
              </div>
            {/if}
            <div class="section-card flex-1">
              <div class="spec-headline">{hwInfo.cpu.model || "Unknown Processor"}</div>
              <div class="detail-grid compact">
                <div class="detail-item"><span class="detail-label">Vendor</span><span class="detail-value">{hwInfo.cpu.vendor}</span></div>
                <div class="detail-item"><span class="detail-label">Architecture</span><span class="detail-value">{hwInfo.cpu.architecture}</span></div>
                <div class="detail-item"><span class="detail-label">Cores</span><span class="detail-value">{hwInfo.cpu.cores}</span></div>
                <div class="detail-item"><span class="detail-label">Threads</span><span class="detail-value">{hwInfo.cpu.threads}</span></div>
                {#if hwInfo.cpu.max_freq_mhz > 0}
                  <div class="detail-item"><span class="detail-label">Max Frequency</span><span class="detail-value">{(hwInfo.cpu.max_freq_mhz / 1000).toFixed(2)} GHz</span></div>
                {/if}
                {#if hwInfo.cpu.cache}
                  <div class="detail-item"><span class="detail-label">Cache</span><span class="detail-value">{hwInfo.cpu.cache}</span></div>
                {/if}
              </div>
            </div>
          </div>

          <!-- Per-core bars -->
          {#if stats && stats.per_core_usage.length > 0}
            <div class="section-card">
              <div class="sub-title">Cores</div>
              <div class="core-grid">
                {#each stats.per_core_usage as usage, i}
                  <div class="core-bar-item">
                    <div class="core-label">Core {i}</div>
                    <div class="core-bar-track">
                      <div class="core-bar-fill" style="width: {usage}%; background: {usageColor(usage)}"></div>
                    </div>
                    <div class="core-pct">{usage.toFixed(0)}%</div>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        </section>
      {/if}

      <!-- MEMORY -->
      {#if visibleSections.size === 0 || visibleSections.has("memory")}
        <section class="panel-section">
          <h3 class="section-title"><span class="section-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" width="1em" height="1em"><rect x="3" y="5" width="18" height="14" rx="2"/><path d="M7 9v6M11 9v6M15 9v6M19 9v6"/></svg></span> Memory</h3>

          <div class="section-card">
            <div class="gauge-header">
              <span class="gauge-label">{formatBytes((stats ?? hwInfo).memory.total_bytes)} RAM</span>
              <span class="gauge-value" style="color: {usageColor(pct((stats ?? hwInfo).memory.used_bytes, (stats ?? hwInfo).memory.total_bytes))}">{pct((stats ?? hwInfo).memory.used_bytes, (stats ?? hwInfo).memory.total_bytes)}%</span>
            </div>
            {#if memHistory.length >= 2}
              <svg class="sparkline" viewBox="0 0 200 50" preserveAspectRatio="none">
                <path d={sparklineAreaPath(memHistory, 200, 50)} fill="#7c3aed18" />
                <path d={sparklinePath(memHistory, 200, 50)} fill="none" stroke="#7c3aed" stroke-width="1.5" />
              </svg>
            {/if}
            <div class="mem-bar-wrapper">
              <div class="mem-bar-track">
                <div class="mem-bar-fill" style="width: {pct((stats ?? hwInfo).memory.used_bytes, (stats ?? hwInfo).memory.total_bytes)}%; background: #7c3aed"></div>
              </div>
              <div class="mem-bar-labels">
                <span>{formatBytes((stats ?? hwInfo).memory.used_bytes)} used</span>
                <span>{formatBytes((stats ?? hwInfo).memory.available_bytes)} available</span>
              </div>
            </div>
            {#if (stats ?? hwInfo).memory.swap_total_bytes > 0}
              <div class="swap-bar-wrapper">
                <div class="sub-title">Swap</div>
                <div class="swap-bar-track">
                  <div class="swap-bar-fill" style="width: {pct((stats ?? hwInfo).memory.swap_used_bytes, (stats ?? hwInfo).memory.swap_total_bytes)}%"></div>
                </div>
                <div class="swap-labels">
                  <span>{formatBytes((stats ?? hwInfo).memory.swap_used_bytes)} used</span>
                  <span>{formatBytes((stats ?? hwInfo).memory.swap_total_bytes)} total</span>
                </div>
              </div>
            {/if}
          </div>
        </section>
      {/if}

      <!-- GPU -->
      {#if visibleSections.size === 0 || visibleSections.has("gpu")}
        <section class="panel-section">
          <h3 class="section-title"><span class="section-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" width="1em" height="1em"><rect x="2" y="6" width="20" height="12" rx="2"/><path d="M6 10h2v4H6zM10 10h2v4h-2zM14 10h4v4h-4z"/></svg></span> GPU</h3>
          {#if hwInfo.gpus.length === 0}
            <div class="section-card">
              <p class="empty-msg">No GPU detected via lspci or sysfs.</p>
            </div>
          {:else}
            {#each hwInfo.gpus as gpu, i}
              <div class="section-card">
                <div class="gpu-header">
                  <span class="gpu-index">GPU {i}</span>
                  <span class="gpu-name">{gpu.name || "Unknown"}</span>
                </div>

                {#if gpu.vram_total_mb > 0}
                  <div class="mem-bar-wrapper">
                    <div class="mem-bar-track">
                      <div class="mem-bar-fill" style="width: {pct(gpu.vram_used_mb, gpu.vram_total_mb)}%; background: #7c3aed"></div>
                    </div>
                    <div class="mem-bar-labels">
                      <span>VRAM: {gpu.vram_used_mb} / {gpu.vram_total_mb} MiB</span>
                      <span>{gpu.vram_free_mb} MiB free</span>
                    </div>
                  </div>
                {/if}

                <div class="detail-grid compact">
                  <div class="detail-item"><span class="detail-label">Vendor</span><span class="detail-value">{gpu.vendor || "—"}</span></div>
                  <div class="detail-item"><span class="detail-label">Driver</span><span class="detail-value">{gpu.driver || "—"}</span></div>

                  {#if gpu.clock_gpu_mhz > 0}
                    <div class="detail-item"><span class="detail-label">GPU Clock</span><span class="detail-value">{gpu.clock_gpu_mhz} / {gpu.clock_gpu_max_mhz} MHz</span></div>
                  {/if}
                  {#if gpu.clock_mem_mhz > 0}
                    <div class="detail-item"><span class="detail-label">Mem Clock</span><span class="detail-value">{gpu.clock_mem_mhz} / {gpu.clock_mem_max_mhz} MHz</span></div>
                  {/if}
                  {#if gpu.power_draw_w > 0}
                    <div class="detail-item"><span class="detail-label">Power</span><span class="detail-value">{gpu.power_draw_w.toFixed(1)} / {gpu.power_limit_w.toFixed(0)} W</span></div>
                  {/if}
                  {#if gpu.fan_speed_pct}
                    <div class="detail-item"><span class="detail-label">Fan</span><span class="detail-value">{gpu.fan_speed_pct}</span></div>
                  {/if}
                  {#if gpu.pstate}
                    <div class="detail-item"><span class="detail-label">P-State</span><span class="detail-value">{gpu.pstate}</span></div>
                  {/if}
                  {#if gpu.pcie_link_speed}
                    <div class="detail-item"><span class="detail-label">PCIe</span><span class="detail-value">{gpu.pcie_link_speed} {gpu.pcie_link_width}</span></div>
                  {/if}
                  <div class="detail-item"><span class="detail-label">PCI Slot</span><span class="detail-value mono">{gpu.pci_slot || "—"}</span></div>
                </div>
              </div>
            {/each}
          {/if}
        </section>
      {/if}

      <!-- STORAGE -->
      {#if visibleSections.size === 0 || visibleSections.has("storage")}
        <section class="panel-section">
          <h3 class="section-title"><span class="section-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" width="1em" height="1em"><ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/></svg></span> Storage</h3>
          {#if hwInfo.disks.length === 0}
            <div class="section-card">
              <p class="empty-msg">No disks detected.</p>
            </div>
          {:else}
            {#each hwInfo.disks as disk}
              <div class="section-card">
                <div class="disk-header">
                  <div class="disk-title">
                    <span class="disk-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" width="1em" height="1em"><ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/></svg></span>
                    <span class="disk-name">/dev/{disk.name}</span>
                    <span class="type-badge" class:ssd={disk.disk_type === "SSD"} class:hdd={disk.disk_type === "HDD"}>
                      {disk.disk_type}
                    </span>
                  </div>
                  <span class="disk-size">{formatBytes(disk.size_bytes)}</span>
                </div>
                {#if disk.model}
                  <div class="disk-model">{disk.model}</div>
                {/if}
                {#if disk.partitions.length > 0}
                  <div class="partition-list">
                    {#each disk.partitions as part}
                      <div class="partition-row">
                        <span class="part-name">{part.name}</span>
                        <span class="part-size">{formatBytes(part.size_bytes)}</span>
                        <span class="part-fs">{part.fstype || "—"}</span>
                        <span class="part-mount">{part.mountpoint || "—"}</span>
                      </div>
                    {/each}
                  </div>
                {/if}
              </div>
            {/each}
          {/if}
        </section>
      {/if}

      <!-- NETWORK -->
      {#if visibleSections.size === 0 || visibleSections.has("network")}
        <section class="panel-section">
          <h3 class="section-title"><span class="section-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" width="1em" height="1em"><circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg></span> Network</h3>

          <!-- Live network sparkline -->
          {#if stats}
            <div class="section-card">
              <div class="gauge-header">
                <span class="gauge-label">Throughput</span>
                <span class="gauge-value net-rates">
                  <span class="rx-rate">↓ {formatRate(stats.net_interfaces.reduce((s, n) => s + n.rx_rate, 0))}</span>
                  <span class="tx-rate">↑ {formatRate(stats.net_interfaces.reduce((s, n) => s + n.tx_rate, 0))}</span>
                </span>
              </div>
              <svg class="sparkline" viewBox="0 0 200 50" preserveAspectRatio="none">
                <path d={sparklineAreaPath(netRxHistory, 200, 50)} fill="#22c55e10" />
                <path d={sparklinePath(netRxHistory, 200, 50)} fill="none" stroke="#22c55e" stroke-width="1.5" />
                <path d={sparklinePath(netTxHistory, 200, 50)} fill="none" stroke="#f59e0b" stroke-width="1" stroke-dasharray="3,2" />
              </svg>
              <div class="gauge-sub">
                <span style="color: #22c55e">● RX</span>
                <span style="color: #f59e0b">● TX</span>
                <span style="margin-left: auto">Uptime: {stats.uptime}</span>
              </div>
            </div>
          {/if}

          <!-- Interfaces -->
          {#if hwInfo.network.length > 0}
            <div class="section-card">
              <div class="sub-title">Interfaces</div>
              <div class="net-list">
                {#each hwInfo.network as iface}
                  <div class="net-card" class:net-up={iface.state === "up"}>
                    <div class="net-header">
                      <div class="net-title">
                        <span class="state-dot" class:up={iface.state === "up"}></span>
                        <span class="net-name">{iface.name}</span>
                        <span class="net-type-badge">{iface.iface_type}</span>
                      </div>
                      {#if stats}
                        {@const liveIface = stats.net_interfaces.find(n => n.name === iface.name)}
                        {#if liveIface}
                          <span class="gauge-value net-rates" style="font-size: 0.75rem">
                            <span class="rx-rate">↓ {formatRate(liveIface.rx_rate)}</span>
                            <span class="tx-rate">↑ {formatRate(liveIface.tx_rate)}</span>
                          </span>
                        {:else}
                          <span class="net-state">{iface.state}</span>
                        {/if}
                      {:else}
                        <span class="net-state">{iface.state}</span>
                      {/if}
                    </div>
                    <div class="detail-grid compact">
                      <div class="detail-item"><span class="detail-label">MAC</span><span class="detail-value mono">{iface.mac || "—"}</span></div>
                      <div class="detail-item"><span class="detail-label">IPv4</span><span class="detail-value mono">{iface.ipv4 || "—"}</span></div>
                      {#if iface.ipv6}
                        <div class="detail-item"><span class="detail-label">IPv6</span><span class="detail-value mono">{iface.ipv6}</span></div>
                      {/if}
                      {#if iface.speed}
                        <div class="detail-item"><span class="detail-label">Speed</span><span class="detail-value">{iface.speed}</span></div>
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        </section>
      {/if}

      <!-- SENSORS -->
      {#if visibleSections.size === 0 || visibleSections.has("sensors")}
        <section class="panel-section">
          <h3 class="section-title"><span class="section-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" width="1em" height="1em"><path d="M14 14.76V3.5a2.5 2.5 0 0 0-5 0v11.26a4.5 4.5 0 1 0 5 0z"/></svg></span> Sensors</h3>
          {#if !sensorData || sensorData.length === 0}
            <div class="section-card">
              <p class="empty-msg">No sensor data available.</p>
            </div>
          {:else}
            <div class="section-card">
              {#each sensorData as group}
                <div class="sensor-group">
                  <h4 class="sensor-group-name">{group.name}</h4>
                  <div class="sensor-readings">
                    {#each group.readings as reading}
                      <div class="sensor-card">
                        <div class="sensor-label">{reading.label}</div>
                        <div class="sensor-value" style="color: {reading.unit === '°C' ? tempColor(reading.value, reading.high, reading.critical) : '#e2e8f0'}">
                          {reading.value.toFixed(reading.unit === "RPM" ? 0 : 1)}<span class="sensor-unit">{reading.unit}</span>
                        </div>
                        {#if reading.unit === "°C" && (reading.high || reading.critical)}
                          <div class="sensor-thresholds">
                            {#if reading.high}<span>High: {reading.high}°C</span>{/if}
                            {#if reading.critical}<span>Crit: {reading.critical}°C</span>{/if}
                          </div>
                        {/if}
                      </div>
                    {/each}
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </section>
      {/if}

      <!-- PROCESSES -->
      {#if visibleSections.size === 0 || visibleSections.has("processes")}
        <section class="panel-section">
          <h3 class="section-title"><span class="section-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" width="1em" height="1em"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg></span> Processes</h3>
          {#if stats && stats.top_processes.length > 0}
            <div class="section-card">
              <div class="proc-table">
                <div class="proc-header">
                  <span class="proc-col-pid">PID</span>
                  <span class="proc-col-name">Name</span>
                  <span class="proc-col-cpu">CPU</span>
                  <span class="proc-col-mem">Memory</span>
                  <span class="proc-col-state">State</span>
                </div>
                {#each stats.top_processes as proc}
                  <div class="proc-row">
                    <span class="proc-col-pid">{proc.pid}</span>
                    <span class="proc-col-name">{proc.name}</span>
                    <span class="proc-col-cpu" style="color: {usageColor(proc.cpu_percent)}">{proc.cpu_percent.toFixed(1)}%</span>
                    <span class="proc-col-mem">{formatBytes(proc.mem_bytes)}</span>
                    <span class="proc-col-state">{proc.state}</span>
                  </div>
                {/each}
              </div>
            </div>
          {:else}
            <div class="section-card">
              <p class="empty-msg">Waiting for process data...</p>
            </div>
          {/if}
        </section>
      {/if}

    </div>
  {/if}
</div>

<style>
  .hardware-info {
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

  .host-badge {
    font-size: 0.85rem;
    color: #64748b;
    background: #1e293b;
    padding: 2px 10px;
    border-radius: 12px;
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

  .header-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .rate-selector {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .rate-label {
    font-size: 0.75rem;
    color: #64748b;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .rate-options {
    display: flex;
    gap: 2px;
    background: #0f172a;
    padding: 3px;
    border-radius: 8px;
    border: 1px solid #1e293b;
  }

  .rate-btn {
    background: transparent;
    border: none;
    color: #64748b;
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 0.78rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
  }

  .rate-btn:hover {
    color: #94a3b8;
  }

  .rate-btn.active {
    background: #1e293b;
    color: #c4b5fd;
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

  .error-box {
    background: #ef444418;
    border: 1px solid #ef444440;
    border-radius: 8px;
    padding: 16px;
    color: #f87171;
    font-size: 0.9rem;
  }

  /* Filter chips */
  .filter-chips {
    display: flex;
    gap: 6px;
    margin-bottom: 20px;
    overflow-x: auto;
    padding-bottom: 4px;
    flex-wrap: wrap;
  }

  .chip {
    background: #1e293b;
    border: 1px solid #334155;
    color: #94a3b8;
    padding: 6px 14px;
    border-radius: 20px;
    cursor: pointer;
    font-size: 0.85rem;
    font-weight: 600;
    white-space: nowrap;
    display: flex;
    align-items: center;
    gap: 6px;
    transition: all 0.15s;
    user-select: none;
  }

  .chip:hover {
    background: #334155;
    color: #e2e8f0;
  }

  .chip.active {
    background: #7c3aed20;
    border-color: #7c3aed;
    color: #c4b5fd;
  }

  .chip-icon {
    font-size: 0.85rem;
    display: inline-flex;
    align-items: center;
  }

  /* Live dot */
  .live-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #22c55e;
    box-shadow: 0 0 6px #22c55e80;
    animation: pulse-dot 2s ease-in-out infinite;
    flex-shrink: 0;
    align-self: center;
  }

  @keyframes pulse-dot {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  /* Panel body */
  .panel-body {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .panel-section {
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
    display: inline-flex;
    align-items: center;
  }

  .section-card {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 12px;
    padding: 16px;
  }

  .section-split {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 10px;
  }

  .flex-1 {
    min-width: 0;
  }

  .spec-headline {
    font-size: 1rem;
    font-weight: 600;
    color: #f1f5f9;
    margin-bottom: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sub-title {
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #94a3b8;
    font-weight: 700;
    margin-bottom: 10px;
  }

  /* Gauge / sparkline */
  .gauge-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 10px;
  }

  .gauge-label {
    font-size: 0.82rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #94a3b8;
    font-weight: 700;
  }

  .gauge-value {
    font-size: 1.3rem;
    font-weight: 700;
  }

  .net-rates {
    display: flex;
    gap: 10px;
    font-size: 0.85rem;
  }

  .rx-rate { color: #22c55e; }
  .tx-rate { color: #f59e0b; }

  .sparkline {
    width: 100%;
    height: 50px;
    display: block;
  }

  .gauge-sub {
    display: flex;
    gap: 10px;
    font-size: 0.8rem;
    color: #94a3b8;
    margin-top: 6px;
  }

  /* Detail grid */
  .detail-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 16px;
  }

  .detail-grid.compact {
    gap: 10px;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  }

  .detail-item {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .detail-label {
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #94a3b8;
    font-weight: 700;
  }

  .detail-value {
    font-size: 1rem;
    color: #f1f5f9;
    font-weight: 500;
  }

  .detail-value.mono {
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 0.9rem;
  }

  .empty-msg {
    color: #64748b;
    font-size: 0.9rem;
    text-align: center;
    padding: 20px 0;
    margin: 0;
  }

  /* Memory bar */
  .mem-bar-wrapper {
    width: 100%;
    margin-bottom: 12px;
  }

  .mem-bar-track {
    width: 100%;
    height: 10px;
    background: #0f172a;
    border-radius: 5px;
    overflow: hidden;
    margin-bottom: 4px;
  }

  .mem-bar-fill {
    height: 100%;
    border-radius: 5px;
    transition: width 0.3s;
  }

  .mem-bar-labels {
    display: flex;
    justify-content: space-between;
    font-size: 0.85rem;
    color: #cbd5e1;
  }

  /* Core bars */
  .core-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 6px;
  }

  .core-bar-item {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .core-label {
    font-size: 0.8rem;
    color: #cbd5e1;
    width: 52px;
    flex-shrink: 0;
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  .core-bar-track {
    flex: 1;
    height: 8px;
    background: #0f172a;
    border-radius: 4px;
    overflow: hidden;
  }

  .core-bar-fill {
    height: 100%;
    border-radius: 4px;
    transition: width 0.4s, background 0.3s;
  }

  .core-pct {
    font-size: 0.8rem;
    color: #cbd5e1;
    width: 36px;
    text-align: right;
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  /* Swap */
  .swap-bar-wrapper {
    max-width: 500px;
  }

  .swap-bar-track {
    width: 100%;
    height: 10px;
    background: #0f172a;
    border-radius: 5px;
    overflow: hidden;
    margin-bottom: 4px;
  }

  .swap-bar-fill {
    height: 100%;
    background: #6366f1;
    border-radius: 5px;
    transition: width 0.4s;
  }

  .swap-labels {
    display: flex;
    justify-content: space-between;
    font-size: 0.82rem;
    color: #94a3b8;
  }

  /* GPU */
  .gpu-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 14px;
  }

  .gpu-index {
    font-size: 0.78rem;
    font-weight: 700;
    text-transform: uppercase;
    color: #7c3aed;
    background: #7c3aed18;
    padding: 2px 8px;
    border-radius: 6px;
  }

  .gpu-name {
    font-size: 1rem;
    font-weight: 600;
    color: #f1f5f9;
  }

  /* Disks */
  .disk-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
  }

  .disk-title {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .disk-icon {
    font-size: 1.1rem;
    display: inline-flex;
    align-items: center;
  }

  .disk-name {
    font-weight: 600;
    color: #f1f5f9;
    font-size: 1rem;
  }

  .type-badge {
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    padding: 2px 8px;
    border-radius: 6px;
    letter-spacing: 0.5px;
  }

  .type-badge.ssd {
    background: #7c3aed18;
    color: #a78bfa;
  }

  .type-badge.hdd {
    background: #f59e0b18;
    color: #fbbf24;
  }

  .disk-size {
    font-size: 0.9rem;
    color: #94a3b8;
    font-weight: 600;
  }

  .disk-model {
    font-size: 0.88rem;
    color: #94a3b8;
    margin-bottom: 12px;
  }

  .partition-list {
    border-top: 1px solid #334155;
    padding-top: 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .partition-row {
    display: grid;
    grid-template-columns: 1fr 80px 70px 1fr;
    gap: 8px;
    font-size: 0.85rem;
    align-items: center;
  }

  .part-name {
    color: #f1f5f9;
    font-weight: 500;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 0.82rem;
  }

  .part-size {
    color: #cbd5e1;
    text-align: right;
  }

  .part-fs {
    color: #7c3aed;
    font-weight: 600;
    text-transform: uppercase;
    font-size: 0.78rem;
    text-align: center;
  }

  .part-mount {
    color: #94a3b8;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 0.8rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Network */
  .state-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #475569;
    flex-shrink: 0;
  }

  .state-dot.up {
    background: #22c55e;
    box-shadow: 0 0 4px #22c55e80;
  }

  .net-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .net-card {
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 10px;
    padding: 14px;
  }

  .net-card.net-up {
    border-color: #22c55e30;
  }

  .net-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }

  .net-title {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .net-name {
    font-weight: 600;
    color: #f1f5f9;
    font-size: 1rem;
  }

  .net-type-badge {
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    padding: 2px 8px;
    border-radius: 6px;
    background: #6366f118;
    color: #818cf8;
    letter-spacing: 0.5px;
  }

  .net-state {
    font-size: 0.82rem;
    color: #94a3b8;
    text-transform: uppercase;
    font-weight: 600;
    letter-spacing: 0.5px;
  }

  /* Sensors */
  .sensor-group {
    margin-bottom: 20px;
  }

  .sensor-group:last-child {
    margin-bottom: 0;
  }

  .sensor-group-name {
    margin: 0 0 12px;
    font-size: 0.9rem;
    font-weight: 700;
    color: #cbd5e1;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .sensor-readings {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 10px;
  }

  .sensor-card {
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 10px;
    padding: 14px;
    text-align: center;
  }

  .sensor-label {
    font-size: 0.82rem;
    color: #94a3b8;
    margin-bottom: 6px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sensor-value {
    font-size: 1.5rem;
    font-weight: 700;
  }

  .sensor-unit {
    font-size: 0.75rem;
    font-weight: 500;
    opacity: 0.7;
    margin-left: 2px;
  }

  .sensor-thresholds {
    display: flex;
    gap: 10px;
    justify-content: center;
    margin-top: 6px;
    font-size: 0.75rem;
    color: #64748b;
  }

  /* Process table */
  .proc-table {
    overflow-x: auto;
  }

  .proc-header, .proc-row {
    display: grid;
    grid-template-columns: 60px 1fr 70px 80px 80px;
    gap: 8px;
    padding: 6px 8px;
    align-items: center;
  }

  .proc-header {
    border-bottom: 1px solid #334155;
    margin-bottom: 2px;
  }

  .proc-header span {
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #94a3b8;
    font-weight: 700;
  }

  .proc-row {
    border-radius: 4px;
    font-size: 0.88rem;
  }

  .proc-row:hover {
    background: #0f172a;
  }

  .proc-col-pid {
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 0.82rem;
    color: #94a3b8;
  }

  .proc-col-name {
    color: #f1f5f9;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .proc-col-cpu {
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 0.85rem;
    font-weight: 600;
    text-align: right;
  }

  .proc-col-mem {
    color: #cbd5e1;
    font-size: 0.85rem;
    text-align: right;
  }

  .proc-col-state {
    color: #94a3b8;
    font-size: 0.8rem;
    text-align: right;
  }
</style>
