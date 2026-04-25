<script>
  import { invoke } from "@tauri-apps/api/core";

  let { active = true } = $props();

  let snapshot = $state(null);
  let loading = $state(true);
  let bandwidth = $state({});
  let prevTraffic = $state(null);
  let prevTime = $state(null);
  let pollTimer = $state(null);
  let snapshotTimer = $state(null);
  let snapshotPending = false;
  let connFilter = $state("all");
  let connSearch = $state("");
  let hideLocal = $state(false);
  let activeTab = $state("overview");
  let lastUpdate = $state(null);

  // Sorting state
  let connSortCol = $state("remote_port");
  let connSortAsc = $state(true);
  let portSortCol = $state("port");
  let portSortAsc = $state(true);

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
  let snapshotRate = $state(3000);

  async function loadSnapshot() {
    if (snapshotPending) return;
    snapshotPending = true;
    loading = snapshot === null;
    try {
      const fresh = await invoke("get_network_snapshot");
      if (snapshot) {
        // Merge into existing object to avoid full DOM rebuild
        snapshot.interfaces = fresh.interfaces;
        snapshot.connections = fresh.connections;
        snapshot.listening = fresh.listening;
        snapshot.dns = fresh.dns;
        snapshot.traffic = fresh.traffic;
      } else {
        snapshot = fresh;
      }
      lastUpdate = Date.now();
    } finally {
      loading = false;
      snapshotPending = false;
    }
  }

  async function pollBandwidth() {
    try {
      const traffic = await invoke("get_traffic_snapshot");
      const now = Date.now();

      if (prevTraffic && prevTime) {
        const dt = (now - prevTime) / 1000;
        const rates = {};
        for (const iface of traffic) {
          const prev = prevTraffic.find((p) => p.name === iface.name);
          if (prev && dt > 0) {
            rates[iface.name] = {
              rx_rate: Math.max(0, (iface.rx_bytes - prev.rx_bytes) / dt),
              tx_rate: Math.max(0, (iface.tx_bytes - prev.tx_bytes) / dt),
              rx_pps: Math.max(0, (iface.rx_packets - prev.rx_packets) / dt),
              tx_pps: Math.max(0, (iface.tx_packets - prev.tx_packets) / dt),
            };
          }
        }
        bandwidth = rates;
      }

      prevTraffic = traffic;
      prevTime = now;
    } catch (_) {
      // silently retry next tick
    }
  }

  function startPolling() {
    pollBandwidth();
    loadSnapshot();
    pollTimer = setInterval(pollBandwidth, pollRate);
    snapshotTimer = setInterval(loadSnapshot, snapshotRate);
  }

  function restartPolling() {
    stopPolling();
    prevTraffic = null;
    prevTime = null;
    startPolling();
  }

  function setPollRate(ms) {
    pollRate = ms;
    snapshotRate = Math.max(ms * 2, 2000);
    restartPolling();
  }

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
    if (snapshotTimer) {
      clearInterval(snapshotTimer);
      snapshotTimer = null;
    }
  }

  // Only poll network state while this view is the active tab so background
  // tabs don't keep hammering ss / ip / resolvectl.
  $effect(() => {
    if (active) {
      startPolling();
      return () => stopPolling();
    }
  });

  let updateAgo = $state("");
  let agoTimer = $state(null);

  $effect(() => {
    if (!active) return;
    agoTimer = setInterval(() => {
      if (lastUpdate) {
        const sec = Math.round((Date.now() - lastUpdate) / 1000);
        updateAgo = sec < 2 ? "just now" : sec + "s ago";
      }
    }, 5000);
    return () => { if (agoTimer) clearInterval(agoTimer); };
  });

  function formatRate(bytesPerSec) {
    if (bytesPerSec < 1024) return bytesPerSec.toFixed(0) + " B/s";
    if (bytesPerSec < 1024 * 1024) return (bytesPerSec / 1024).toFixed(1) + " KB/s";
    if (bytesPerSec < 1024 * 1024 * 1024)
      return (bytesPerSec / (1024 * 1024)).toFixed(2) + " MB/s";
    return (bytesPerSec / (1024 * 1024 * 1024)).toFixed(2) + " GB/s";
  }

  function formatBytes(bytes) {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
  }

  function formatPps(pps) {
    if (pps < 1000) return Math.round(pps) + " pps";
    return (pps / 1000).toFixed(1) + "k pps";
  }

  function stateColor(state) {
    const s = state.toUpperCase();
    if (s === "ESTAB" || s === "ESTABLISHED") return "estab";
    if (s === "TIME-WAIT" || s === "TIME_WAIT") return "timewait";
    if (s === "CLOSE-WAIT" || s === "CLOSE_WAIT") return "closewait";
    if (s === "UNCONN") return "unconn";
    if (s === "SYN-SENT" || s === "SYN_SENT") return "synsent";
    return "other";
  }

  function isLocal(addr) {
    return addr === "127.0.0.1" || addr === "::1" || addr === "0.0.0.0" || addr === "::" || addr.startsWith("127.") || addr === "localhost";
  }

  function sortBy(list, col, asc) {
    return [...list].sort((a, b) => {
      let va = a[col], vb = b[col];
      if (typeof va === "string") va = va.toLowerCase();
      if (typeof vb === "string") vb = vb.toLowerCase();
      if (va < vb) return asc ? -1 : 1;
      if (va > vb) return asc ? 1 : -1;
      return 0;
    });
  }

  function toggleConnSort(col) {
    if (connSortCol === col) {
      connSortAsc = !connSortAsc;
    } else {
      connSortCol = col;
      connSortAsc = true;
    }
  }

  function togglePortSort(col) {
    if (portSortCol === col) {
      portSortAsc = !portSortAsc;
    } else {
      portSortCol = col;
      portSortAsc = true;
    }
  }

  function sortIndicator(activeCol, col, asc) {
    if (activeCol !== col) return " ↕";
    return asc ? " ↑" : " ↓";
  }

  let filteredConnections = $derived.by(() => {
    if (!snapshot) return [];
    let conns = snapshot.connections;
    if (connFilter !== "all") {
      conns = conns.filter((c) => c.protocol.toLowerCase() === connFilter);
    }
    if (hideLocal) {
      conns = conns.filter((c) => !isLocal(c.remote_addr) && !isLocal(c.local_addr));
    }
    if (connSearch.trim()) {
      const q = connSearch.toLowerCase();
      conns = conns.filter(
        (c) =>
          c.process.toLowerCase().includes(q) ||
          c.remote_addr.toLowerCase().includes(q) ||
          c.local_addr.toLowerCase().includes(q) ||
          c.local_port.toString().includes(q) ||
          c.remote_port.toString().includes(q)
      );
    }
    return sortBy(conns, connSortCol, connSortAsc);
  });

  let filteredPorts = $derived.by(() => {
    if (!snapshot) return [];
    let ports = snapshot.listening;
    if (hideLocal) {
      ports = ports.filter((p) => !isLocal(p.address));
    }
    return sortBy(ports, portSortCol, portSortAsc);
  });

  let connectionStats = $derived.by(() => {
    if (!snapshot) return { total: 0, tcp: 0, udp: 0, established: 0 };
    const conns = snapshot.connections;
    return {
      total: conns.length,
      tcp: conns.filter((c) => c.protocol.toLowerCase() === "tcp").length,
      udp: conns.filter((c) => c.protocol.toLowerCase() === "udp").length,
      established: conns.filter(
        (c) => c.state.toUpperCase() === "ESTAB" || c.state.toUpperCase() === "ESTABLISHED"
      ).length,
    };
  });

  let activeInterfaces = $derived(
    snapshot?.interfaces?.filter((i) => i.state === "up") ?? []
  );

  async function refreshAll() {
    await loadSnapshot();
  }
</script>

<div class="network">
  <div class="header">
    <div class="header-left">
      <h2>Network Monitor</h2>
      <span class="iface-count">{activeInterfaces.length} active interface{activeInterfaces.length !== 1 ? "s" : ""}</span>
      {#if snapshot}
        <span class="live-badge"><span class="live-dot"></span> LIVE</span>
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
      {#if updateAgo}
        <span class="last-update">Updated {updateAgo}</span>
      {/if}
      <button class="btn-refresh" onclick={refreshAll} disabled={loading}>
        <span class:spin={loading}>↻</span> Refresh
      </button>
    </div>
  </div>

  {#if loading && !snapshot}
    <div class="loading">
      <div class="spinner"></div>
      <span>Scanning network…</span>
    </div>
  {:else if snapshot}
    <!-- Tab nav -->
    <div class="tabs">
      <button class="tab" class:active={activeTab === "overview"} onclick={() => activeTab = "overview"}>
        Overview
      </button>
      <button class="tab" class:active={activeTab === "connections"} onclick={() => activeTab = "connections"}>
        Connections <span class="tab-count">{snapshot.connections.length}</span>
      </button>
      <button class="tab" class:active={activeTab === "ports"} onclick={() => activeTab = "ports"}>
        Listening Ports <span class="tab-count">{snapshot.listening.length}</span>
      </button>
    </div>

    {#if activeTab === "overview"}
      <!-- ═══ BANDWIDTH ═══ -->
      <div class="section">
        <h3 class="section-title"><span class="section-icon"><svg style="width:1em;height:1em;vertical-align:middle" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M2 20h.01M7 20v-4M12 20v-8M17 20v-12M22 20V8"/></svg></span> Real-Time Bandwidth</h3>
        <div class="bandwidth-grid">
          {#each snapshot.interfaces.filter((i) => i.state === "up") as iface}
            {@const rates = bandwidth[iface.name]}
            <div class="bandwidth-card">
              <div class="bw-header">
                <span class="bw-name">{iface.name}</span>
                <span class="bw-type">{iface.interface_type}</span>
              </div>
              <div class="bw-rates">
                <div class="bw-rate down">
                  <span class="bw-arrow">↓</span>
                  <span class="bw-value">{rates ? formatRate(rates.rx_rate) : "—"}</span>
                  <span class="bw-label">Download</span>
                </div>
                <div class="bw-rate up">
                  <span class="bw-arrow">↑</span>
                  <span class="bw-value">{rates ? formatRate(rates.tx_rate) : "—"}</span>
                  <span class="bw-label">Upload</span>
                </div>
              </div>
              {#if rates}
                <div class="bw-pps">
                  <span>↓ {formatPps(rates.rx_pps)}</span>
                  <span>↑ {formatPps(rates.tx_pps)}</span>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      </div>

      <!-- ═══ INTERFACES ═══ -->
      <div class="section">
        <h3 class="section-title"><span class="section-icon"><svg style="width:1em;height:1em;vertical-align:middle" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="9" width="6" height="6" rx="1"/><rect x="16" y="9" width="6" height="6" rx="1"/><rect x="9" y="2" width="6" height="6" rx="1"/><rect x="9" y="16" width="6" height="6" rx="1"/><path d="M5 15v2a2 2 0 0 0 2 2h2M5 9V7a2 2 0 0 1 2-2h2M19 15v2a2 2 0 0 1-2 2h-2M19 9V7a2 2 0 0 0-2-2h-2"/></svg></span> Interfaces</h3>
        <div class="iface-grid">
          {#each snapshot.interfaces as iface}
            <div class="iface-card" class:up={iface.state === "up"}>
              <div class="iface-header">
                <span class="iface-name">{iface.name}</span>
                <span class="iface-state" class:up={iface.state === "up"} class:down={iface.state !== "up"}>
                  {iface.state}
                </span>
              </div>
              <div class="iface-type">{iface.interface_type} · {iface.speed}</div>
              <div class="iface-details">
                <div class="iface-row">
                  <span class="iface-label">IPv4</span>
                  <span class="iface-val mono">{iface.ipv4 || "—"}</span>
                </div>
                {#if iface.ipv6}
                  <div class="iface-row">
                    <span class="iface-label">IPv6</span>
                    <span class="iface-val mono truncate">{iface.ipv6}</span>
                  </div>
                {/if}
                <div class="iface-row">
                  <span class="iface-label">MAC</span>
                  <span class="iface-val mono">{iface.mac}</span>
                </div>
                <div class="iface-row">
                  <span class="iface-label">MTU</span>
                  <span class="iface-val">{iface.mtu}</span>
                </div>
              </div>
              <div class="iface-traffic">
                <div class="traffic-item">
                  <span class="traffic-label">RX</span>
                  <span class="traffic-val">{formatBytes(iface.rx_bytes)}</span>
                  <span class="traffic-secondary">{iface.rx_packets.toLocaleString()} pkts</span>
                </div>
                <div class="traffic-item">
                  <span class="traffic-label">TX</span>
                  <span class="traffic-val">{formatBytes(iface.tx_bytes)}</span>
                  <span class="traffic-secondary">{iface.tx_packets.toLocaleString()} pkts</span>
                </div>
              </div>
              {#if iface.rx_errors > 0 || iface.tx_errors > 0 || iface.rx_dropped > 0 || iface.tx_dropped > 0}
                <div class="iface-errors">
                  <span><svg style="width:1em;height:1em;vertical-align:middle;display:inline-block" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg> Errors: RX {iface.rx_errors} / TX {iface.tx_errors}</span>
                  <span>Dropped: RX {iface.rx_dropped} / TX {iface.tx_dropped}</span>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      </div>

      <!-- ═══ DNS / ROUTING ═══ -->
      <div class="section">
        <h3 class="section-title"><span class="section-icon"><svg style="width:1em;height:1em;vertical-align:middle" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg></span> DNS & Routing</h3>
        <div class="dns-grid">
          <div class="dns-card">
            <div class="dns-label">Gateway</div>
            <div class="dns-val mono">{snapshot.dns.gateway || "—"}</div>
          </div>
          <div class="dns-card">
            <div class="dns-label">Resolver</div>
            <div class="dns-val">{snapshot.dns.resolver || "—"}</div>
          </div>
          <div class="dns-card span-2">
            <div class="dns-label">DNS Servers</div>
            <div class="dns-val mono">
              {#if snapshot.dns.servers.length > 0}
                {snapshot.dns.servers.join(" · ")}
              {:else}
                —
              {/if}
            </div>
          </div>
          {#if snapshot.dns.search_domains.length > 0}
            <div class="dns-card span-2">
              <div class="dns-label">Search Domains</div>
              <div class="dns-val mono">{snapshot.dns.search_domains.join(", ")}</div>
            </div>
          {/if}
        </div>
      </div>

      <!-- ═══ QUICK STATS ═══ -->
      <div class="section">
        <h3 class="section-title"><span class="section-icon"><svg style="width:1em;height:1em;vertical-align:middle" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="20" x2="18" y2="10"/><line x1="12" y1="20" x2="12" y2="4"/><line x1="6" y1="20" x2="6" y2="14"/></svg></span> Connection Summary</h3>
        <div class="stats-grid">
          <div class="stat-card">
            <div class="stat-val">{connectionStats.total}</div>
            <div class="stat-label">Total Connections</div>
          </div>
          <div class="stat-card">
            <div class="stat-val">{connectionStats.established}</div>
            <div class="stat-label">Established</div>
          </div>
          <div class="stat-card">
            <div class="stat-val">{connectionStats.tcp}</div>
            <div class="stat-label">TCP</div>
          </div>
          <div class="stat-card">
            <div class="stat-val">{connectionStats.udp}</div>
            <div class="stat-label">UDP</div>
          </div>
          <div class="stat-card">
            <div class="stat-val">{snapshot.listening.length}</div>
            <div class="stat-label">Listening Ports</div>
          </div>
          <div class="stat-card">
            <div class="stat-val">{snapshot.interfaces.length}</div>
            <div class="stat-label">Interfaces</div>
          </div>
        </div>
      </div>

    {:else if activeTab === "connections"}
      <!-- ═══ CONNECTIONS TABLE ═══ -->
      <div class="section">
        <div class="conn-toolbar">
          <div class="conn-filters">
            <button class="filter-btn" class:active={connFilter === "all"} onclick={() => connFilter = "all"}>All</button>
            <button class="filter-btn" class:active={connFilter === "tcp"} onclick={() => connFilter = "tcp"}>TCP</button>
            <button class="filter-btn" class:active={connFilter === "udp"} onclick={() => connFilter = "udp"}>UDP</button>
          </div>
          <label class="toggle-label">
            <input type="checkbox" bind:checked={hideLocal} class="toggle-check" />
            <span>Hide local</span>
          </label>
          <label class="search-wrapper">
            <span class="sr-only">Search connections</span>
            <input
              type="text"
              class="conn-search"
              placeholder="Filter by process, address, port…"
              bind:value={connSearch}
            />
          </label>
        </div>

        {#if filteredConnections.length === 0}
          <div class="empty-state">No matching connections</div>
        {:else}
          <div class="table-wrapper">
            <table class="conn-table">
              <thead>
                <tr>
                  <th class="sortable" onclick={() => toggleConnSort("protocol")}>Proto{sortIndicator(connSortCol, "protocol", connSortAsc)}</th>
                  <th class="sortable" onclick={() => toggleConnSort("state")}>State{sortIndicator(connSortCol, "state", connSortAsc)}</th>
                  <th class="sortable" onclick={() => toggleConnSort("local_port")}>Local{sortIndicator(connSortCol, "local_port", connSortAsc)}</th>
                  <th class="sortable" onclick={() => toggleConnSort("remote_addr")}>Remote{sortIndicator(connSortCol, "remote_addr", connSortAsc)}</th>
                  <th class="sortable" onclick={() => toggleConnSort("process")}>Process{sortIndicator(connSortCol, "process", connSortAsc)}</th>
                </tr>
              </thead>
              <tbody>
                {#each filteredConnections as conn}
                  <tr>
                    <td><span class="proto-badge">{conn.protocol.toUpperCase()}</span></td>
                    <td><span class="state-badge {stateColor(conn.state)}">{conn.state}</span></td>
                    <td class="mono">{conn.local_addr}:{conn.local_port}</td>
                    <td class="mono">{conn.remote_addr}:{conn.remote_port}</td>
                    <td>
                      {#if conn.process}
                        <span class="process-name">{conn.process}</span>
                        <span class="pid">({conn.pid})</span>
                      {:else}
                        <span class="no-proc">—</span>
                      {/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
          <div class="table-footer">{filteredConnections.length} connection{filteredConnections.length !== 1 ? "s" : ""}</div>
        {/if}
      </div>

    {:else if activeTab === "ports"}
      <!-- ═══ LISTENING PORTS ═══ -->
      <div class="section">
        <div class="conn-toolbar">
          <label class="toggle-label">
            <input type="checkbox" bind:checked={hideLocal} class="toggle-check" />
            <span>Hide local</span>
          </label>
        </div>
        {#if filteredPorts.length === 0}
          <div class="empty-state">No listening ports detected</div>
        {:else}
          <div class="table-wrapper">
            <table class="conn-table">
              <thead>
                <tr>
                  <th class="sortable" onclick={() => togglePortSort("port")}>Port{sortIndicator(portSortCol, "port", portSortAsc)}</th>
                  <th class="sortable" onclick={() => togglePortSort("protocol")}>Proto{sortIndicator(portSortCol, "protocol", portSortAsc)}</th>
                  <th class="sortable" onclick={() => togglePortSort("address")}>Address{sortIndicator(portSortCol, "address", portSortAsc)}</th>
                  <th class="sortable" onclick={() => togglePortSort("process")}>Process{sortIndicator(portSortCol, "process", portSortAsc)}</th>
                </tr>
              </thead>
              <tbody>
                {#each filteredPorts as p}
                  <tr>
                    <td><span class="port-num">{p.port}</span></td>
                    <td><span class="proto-badge">{p.protocol.toUpperCase()}</span></td>
                    <td class="mono">{p.address}</td>
                    <td>
                      {#if p.process}
                        <span class="process-name">{p.process}</span>
                        <span class="pid">({p.pid})</span>
                      {:else}
                        <span class="no-proc">—</span>
                      {/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
          <div class="table-footer">{filteredPorts.length} listening port{filteredPorts.length !== 1 ? "s" : ""}</div>
        {/if}
      </div>
    {/if}
  {/if}
</div>

<style>
  .network {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .header-left h2 {
    margin: 0;
    font-size: 1.5rem;
    color: #f1f5f9;
  }

  .iface-count {
    background: #0891b218;
    color: #22d3ee;
    font-size: 0.78rem;
    font-weight: 700;
    padding: 3px 10px;
    border-radius: 20px;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .btn-refresh {
    background: #0891b2;
    border: none;
    color: #fff;
    padding: 8px 18px;
    border-radius: 8px;
    font-size: 0.88rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .btn-refresh:hover {
    background: #06b6d4;
  }

  .btn-refresh:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .last-update {
    font-size: 0.78rem;
    color: #475569;
    min-width: 7.5em;
    text-align: right;
  }

  .live-badge {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.7rem;
    font-weight: 700;
    color: #4ade80;
    letter-spacing: 0.5px;
    text-transform: uppercase;
  }

  .live-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #4ade80;
    animation: pulse-dot 1.5s ease-in-out infinite;
  }

  @keyframes pulse-dot {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
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
    color: #22d3ee;
  }

  /* ── Tabs ── */

  .tabs {
    display: flex;
    gap: 2px;
    background: #0f172a;
    padding: 4px;
    border-radius: 12px;
    border: 1px solid #334155;
    box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.3);
  }

  .tab {
    flex: 1;
    background: transparent;
    border: 1px solid transparent;
    color: #94a3b8;
    padding: 11px 20px;
    border-radius: 9px;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    letter-spacing: -0.01em;
  }

  .tab:hover {
    color: #e2e8f0;
    background: rgba(255, 255, 255, 0.05);
  }

  .tab.active {
    background: linear-gradient(180deg, #1e293b 0%, #1a2436 100%);
    color: #22d3ee;
    border-color: #334155;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25), inset 0 1px 0 rgba(255, 255, 255, 0.06);
  }

  .tab-count {
    background: #1e293b;
    padding: 2px 8px;
    border-radius: 10px;
    font-size: 0.75rem;
    border: 1px solid #334155;
  }

  .tab.active .tab-count {
    background: rgba(34, 211, 238, 0.15);
    border-color: rgba(34, 211, 238, 0.25);
    color: #22d3ee;
  }

  /* ── Sections ── */

  .section {
    display: flex;
    flex-direction: column;
    gap: 12px;
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

  /* ── Bandwidth Cards ── */

  .bandwidth-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 12px;
  }

  .bandwidth-card {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 12px;
    padding: 16px;
    transition: border-color 0.15s;
  }

  .bandwidth-card:hover {
    border-color: #0891b240;
  }

  .bw-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 14px;
  }

  .bw-name {
    font-weight: 700;
    font-size: 1rem;
    color: #f1f5f9;
  }

  .bw-type {
    font-size: 0.75rem;
    color: #64748b;
    background: #0f172a;
    padding: 2px 8px;
    border-radius: 4px;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    font-weight: 600;
  }

  .bw-rates {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .bw-rate {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 10px 0;
    border-radius: 8px;
    background: #0f172a;
  }

  .bw-arrow {
    font-size: 1.1rem;
    font-weight: 700;
  }

  .bw-rate.down .bw-arrow {
    color: #22d3ee;
  }

  .bw-rate.up .bw-arrow {
    color: #a78bfa;
  }

  .bw-value {
    font-size: 1.15rem;
    font-weight: 700;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    color: #f1f5f9;
  }

  .bw-label {
    font-size: 0.72rem;
    color: #64748b;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .bw-pps {
    display: flex;
    justify-content: space-around;
    margin-top: 10px;
    font-size: 0.78rem;
    color: #64748b;
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  /* ── Interface Cards ── */

  .iface-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 12px;
  }

  .iface-card {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 12px;
    padding: 16px;
    transition: border-color 0.15s;
  }

  .iface-card.up {
    border-color: #0891b230;
  }

  .iface-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 4px;
  }

  .iface-name {
    font-weight: 700;
    font-size: 1.05rem;
    color: #f1f5f9;
  }

  .iface-state {
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: uppercase;
    padding: 2px 8px;
    border-radius: 6px;
    letter-spacing: 0.5px;
  }

  .iface-state.up {
    background: #22d3ee18;
    color: #22d3ee;
  }

  .iface-state.down {
    background: #64748b18;
    color: #64748b;
  }

  .iface-type {
    font-size: 0.82rem;
    color: #64748b;
    margin-bottom: 12px;
  }

  .iface-details {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 12px;
  }

  .iface-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .iface-label {
    font-size: 0.78rem;
    color: #64748b;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    min-width: 36px;
  }

  .iface-val {
    font-size: 0.88rem;
    color: #cbd5e1;
  }

  .iface-traffic {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    padding-top: 10px;
    border-top: 1px solid #334155;
  }

  .traffic-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .traffic-label {
    font-size: 0.72rem;
    color: #64748b;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .traffic-val {
    font-size: 0.95rem;
    font-weight: 600;
    color: #f1f5f9;
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  .traffic-secondary {
    font-size: 0.75rem;
    color: #475569;
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  .iface-errors {
    margin-top: 8px;
    padding: 8px 10px;
    background: #ef444412;
    border-radius: 6px;
    font-size: 0.78rem;
    color: #f87171;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  /* ── DNS Grid ── */

  .dns-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .dns-card {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 10px;
    padding: 14px;
  }

  .dns-card.span-2 {
    grid-column: span 2;
  }

  .dns-label {
    font-size: 0.78rem;
    color: #64748b;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    margin-bottom: 4px;
  }

  .dns-val {
    font-size: 0.95rem;
    color: #f1f5f9;
    font-weight: 600;
  }

  /* ── Stats Grid ── */

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 10px;
  }

  .stat-card {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 10px;
    padding: 16px;
    text-align: center;
  }

  .stat-val {
    font-size: 1.5rem;
    font-weight: 700;
    color: #22d3ee;
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  .stat-label {
    font-size: 0.78rem;
    color: #64748b;
    margin-top: 4px;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    font-weight: 600;
  }

  /* ── Connection Toolbar ── */

  .conn-toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .conn-filters {
    display: flex;
    gap: 4px;
    background: #0f172a;
    padding: 3px;
    border-radius: 8px;
    border: 1px solid #1e293b;
  }

  .filter-btn {
    background: transparent;
    border: none;
    color: #64748b;
    padding: 6px 14px;
    border-radius: 6px;
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
  }

  .filter-btn:hover {
    color: #94a3b8;
  }

  .filter-btn.active {
    background: #1e293b;
    color: #22d3ee;
  }

  .toggle-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.82rem;
    color: #94a3b8;
    cursor: pointer;
    user-select: none;
    white-space: nowrap;
  }

  .toggle-check {
    accent-color: #0891b2;
    width: 15px;
    height: 15px;
    cursor: pointer;
  }

  .search-wrapper {
    flex: 1;
    min-width: 200px;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    border: 0;
  }

  .conn-search {
    width: 100%;
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 8px;
    padding: 8px 14px;
    color: #f1f5f9;
    font-size: 0.88rem;
    outline: none;
    transition: border-color 0.15s;
    box-sizing: border-box;
  }

  .conn-search:focus {
    border-color: #0891b2;
  }

  .conn-search::placeholder {
    color: #475569;
  }

  /* ── Table ── */

  .table-wrapper {
    overflow-x: auto;
    border-radius: 10px;
    border: 1px solid #334155;
  }

  .conn-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  .conn-table th {
    background: #0f172a;
    color: #64748b;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    font-size: 0.75rem;
    padding: 10px 14px;
    text-align: left;
    border-bottom: 1px solid #1e293b;
    position: sticky;
    top: 0;
    white-space: nowrap;
  }

  .conn-table th.sortable {
    cursor: pointer;
    user-select: none;
    transition: color 0.15s;
  }

  .conn-table th.sortable:hover {
    color: #22d3ee;
  }

  .conn-table td {
    padding: 8px 14px;
    border-bottom: 1px solid #1e293b;
    color: #cbd5e1;
  }

  .conn-table tbody tr:hover {
    background: #1e293b60;
  }

  .conn-table tbody tr:last-child td {
    border-bottom: none;
  }

  .mono {
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 0.82rem;
  }

  .truncate {
    max-width: 200px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .proto-badge {
    background: #334155;
    color: #94a3b8;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 700;
    letter-spacing: 0.3px;
  }

  .state-badge {
    font-size: 0.72rem;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: 6px;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .state-badge.estab {
    background: #22c55e18;
    color: #4ade80;
  }

  .state-badge.timewait {
    background: #f59e0b18;
    color: #fbbf24;
  }

  .state-badge.closewait {
    background: #ef444418;
    color: #f87171;
  }

  .state-badge.unconn {
    background: #64748b18;
    color: #94a3b8;
  }

  .state-badge.synsent {
    background: #3b82f618;
    color: #60a5fa;
  }

  .state-badge.other {
    background: #64748b18;
    color: #94a3b8;
  }

  .port-num {
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-weight: 700;
    color: #22d3ee;
    font-size: 0.95rem;
  }

  .process-name {
    font-weight: 600;
    color: #e2e8f0;
  }

  .pid {
    font-size: 0.78rem;
    color: #64748b;
    margin-left: 4px;
  }

  .no-proc {
    color: #475569;
  }

  .table-footer {
    font-size: 0.82rem;
    color: #64748b;
    text-align: right;
    padding: 4px 0;
  }

  /* ── Misc ── */

  .spin {
    display: inline-block;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
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
    border-top-color: #0891b2;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .empty-state {
    text-align: center;
    padding: 40px 0;
    color: #64748b;
    font-size: 0.95rem;
  }
</style>
