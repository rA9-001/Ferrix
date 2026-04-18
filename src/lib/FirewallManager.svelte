<script>
  import { invoke } from "@tauri-apps/api/core";
  import ConfirmDialog from "./ConfirmDialog.svelte";

  let confirmDisableOpen = $state(false);
  let confirmDeleteOpen = $state(false);
  let pendingDeleteNumber = $state(null);

  let status = $state(null);
  let loading = $state(true);
  let toggling = $state(false);
  let deleting = $state(new Set());
  let adding = $state(false);
  let message = $state(null);

  // Add rule form
  let showAddForm = $state(false);
  let formAction = $state("allow");
  let formDirection = $state("in");
  let formPort = $state("");
  let formProtocol = $state("any");
  let formFromIp = $state("");
  let formComment = $state("");

  async function loadStatus() {
    loading = true;
    status = await invoke("get_firewall_status");
    loading = false;
  }

  async function toggleFirewall() {
    if (status?.active) {
      // Confirm before disabling — turning the firewall off is destructive.
      confirmDisableOpen = true;
      return;
    }
    await performToggle();
  }

  async function performToggle() {
    toggling = true;
    message = null;
    try {
      const response = await invoke("toggle_firewall", { enable: !status.active });
      message = response.result;
      status = response.status;
    } finally {
      toggling = false;
    }
  }

  async function changeDefaultPolicy(direction, policy) {
    message = null;
    const response = await invoke("set_default_policy", { direction, policy });
    message = response.result;
    status = response.status;
  }

  async function changeLogging(level) {
    message = null;
    const response = await invoke("set_firewall_logging", { level });
    message = response.result;
    status = response.status;
  }

  async function deleteRule(number) {
    pendingDeleteNumber = number;
    confirmDeleteOpen = true;
  }

  async function performDelete() {
    const number = pendingDeleteNumber;
    if (number == null) return;
    const next = new Set(deleting);
    next.add(number);
    deleting = next;
    message = null;

    try {
      const response = await invoke("delete_firewall_rule", { number });
      message = response.result;
      status = response.status;
    } finally {
      const done = new Set(deleting);
      done.delete(number);
      deleting = done;
      pendingDeleteNumber = null;
    }
  }

  async function addRule() {
    adding = true;
    message = null;

    try {
      const response = await invoke("add_firewall_rule", {
        action: formAction,
        direction: formDirection,
        port: formPort.trim(),
        protocol: formProtocol,
        fromIp: formFromIp.trim(),
        comment: formComment.trim(),
      });
      message = response.result;
      if (response.result.success) {
        formPort = "";
        formFromIp = "";
        formComment = "";
        showAddForm = false;
      }
      status = response.status;
    } finally {
      adding = false;
    }
  }

  $effect(() => {
    loadStatus();
  });

  let ruleCount = $derived(status?.rules?.length ?? 0);

  function actionColor(action) {
    const a = action.toUpperCase();
    if (a.includes("ALLOW")) return "allow";
    if (a.includes("LIMIT")) return "limit";
    return "deny";
  }
</script>

<div class="firewall">
  <div class="header">
    <div class="header-left">
      <h2>Firewall Manager</h2>
      {#if status}
        <span class="status-pill" class:active={status.active} class:inactive={!status.active}>
          {status.active ? "Active" : "Inactive"}
        </span>
        {#if status.backend !== "none"}
          <span class="backend-badge">{status.backend.toUpperCase()}</span>
        {/if}
      {/if}
    </div>
    <div class="header-actions">
      <button class="btn btn-secondary" onclick={loadStatus} disabled={loading}>
        <span class:spin={loading}>↻</span> Refresh
      </button>
      {#if status && status.backend !== "none"}
        <button
          class="btn"
          class:btn-danger={status.active}
          class:btn-primary={!status.active}
          onclick={toggleFirewall}
          disabled={toggling}
        >
          {#if toggling}
            <span class="spin">↻</span>
          {:else}
            {#if status.active}<svg style="width:1em;height:1em;vertical-align:middle;display:inline-block;margin-right:4px" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 9.9-1"/></svg> Disable{:else}<svg style="width:1em;height:1em;vertical-align:middle;display:inline-block;margin-right:4px" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg> Enable{/if}
          {/if}
        </button>
      {/if}
    </div>
  </div>

  {#if message}
    <div class="message" class:success={message.success} class:error={!message.success}>
      <span>{message.success ? "✓" : "✗"}</span>
      {message.message}
    </div>
  {/if}

  {#if loading && !status}
    <div class="loading">
      <div class="spinner"></div>
      <p>Detecting firewall...</p>
    </div>
  {:else if !status || status.backend === "none"}
    <div class="empty-state">
      <div class="empty-icon"><svg style="width:48px;height:48px" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22c4-4 8-7.5 8-12a8 8 0 0 0-16 0c0 4.5 4 8 8 12z"/><path d="M12 22c-2-2-4-3.75-4-6a4 4 0 0 1 8 0c0 2.25-2 4-4 6z"/></svg></div>
      <h3>No Firewall Detected</h3>
      <p>Install UFW to manage your firewall:</p>
      <code class="install-cmd">sudo pacman -S ufw && sudo systemctl enable --now ufw</code>
    </div>
  {:else}
    <div class="info-banner">
      <span class="info-icon"><svg style="width:1em;height:1em;vertical-align:middle" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg></span>
      <span>UFW (Uncomplicated Firewall) manages iptables/nftables rules. Changes take effect immediately and persist across reboots. Root access is required.</span>
    </div>

    <!-- Defaults & Logging Panel -->
    <div class="controls-grid">
      <div class="control-card">
        <div class="control-label">Default Incoming</div>
        <div class="control-value">
          <select
            class="policy-select"
            class:policy-deny={status.default_incoming === "deny" || status.default_incoming === "reject"}
            class:policy-allow={status.default_incoming === "allow"}
            value={status.default_incoming}
            onchange={(e) => changeDefaultPolicy("incoming", e.target.value)}
          >
            <option value="deny">Deny</option>
            <option value="reject">Reject</option>
            <option value="allow">Allow</option>
          </select>
        </div>
      </div>
      <div class="control-card">
        <div class="control-label">Default Outgoing</div>
        <div class="control-value">
          <select
            class="policy-select"
            class:policy-deny={status.default_outgoing === "deny" || status.default_outgoing === "reject"}
            class:policy-allow={status.default_outgoing === "allow"}
            value={status.default_outgoing}
            onchange={(e) => changeDefaultPolicy("outgoing", e.target.value)}
          >
            <option value="allow">Allow</option>
            <option value="deny">Deny</option>
            <option value="reject">Reject</option>
          </select>
        </div>
      </div>
      <div class="control-card">
        <div class="control-label">Logging</div>
        <div class="control-value">
          <select
            class="policy-select policy-neutral"
            value={status.logging.split(" ")[1]?.replace("(", "").replace(")", "") || status.logging.split(" ")[0]}
            onchange={(e) => changeLogging(e.target.value)}
          >
            <option value="off">Off</option>
            <option value="low">Low</option>
            <option value="medium">Medium</option>
            <option value="high">High</option>
            <option value="full">Full</option>
          </select>
        </div>
      </div>
      <div class="control-card">
        <div class="control-label">Rules</div>
        <div class="control-value">
          <span class="rule-count">{ruleCount}</span>
        </div>
      </div>
    </div>

    <!-- Rules Section -->
    <section class="rules-section">
      <div class="rules-header">
        <h3 class="section-title">
          <span class="section-icon"><svg style="width:1em;height:1em;vertical-align:middle" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/><rect x="8" y="2" width="8" height="4" rx="1" ry="1"/></svg></span>
          Firewall Rules
        </h3>
        <button class="btn btn-primary btn-small" onclick={() => showAddForm = !showAddForm}>
          {showAddForm ? "✕ Cancel" : "+ Add Rule"}
        </button>
      </div>

      {#if showAddForm}
        <div class="add-form">
          <div class="form-row">
            <div class="form-group">
              <label class="form-label">
                Action
                <select class="form-input" bind:value={formAction}>
                  <option value="allow">Allow</option>
                  <option value="deny">Deny</option>
                  <option value="reject">Reject</option>
                  <option value="limit">Limit</option>
                </select>
              </label>
            </div>
            <div class="form-group">
              <label class="form-label">
                Direction
                <select class="form-input" bind:value={formDirection}>
                  <option value="in">Incoming</option>
                  <option value="out">Outgoing</option>
                </select>
              </label>
            </div>
            <div class="form-group">
              <label class="form-label">
                Port
                <input
                  type="text"
                  class="form-input"
                  placeholder="e.g. 22, 80, 3000:3010"
                  bind:value={formPort}
                />
              </label>
            </div>
            <div class="form-group">
              <label class="form-label">
                Protocol
                <select class="form-input" bind:value={formProtocol}>
                  <option value="any">Any</option>
                  <option value="tcp">TCP</option>
                  <option value="udp">UDP</option>
                </select>
              </label>
            </div>
          </div>
          <div class="form-row">
            <div class="form-group flex-2">
              <label class="form-label">
                Source IP <span class="optional">(optional)</span>
                <input
                  type="text"
                  class="form-input"
                  placeholder="e.g. 192.168.1.0/24"
                  bind:value={formFromIp}
                />
              </label>
            </div>
            <div class="form-group flex-2">
              <label class="form-label">
                Comment <span class="optional">(optional)</span>
                <input
                  type="text"
                  class="form-input"
                  placeholder="e.g. SSH access"
                  bind:value={formComment}
                />
              </label>
            </div>
            <div class="form-group form-submit">
              <button class="btn btn-primary" onclick={addRule} disabled={adding || !formPort.trim()}>
                {#if adding}
                  <span class="spin">↻</span>
                {:else}
                  Add Rule
                {/if}
              </button>
            </div>
          </div>
        </div>
      {/if}

      {#if status.rules.length === 0}
        <div class="no-rules">
          <p>No rules configured. Default policies apply to all traffic.</p>
        </div>
      {:else}
        <div class="rules-table">
          <div class="table-header">
            <span class="col-num">#</span>
            <span class="col-to">To</span>
            <span class="col-action">Action</span>
            <span class="col-from">From</span>
            <span class="col-comment">Comment</span>
            <span class="col-delete"></span>
          </div>
          {#each status.rules as rule}
            <div class="table-row">
              <span class="col-num">{rule.number}</span>
              <span class="col-to mono">{rule.to}</span>
              <span class="col-action">
                <span class="action-badge {actionColor(rule.action)}">{rule.action}</span>
              </span>
              <span class="col-from mono">{rule.from}</span>
              <span class="col-comment">{rule.comment || "—"}</span>
              <span class="col-delete">
                <button
                  class="btn-delete"
                  onclick={() => deleteRule(rule.number)}
                  disabled={deleting.has(rule.number)}
                  title="Delete rule"
                >
                  {#if deleting.has(rule.number)}
                    <span class="spin">↻</span>
                  {:else}
                    ✕
                  {/if}
                </button>
              </span>
            </div>
          {/each}
        </div>
      {/if}
    </section>
  {/if}
</div>

<ConfirmDialog
  bind:open={confirmDisableOpen}
  title="Disable firewall?"
  message="Disabling the firewall removes all incoming-traffic protection until you re-enable it."
  confirmLabel="Disable"
  destructive
  onconfirm={performToggle}
/>

<ConfirmDialog
  bind:open={confirmDeleteOpen}
  title="Delete firewall rule?"
  message={`Rule #${pendingDeleteNumber} will be permanently removed.`}
  confirmLabel="Delete"
  destructive
  onconfirm={performDelete}
/>

<style>
  .firewall {
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

  .status-pill {
    font-size: 0.8rem;
    font-weight: 700;
    text-transform: uppercase;
    padding: 3px 12px;
    border-radius: 12px;
    letter-spacing: 0.5px;
  }

  .status-pill.active {
    color: #4ade80;
    background: #22c55e18;
  }

  .status-pill.inactive {
    color: #f87171;
    background: #ef444418;
  }

  .backend-badge {
    font-size: 0.72rem;
    font-weight: 700;
    color: #94a3b8;
    background: #1e293b;
    border: 1px solid #334155;
    padding: 2px 8px;
    border-radius: 6px;
    letter-spacing: 0.5px;
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
    background: #2563eb;
    color: #fff;
  }

  .btn-primary:hover {
    background: #1d4ed8;
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-small {
    padding: 6px 14px;
    font-size: 0.82rem;
  }

  .btn-danger {
    background: #1e293b;
    border: 1px solid #ef4444;
    color: #f87171;
  }

  .btn-danger:hover {
    background: #ef4444;
    color: #fff;
  }

  .btn-danger:disabled {
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

  .message {
    padding: 10px 14px;
    border-radius: 8px;
    font-size: 0.85rem;
    margin-bottom: 16px;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .message.success {
    background: #22c55e12;
    border: 1px solid #22c55e40;
    color: #4ade80;
  }

  .message.error {
    background: #ef444418;
    border: 1px solid #ef444440;
    color: #f87171;
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
    border-top-color: #2563eb;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .empty-state {
    text-align: center;
    padding: 60px 0;
    color: #64748b;
  }

  .empty-icon {
    font-size: 3rem;
    margin-bottom: 12px;
  }

  .empty-state h3 {
    color: #e2e8f0;
    margin: 0 0 8px 0;
  }

  .empty-state p {
    margin: 0 0 16px 0;
    font-size: 0.95rem;
  }

  .install-cmd {
    display: inline-block;
    background: #0f172a;
    border: 1px solid #334155;
    padding: 8px 16px;
    border-radius: 8px;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 0.85rem;
    color: #94a3b8;
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

  /* Controls Grid */
  .controls-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
    margin-bottom: 24px;
  }

  .control-card {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 10px;
    padding: 14px 16px;
    text-align: center;
  }

  .control-label {
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #64748b;
    margin-bottom: 8px;
  }

  .policy-select {
    background: #0f172a;
    border: 1px solid #334155;
    color: #e2e8f0;
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    width: 100%;
    text-align: center;
    appearance: none;
    -webkit-appearance: none;
  }

  .policy-select.policy-deny {
    color: #f87171;
    border-color: #ef444440;
  }

  .policy-select.policy-allow {
    color: #4ade80;
    border-color: #22c55e40;
  }

  .policy-select.policy-neutral {
    color: #94a3b8;
  }

  .rule-count {
    font-size: 1.5rem;
    font-weight: 700;
    color: #93c5fd;
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  /* Rules Section */
  .rules-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .rules-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
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

  /* Add Rule Form */
  .add-form {
    background: #1e293b;
    border: 1px solid #2563eb40;
    border-radius: 12px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .form-row {
    display: flex;
    gap: 12px;
    align-items: flex-end;
    flex-wrap: wrap;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
    min-width: 120px;
  }

  .form-group.flex-2 {
    flex: 2;
  }

  .form-group.form-submit {
    flex: 0;
    min-width: auto;
  }

  .form-label {
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    color: #94a3b8;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .optional {
    font-weight: 400;
    text-transform: none;
    color: #64748b;
  }

  .form-input {
    background: #0f172a;
    border: 1px solid #334155;
    color: #e2e8f0;
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 0.85rem;
    transition: border-color 0.15s;
  }

  .form-input:focus {
    outline: none;
    border-color: #2563eb;
  }

  .form-input::placeholder {
    color: #475569;
  }

  /* Rules Table */
  .no-rules {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 12px;
    padding: 32px;
    text-align: center;
    color: #64748b;
    font-size: 0.9rem;
  }

  .no-rules p {
    margin: 0;
  }

  .rules-table {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 12px;
    overflow: hidden;
  }

  .table-header {
    display: flex;
    padding: 10px 16px;
    background: #0f172a;
    border-bottom: 1px solid #334155;
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #64748b;
    gap: 8px;
  }

  .table-row {
    display: flex;
    padding: 12px 16px;
    align-items: center;
    gap: 8px;
    border-bottom: 1px solid #1e293b;
    transition: background 0.1s;
  }

  .table-row:last-child {
    border-bottom: none;
  }

  .table-row:hover {
    background: #0f172a80;
  }

  .col-num {
    width: 36px;
    flex-shrink: 0;
    color: #475569;
    font-size: 0.82rem;
    font-weight: 600;
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  .col-to {
    flex: 2;
    min-width: 0;
    font-size: 0.88rem;
    color: #e2e8f0;
  }

  .col-action {
    width: 100px;
    flex-shrink: 0;
  }

  .col-from {
    flex: 2;
    min-width: 0;
    font-size: 0.88rem;
    color: #e2e8f0;
  }

  .col-comment {
    flex: 2;
    min-width: 0;
    font-size: 0.82rem;
    color: #64748b;
    font-style: italic;
  }

  .col-delete {
    width: 36px;
    flex-shrink: 0;
    text-align: center;
  }

  .mono {
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 0.82rem;
  }

  .action-badge {
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: uppercase;
    padding: 3px 10px;
    border-radius: 6px;
    letter-spacing: 0.3px;
  }

  .action-badge.allow {
    background: #22c55e18;
    color: #4ade80;
  }

  .action-badge.deny {
    background: #ef444418;
    color: #f87171;
  }

  .action-badge.limit {
    background: #f59e0b18;
    color: #fbbf24;
  }

  .btn-delete {
    background: transparent;
    border: 1px solid transparent;
    color: #475569;
    width: 28px;
    height: 28px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.8rem;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s;
  }

  .btn-delete:hover {
    background: #ef444418;
    border-color: #ef444440;
    color: #f87171;
  }

  .btn-delete:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  @media (max-width: 700px) {
    .controls-grid {
      grid-template-columns: repeat(2, 1fr);
    }

    .form-row {
      flex-direction: column;
    }

    .form-group {
      min-width: 100%;
    }

    .col-comment {
      display: none;
    }
  }
</style>
