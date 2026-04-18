<script>
  import { invoke } from "@tauri-apps/api/core";

  let diskOverview = $state(null);
  let spaceAnalysis = $state(null);
  let scanning = $state(false);
  let loading = $state(true);
  let expandedEntries = $state(new Set());
  let loadedChildren = $state({});
  let loadingChildren = $state(new Set());
  let analyzedPartition = $state(null);
  /** Cache: path → SpaceAnalysis result */
  let scanCache = $state({});

  function formatBytes(bytes) {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
  }

  function getUsageColor(pct) {
    if (pct > 90) return "#ef4444";
    if (pct > 75) return "#f59e0b";
    if (pct > 50) return "#7c3aed";
    return "#22c55e";
  }

  const svgAttrs = 'viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:1em;height:1em;vertical-align:middle;display:inline-block"';
  const svgFolder = `<svg ${svgAttrs}><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>`;
  const svgImage = `<svg ${svgAttrs}><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>`;
  const svgVideo = `<svg ${svgAttrs}><rect x="2" y="2" width="20" height="20" rx="2.18"/><line x1="7" y1="2" x2="7" y2="22"/><line x1="17" y1="2" x2="17" y2="22"/><line x1="2" y1="12" x2="22" y2="12"/><line x1="2" y1="7" x2="7" y2="7"/><line x1="2" y1="17" x2="7" y2="17"/><line x1="17" y1="17" x2="22" y2="17"/><line x1="17" y1="7" x2="22" y2="7"/></svg>`;
  const svgAudio = `<svg ${svgAttrs}><path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>`;
  const svgArchive = `<svg ${svgAttrs}><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><polyline points="3.27 6.96 12 12.01 20.73 6.96"/><line x1="12" y1="22.08" x2="12" y2="12"/></svg>`;
  const svgDisc = `<svg ${svgAttrs}><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="3"/></svg>`;
  const svgFile = `<svg ${svgAttrs}><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>`;

  function getEntryIcon(entry) {
    if (entry.is_dir) return svgFolder;
    const ext = entry.name.split(".").pop()?.toLowerCase();
    const icons = {
      jpg: svgImage, jpeg: svgImage, png: svgImage, gif: svgImage, webp: svgImage, svg: svgImage,
      mp4: svgVideo, mkv: svgVideo, avi: svgVideo, mov: svgVideo, webm: svgVideo,
      mp3: svgAudio, flac: svgAudio, wav: svgAudio, ogg: svgAudio, m4a: svgAudio,
      zip: svgArchive, tar: svgArchive, gz: svgArchive, xz: svgArchive, bz2: svgArchive, rar: svgArchive, "7z": svgArchive,
      iso: svgDisc, img: svgDisc,
    };
    return icons[ext] || svgFile;
  }

  const barColors = [
    "#7c3aed", "#6366f1", "#8b5cf6", "#a78bfa",
    "#06b6d4", "#14b8a6", "#22c55e", "#84cc16",
    "#eab308", "#f59e0b", "#f97316", "#ef4444",
  ];

  function getBarColor(index) {
    return barColors[index % barColors.length];
  }

  async function loadOverview() {
    loading = true;
    try {
      diskOverview = await invoke("get_disk_overview");
    } finally {
      loading = false;
    }
  }

  async function analyzePartition(mountPoint) {
    analyzedPartition = mountPoint;
    expandedEntries = new Set();

    // Use cache if available — instant
    if (scanCache[mountPoint]) {
      spaceAnalysis = scanCache[mountPoint];
      return;
    }

    scanning = true;
    try {
      const result = await invoke("analyze_space", { path: mountPoint });
      scanCache = { ...scanCache, [mountPoint]: result };
      spaceAnalysis = result;
    } finally {
      scanning = false;
    }
  }

  async function toggleExpand(path) {
    if (expandedEntries.has(path)) {
      const next = new Set(expandedEntries);
      next.delete(path);
      expandedEntries = next;
      return;
    }

    // Use cache if available
    if (!loadedChildren[path]) {
      if (scanCache[path]) {
        loadedChildren = { ...loadedChildren, [path]: scanCache[path].entries };
      } else {
        const nextLoading = new Set(loadingChildren);
        nextLoading.add(path);
        loadingChildren = nextLoading;

        try {
          const result = await invoke("analyze_space", { path });
          scanCache = { ...scanCache, [path]: result };
          loadedChildren = { ...loadedChildren, [path]: result.entries };
        } finally {
          const doneLoading = new Set(loadingChildren);
          doneLoading.delete(path);
          loadingChildren = doneLoading;
        }
      }
    }

    const next = new Set(expandedEntries);
    next.add(path);
    expandedEntries = next;
  }

  $effect(() => {
    loadOverview();
  });

  let maxEntrySize = $derived(
    spaceAnalysis?.entries?.length > 0
      ? Math.max(...spaceAnalysis.entries.map((e) => e.size))
      : 1
  );
</script>

{#if loading}
  <div class="empty-state">
    <div class="spinner"></div>
    <h3>Loading disk information...</h3>
  </div>
{:else if diskOverview}
  <header class="content-header">
    <div>
      <h2>Disk Usage</h2>
      <p class="subtitle">Storage overview — click a partition to analyze space usage</p>
    </div>
    <div class="header-actions">
      {#if spaceAnalysis}
        <button class="btn btn-secondary" onclick={() => { spaceAnalysis = null; analyzedPartition = null; }}>
          ← Partitions
        </button>
      {/if}
      <button class="btn btn-secondary" onclick={loadOverview}>↻ Refresh</button>
    </div>
  </header>

  <!-- Partition cards -->
  <div class="partitions">
    {#each diskOverview.partitions as part}
      <div
        class="partition-card"
        class:active={analyzedPartition === part.mount_point}
        role="button"
        tabindex="0"
        onclick={() => analyzePartition(part.mount_point)}
        onkeydown={(e) => {
          if (e.key === "Enter" || e.key === " ") { e.preventDefault(); analyzePartition(part.mount_point); }
        }}
      >
        <div class="part-header">
          <div class="part-title">
            <span class="part-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="width:1em;height:1em;vertical-align:middle;display:inline-block"><ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/></svg></span>
            <div>
              <span class="part-mount">{part.mount_point}</span>
              <span class="part-device">{part.device} · {part.fs_type}</span>
            </div>
          </div>
          <span class="part-percent" style="color: {getUsageColor(part.usage_percent)}">
            {part.usage_percent}%
          </span>
        </div>

        <div class="part-bar-bg">
          <div
            class="part-bar-fill"
            style="width: {part.usage_percent}%; background: {getUsageColor(part.usage_percent)}"
          ></div>
        </div>

        <div class="part-stats">
          <span>{formatBytes(part.used_bytes)} used</span>
          <span>{formatBytes(part.available_bytes)} free</span>
          <span>{formatBytes(part.total_bytes)} total</span>
        </div>
      </div>
    {/each}
  </div>

  <!-- Space Analysis -->
  {#if scanning}
    <div class="empty-state">
      <div class="spinner"></div>
      <h3>Analyzing space usage...</h3>
      <p>Scanning {analyzedPartition} — this may take a moment</p>
    </div>
  {:else if spaceAnalysis}
    <div class="analysis-section">
      <div class="analysis-header">
        <h3>Largest in <span class="analysis-path">{spaceAnalysis.root_path}</span></h3>
        <span class="analysis-total">{formatBytes(spaceAnalysis.total_size)} scanned</span>
      </div>

      <!-- Stacked overview bar -->
      <div class="stacked-bar">
        {#each spaceAnalysis.entries.slice(0, 10) as entry, i}
          {@const pct = (entry.size / spaceAnalysis.total_size) * 100}
          {#if pct >= 0.5}
            <div
              class="stacked-segment"
              style="width: {pct}%; background: {getBarColor(i)}"
              title="{entry.name}: {formatBytes(entry.size)} ({pct.toFixed(1)}%)"
            ></div>
          {/if}
        {/each}
      </div>

      <!-- Legend under stacked bar -->
      <div class="stacked-legend">
        {#each spaceAnalysis.entries.slice(0, 10) as entry, i}
          <span class="legend-item">
            <span class="legend-dot" style="background: {getBarColor(i)}"></span>
            <span class="legend-label">{entry.name}</span>
          </span>
        {/each}
      </div>

      <!-- Entry list -->
      <div class="space-entries">
        {#each spaceAnalysis.entries as entry, i}
          {@const pct = (entry.size / maxEntrySize) * 100}
          <div class="space-entry-group">
            <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
            <div
              class="space-entry"
              class:expandable={entry.is_dir}
              class:expanded={expandedEntries.has(entry.path)}
              role={entry.is_dir ? "button" : undefined}
              tabindex={entry.is_dir ? 0 : -1}
              onclick={() => entry.is_dir && toggleExpand(entry.path)}
              onkeydown={(e) => {
                if (entry.is_dir && (e.key === "Enter" || e.key === " ")) {
                  e.preventDefault();
                  toggleExpand(entry.path);
                }
              }}
            >
              <span class="entry-rank" style="color: {getBarColor(i)}">{i + 1}</span>
              <span class="entry-icon">{@html getEntryIcon(entry)}</span>
              <div class="entry-info">
                <span class="entry-name">{entry.name}</span>
                <span class="entry-path">{entry.path}</span>
              </div>
              <div class="entry-bar-container">
                <div
                  class="entry-bar"
                  style="width: {Math.max(pct, 1)}%; background: {getBarColor(i)}"
                ></div>
              </div>
              <span class="entry-size">{formatBytes(entry.size)}</span>
              {#if entry.is_dir}
                {#if loadingChildren.has(entry.path)}
                  <span class="entry-spinner"></span>
                {:else}
                  <span class="entry-chevron" class:rotated={expandedEntries.has(entry.path)}>▾</span>
                {/if}
              {:else}
                <span class="entry-chevron-placeholder"></span>
              {/if}
            </div>

            {#if expandedEntries.has(entry.path) && loadedChildren[entry.path]}
              {@const children = loadedChildren[entry.path]}
              {@const maxChildSize = children.length > 0 ? Math.max(...children.map(c => c.size)) : 1}
              <div class="children-list">
                {#each children as child}
                  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                  <div
                    class="child-entry"
                    class:child-expandable={child.is_dir}
                    class:child-expanded={expandedEntries.has(child.path)}
                    role={child.is_dir ? "button" : undefined}
                    tabindex={child.is_dir ? 0 : -1}
                    onclick={() => child.is_dir && toggleExpand(child.path)}
                    onkeydown={(e) => {
                      if (child.is_dir && (e.key === "Enter" || e.key === " ")) {
                        e.preventDefault();
                        toggleExpand(child.path);
                      }
                    }}
                  >
                    <span class="child-icon">{@html getEntryIcon(child)}</span>
                    <span class="child-name">{child.name}</span>
                    <div class="child-bar-container">
                      <div
                        class="child-bar"
                        style="width: {Math.max((child.size / maxChildSize) * 100, 1)}%"
                      ></div>
                    </div>
                    <span class="child-size">{formatBytes(child.size)}</span>
                    {#if child.is_dir}
                      {#if loadingChildren.has(child.path)}
                        <span class="child-spinner"></span>
                      {:else}
                        <span class="child-chevron" class:rotated={expandedEntries.has(child.path)}>▾</span>
                      {/if}
                    {/if}
                  </div>

                  {#if expandedEntries.has(child.path) && loadedChildren[child.path]}
                    {@const grandchildren = loadedChildren[child.path]}
                    {@const maxGrandSize = grandchildren.length > 0 ? Math.max(...grandchildren.map(g => g.size)) : 1}
                    <div class="grandchildren-list">
                      {#each grandchildren as gc}
                        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                        <div
                          class="child-entry grandchild"
                          class:child-expandable={gc.is_dir}
                          class:child-expanded={expandedEntries.has(gc.path)}
                          role={gc.is_dir ? "button" : undefined}
                          tabindex={gc.is_dir ? 0 : -1}
                          onclick={() => gc.is_dir && toggleExpand(gc.path)}
                          onkeydown={(e) => {
                            if (gc.is_dir && (e.key === "Enter" || e.key === " ")) {
                              e.preventDefault();
                              toggleExpand(gc.path);
                            }
                          }}
                        >
                          <span class="child-icon">{@html getEntryIcon(gc)}</span>
                          <span class="child-name">{gc.name}</span>
                          <div class="child-bar-container">
                            <div
                              class="child-bar"
                              style="width: {Math.max((gc.size / maxGrandSize) * 100, 1)}%"
                            ></div>
                          </div>
                          <span class="child-size">{formatBytes(gc.size)}</span>
                          {#if gc.is_dir}
                            {#if loadingChildren.has(gc.path)}
                              <span class="child-spinner"></span>
                            {:else}
                              <span class="child-chevron" class:rotated={expandedEntries.has(gc.path)}>▾</span>
                            {/if}
                          {/if}
                        </div>

                        {#if expandedEntries.has(gc.path) && loadedChildren[gc.path]}
                          {@const ggchildren = loadedChildren[gc.path]}
                          {@const maxGGSize = ggchildren.length > 0 ? Math.max(...ggchildren.map(g => g.size)) : 1}
                          <div class="grandchildren-list deeper">
                            {#each ggchildren as gg}
                              <div class="child-entry grandchild">
                                <span class="child-icon">{@html getEntryIcon(gg)}</span>
                                <span class="child-name">{gg.name}</span>
                                <div class="child-bar-container">
                                  <div
                                    class="child-bar"
                                    style="width: {Math.max((gg.size / maxGGSize) * 100, 1)}%"
                                  ></div>
                                </div>
                                <span class="child-size">{formatBytes(gg.size)}</span>
                              </div>
                            {/each}
                          </div>
                        {/if}
                      {/each}
                    </div>
                  {/if}
                {/each}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}
{/if}

<style>
  .content-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 28px;
    gap: 16px;
  }
  .content-header h2 { font-size: 24px; font-weight: 700; color: #fafafa; letter-spacing: -0.01em; }
  .subtitle { color: #71717a; font-size: 14px; margin-top: 6px; }
  .header-actions { display: flex; gap: 10px; flex-shrink: 0; }

  .btn {
    padding: 10px 22px;
    border: none;
    border-radius: 8px;
    font-size: 14px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    transition: all 0.2s ease;
    white-space: nowrap;
  }
  .btn:disabled { opacity: 0.45; cursor: not-allowed; }
  .btn-secondary {
    background: #27272a;
    color: #e4e4e7;
    border: 1px solid #3f3f46;
  }
  .btn-secondary:hover:not(:disabled) { background: #3f3f46; border-color: #52525b; }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: #71717a;
    min-height: 300px;
  }
  .empty-state h3 { font-size: 18px; color: #a1a1aa; margin-bottom: 8px; font-weight: 600; }
  .empty-state p { font-size: 14px; }
  .spinner {
    width: 44px;
    height: 44px;
    border: 3px solid #27272a;
    border-top: 3px solid #7c3aed;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin-bottom: 20px;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  /* Partitions */
  .partitions {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
    gap: 16px;
    margin-bottom: 32px;
  }
  .partition-card {
    padding: 20px;
    background: #18181b;
    border: 1px solid #27272a;
    border-radius: 12px;
    cursor: pointer;
    transition: all 0.2s ease;
    user-select: none;
  }
  .partition-card:hover { border-color: #7c3aed; background: #1a1625; }
  .partition-card.active { border-color: #7c3aed; background: #1a1625; box-shadow: 0 0 0 1px #7c3aed; }

  .part-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 16px;
  }
  .part-title { display: flex; align-items: center; gap: 12px; }
  .part-icon { font-size: 28px; line-height: 1; }
  .part-mount { display: block; font-size: 16px; font-weight: 600; color: #fafafa; }
  .part-device { display: block; font-size: 11px; color: #52525b; margin-top: 3px; }
  .part-percent { font-size: 24px; font-weight: 700; font-variant-numeric: tabular-nums; }

  .part-bar-bg {
    width: 100%;
    height: 8px;
    background: #27272a;
    border-radius: 4px;
    overflow: hidden;
    margin-bottom: 14px;
  }
  .part-bar-fill {
    height: 100%;
    border-radius: 4px;
    transition: width 0.4s ease;
  }

  .part-stats {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: #71717a;
  }

  /* Analysis Section */
  .analysis-section {
    margin-top: 8px;
  }
  .analysis-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 18px;
  }
  .analysis-header h3 {
    font-size: 16px;
    font-weight: 600;
    color: #d4d4d8;
  }
  .analysis-path {
    color: #7c3aed;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 14px;
  }
  .analysis-total {
    font-size: 13px;
    color: #71717a;
    font-weight: 500;
  }

  /* Stacked Bar */
  .stacked-bar {
    display: flex;
    height: 28px;
    border-radius: 8px;
    overflow: hidden;
    background: #27272a;
    margin-bottom: 12px;
    gap: 1px;
  }
  .stacked-segment {
    height: 100%;
    min-width: 2px;
    transition: opacity 0.15s;
  }
  .stacked-segment:hover {
    opacity: 0.8;
  }

  .stacked-legend {
    display: flex;
    flex-wrap: wrap;
    gap: 10px 18px;
    margin-bottom: 24px;
  }
  .legend-item {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .legend-dot {
    width: 8px;
    height: 8px;
    border-radius: 2px;
    flex-shrink: 0;
  }
  .legend-label {
    font-size: 11px;
    color: #a1a1aa;
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Space Entries */
  .space-entries {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .space-entry-group {
    border-radius: 10px;
    overflow: hidden;
  }
  .space-entry {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 11px 16px;
    border-radius: 10px;
    transition: background 0.12s ease;
    user-select: none;
  }
  .space-entry.expandable { cursor: pointer; }
  .space-entry:hover { background: #18181b; }
  .space-entry.expandable:hover { background: #1a1625; }

  .entry-rank {
    font-size: 12px;
    font-weight: 700;
    min-width: 22px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }
  .entry-icon { font-size: 18px; line-height: 1; flex-shrink: 0; }
  .entry-info { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .entry-name {
    font-size: 13px;
    font-weight: 600;
    color: #e4e4e7;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .entry-path {
    font-size: 11px;
    color: #3f3f46;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin-top: 1px;
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  .entry-bar-container {
    width: 180px;
    height: 7px;
    background: #1e1e22;
    border-radius: 4px;
    overflow: hidden;
    flex-shrink: 0;
  }
  .entry-bar {
    height: 100%;
    border-radius: 4px;
    transition: width 0.3s ease;
  }

  .entry-size {
    font-size: 12px;
    color: #a1a1aa;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    min-width: 75px;
    text-align: right;
    flex-shrink: 0;
  }

  .entry-chevron {
    color: #52525b;
    font-size: 13px;
    width: 18px;
    text-align: center;
    flex-shrink: 0;
    transition: transform 0.2s ease;
  }
  .entry-chevron.rotated { transform: rotate(180deg); }
  .entry-chevron-placeholder { width: 18px; flex-shrink: 0; }

  /* Children */
  .children-list {
    padding: 2px 0 6px 50px;
    display: flex;
    flex-direction: column;
    gap: 1px;
    background: #111113;
    border-radius: 0 0 10px 10px;
    margin-top: -2px;
  }
  .child-entry {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 14px;
    border-radius: 6px;
    transition: background 0.12s;
    user-select: none;
  }
  .child-entry:hover { background: #18181b; }
  .child-entry.child-expandable { cursor: pointer; }
  .child-entry.child-expandable:hover { background: #1a1625; }
  .child-icon { font-size: 14px; line-height: 1; flex-shrink: 0; }
  .child-name {
    flex: 1;
    font-size: 12px;
    color: #a1a1aa;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .child-entry.child-expandable .child-name { color: #d4d4d8; font-weight: 600; }
  .child-bar-container {
    width: 120px;
    height: 5px;
    background: #1e1e22;
    border-radius: 3px;
    overflow: hidden;
    flex-shrink: 0;
  }
  .child-bar {
    height: 100%;
    border-radius: 3px;
    background: #3f3f46;
  }
  .child-size {
    font-size: 11px;
    color: #71717a;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    min-width: 65px;
    text-align: right;
    flex-shrink: 0;
  }
  .child-chevron {
    color: #52525b;
    font-size: 12px;
    width: 16px;
    text-align: center;
    flex-shrink: 0;
    transition: transform 0.2s ease;
  }
  .child-chevron.rotated { transform: rotate(180deg); }

  .grandchildren-list {
    padding: 2px 0 4px 28px;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .grandchildren-list.deeper {
    padding-left: 28px;
  }
  .grandchild .child-name { font-size: 11px; }
  .grandchild .child-size { font-size: 10px; }

  /* Inline spinners for lazy loading */
  .entry-spinner, .child-spinner {
    width: 14px;
    height: 14px;
    border: 2px solid #27272a;
    border-top: 2px solid #7c3aed;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    flex-shrink: 0;
  }
  .child-spinner {
    width: 12px;
    height: 12px;
  }
</style>
