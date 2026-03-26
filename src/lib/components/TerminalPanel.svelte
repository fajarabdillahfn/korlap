<script lang="ts">
  import Terminal from './Terminal.svelte';
  import { invoke } from '@tauri-apps/api/core';

  let { taskId }: { taskId: string } = $props();

  interface TermTab {
    id: string;
    label: string;
  }

  const initialTabId = `${taskId}-term-1`;
  let tabs = $state<TermTab[]>([{ id: initialTabId, label: 'Terminal 1' }]);
  let activeTabId = $state(initialTabId);
  let nextIndex = $state(2);

  function addTab() {
    const id = `${taskId}-term-${nextIndex}`;
    tabs = [...tabs, { id, label: `Terminal ${nextIndex}` }];
    activeTabId = id;
    nextIndex += 1;
  }

  function closeTab(tabId: string) {
    if (tabs.length === 1) return; // keep at least one
    // Kill the PTY process before removing the tab to prevent shell process leaks.
    invoke('pty_kill', { terminal_id: tabId }).catch(() => {});
    const idx = tabs.findIndex(t => t.id === tabId);
    tabs = tabs.filter(t => t.id !== tabId);
    if (activeTabId === tabId) {
      activeTabId = tabs[Math.max(0, idx - 1)].id;
    }
  }
</script>

<div class="terminal-panel">
  <div class="tab-bar">
    {#each tabs as tab (tab.id)}
      <button
        class="tab"
        class:active={tab.id === activeTabId}
        onclick={() => { activeTabId = tab.id; }}
      >
        {tab.label}
        {#if tabs.length > 1}
          <span
            class="tab-close"
            role="button"
            tabindex="0"
            onclick={(e) => { e.stopPropagation(); closeTab(tab.id); }}
            onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); closeTab(tab.id); } }}
          >×</span>
        {/if}
      </button>
    {/each}
    <button class="new-tab-btn" onclick={addTab}>+</button>
  </div>

  <div class="tab-content">
    {#each tabs as tab (tab.id)}
      <div class="term-wrapper" class:hidden={tab.id !== activeTabId}>
        <Terminal taskId={taskId} terminalId={tab.id} />
      </div>
    {/each}
  </div>
</div>

<style>
  .terminal-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-base);
  }

  .tab-bar {
    display: flex;
    align-items: center;
    background: var(--bg-card);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    overflow-x: auto;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    font-size: 12px;
    background: transparent;
    border: none;
    border-right: 1px solid var(--border);
    color: var(--text-muted);
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.1s, color 0.1s;
  }

  .tab:hover,
  .tab.active {
    background: var(--bg-active);
    color: var(--text-primary);
  }

  .tab-close {
    font-size: 14px;
    line-height: 1;
    color: var(--text-muted);
    cursor: pointer;
  }

  .tab-close:hover {
    color: var(--error);
  }

  .new-tab-btn {
    padding: 6px 12px;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
  }

  .new-tab-btn:hover {
    color: var(--text-primary);
  }

  .tab-content {
    flex: 1;
    overflow: hidden;
    position: relative;
  }

  .term-wrapper {
    position: absolute;
    inset: 0;
  }

  .term-wrapper.hidden {
    display: none;
  }
</style>
