<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';

  async function minimize() {
    await getCurrentWindow().minimize();
  }

  async function toggleMaximize() {
    await getCurrentWindow().toggleMaximize();
  }

  async function close() {
    await getCurrentWindow().close();
  }
</script>

<div class="titlebar" data-tauri-drag-region>
  <div class="titlebar-controls">
    <button class="titlebar-btn minimize" onclick={minimize} aria-label="Minimize">
      <svg width="10" height="1" viewBox="0 0 10 1" fill="none">
        <line x1="0" y1="0.5" x2="10" y2="0.5" stroke="currentColor" stroke-width="1.5"/>
      </svg>
    </button>
    <button class="titlebar-btn maximize" onclick={toggleMaximize} aria-label="Maximize">
      <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
        <rect x="0.75" y="0.75" width="8.5" height="8.5" stroke="currentColor" stroke-width="1.5"/>
      </svg>
    </button>
    <button class="titlebar-btn close" onclick={close} aria-label="Close">
      <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
        <line x1="1" y1="1" x2="9" y2="9" stroke="currentColor" stroke-width="1.5"/>
        <line x1="9" y1="1" x2="1" y2="9" stroke="currentColor" stroke-width="1.5"/>
      </svg>
    </button>
  </div>
</div>

<style>
  .titlebar {
    height: 32px;
    background: var(--bg-titlebar);
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding: 0 8px;
    flex-shrink: 0;
    user-select: none;
    -webkit-user-select: none;
  }

  .titlebar-controls {
    display: flex;
    gap: 4px;
  }

  .titlebar-btn {
    width: 28px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-muted);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .titlebar-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .titlebar-btn.close:hover {
    background: var(--error);
    color: var(--text-on-error);
  }
</style>
