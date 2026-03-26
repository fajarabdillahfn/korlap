<script lang="ts">
  import { toasts, dismissToast } from '$lib/toast.svelte';
</script>

<div class="toast-stack" aria-live="polite">
  {#each toasts as toast (toast.id)}
    <div class="toast toast-{toast.kind}" role="alert">
      <span class="toast-msg">{toast.message}</span>
      <button class="toast-close" onclick={() => dismissToast(toast.id)} aria-label="Dismiss">×</button>
    </div>
  {/each}
</div>

<style>
  .toast-stack {
    position: fixed;
    bottom: 20px;
    right: 20px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 1000;
    max-width: 420px;
  }

  .toast {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 12px 14px;
    border-radius: 8px;
    border: 1px solid var(--border-light);
    background: var(--bg-card);
    font-size: 13px;
    color: var(--text-primary);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
    animation: slide-in 0.15s ease-out;
  }

  @keyframes slide-in {
    from { opacity: 0; transform: translateY(8px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  .toast-error {
    border-color: var(--error);
    background: var(--error-bg);
    color: var(--text-on-error);
  }

  .toast-success {
    border-color: var(--status-ok);
  }

  .toast-info {
    border-color: var(--accent);
  }

  .toast-msg {
    flex: 1;
    line-height: 1.4;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .toast-close {
    background: transparent;
    border: none;
    color: inherit;
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
    opacity: 0.7;
    padding: 0;
    flex-shrink: 0;
  }

  .toast-close:hover {
    opacity: 1;
  }
</style>
