<!--
  Reusable confirmation modal.
  Usage:
    let confirm = $state({ open: false, ... });
    <ConfirmDialog
      bind:open={confirm.open}
      title="Remove package?"
      message="This will uninstall Firefox via pkexec."
      confirmLabel="Remove"
      destructive
      onconfirm={() => doRemove()}
    />
-->
<script>
  let {
    open = $bindable(false),
    title = "Are you sure?",
    message = "",
    confirmLabel = "Confirm",
    cancelLabel = "Cancel",
    destructive = false,
    onconfirm = () => {},
    oncancel = () => {},
  } = $props();

  function close() {
    open = false;
    oncancel();
  }

  function confirm() {
    open = false;
    onconfirm();
  }

  function onkeydown(e) {
    if (!open) return;
    if (e.key === "Escape") close();
    if (e.key === "Enter") confirm();
  }
</script>

<svelte:window on:keydown={onkeydown} />

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="overlay" role="presentation" onclick={close}>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="confirm-title"
      tabindex="-1"
      onkeydown={(e) => e.stopPropagation()}
    >
      <h3 id="confirm-title">{title}</h3>
      {#if message}
        <p class="message">{message}</p>
      {/if}
      <div class="actions">
        <button type="button" class="btn-cancel" onclick={close}>{cancelLabel}</button>
        <button
          type="button"
          class="btn-confirm"
          class:destructive
          onclick={confirm}
        >
          {confirmLabel}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(8, 12, 20, 0.65);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    animation: fade 0.15s ease;
  }
  .dialog {
    background: var(--bg-elevated, #1a2332);
    border: 1px solid var(--border, #2a3a52);
    border-radius: 10px;
    padding: 1.5rem;
    min-width: 320px;
    max-width: 480px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.4);
    color: var(--text, #e6edf3);
  }
  h3 {
    margin: 0 0 0.5rem;
    font-size: 1.1rem;
  }
  .message {
    margin: 0 0 1.25rem;
    color: var(--text-muted, #9aa9bd);
    line-height: 1.5;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }
  .btn-cancel,
  .btn-confirm {
    padding: 0.5rem 1rem;
    border-radius: 6px;
    border: 1px solid var(--border, #2a3a52);
    background: transparent;
    color: inherit;
    cursor: pointer;
    font-size: 0.9rem;
  }
  .btn-cancel:hover {
    background: rgba(255, 255, 255, 0.05);
  }
  .btn-confirm {
    background: var(--accent, #d97706);
    border-color: var(--accent, #d97706);
    color: #fff;
  }
  .btn-confirm:hover {
    filter: brightness(1.1);
  }
  .btn-confirm.destructive {
    background: #b91c1c;
    border-color: #b91c1c;
  }
  @keyframes fade {
    from { opacity: 0; }
    to { opacity: 1; }
  }
</style>
