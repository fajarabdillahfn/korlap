<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { appState, type ActiveTask } from '$lib/state.svelte';
  import ChatPanel from './ChatPanel.svelte';
  import TerminalPanel from './TerminalPanel.svelte';
  import DiffViewer from './DiffViewer.svelte';

  let { task }: { task: ActiveTask } = $props();

  let merging = $state(false);
  let mergeError = $state<string | null>(null);

  async function merge() {
    merging = true;
    mergeError = null;
    try {
      await invoke('update_task_status', {
        task_id: task.id,
        status: 'done',
      });
      // Success — return to Kanban board
      appState.activeTaskId = null;
      appState.activeTask = null;
    } catch (e) {
      mergeError = String(e);
    } finally {
      merging = false;
    }
  }

  const statusLabel: Record<string, string> = {
    todo: 'Todo',
    in_progress: 'In Progress',
    review: 'Review',
    done: 'Done',
  };
</script>

<div class="workspace-view">
  <div class="workspace-header">
    <button class="back-btn" onclick={() => { appState.activeTaskId = null; appState.activeTask = null; }}>
      ← Kanban
    </button>
    <div class="task-info">
      <span class="task-title">{task.title}</span>
      {#if task.branch_name}
        <span class="branch-name">{task.branch_name}</span>
      {/if}
    </div>
    <span class="status-badge status-{task.status}">{statusLabel[task.status] ?? task.status}</span>
  </div>

  <div class="workspace-body">
    <div class="chat-pane">
      <ChatPanel taskId={task.id} />
    </div>
    <div class="terminal-pane">
      <TerminalPanel taskId={task.id} />
    </div>
  </div>

  <div class="kbd-hints">
    <span class="kbd-hint"><kbd>Esc</kbd> Back to Kanban</span>
    <span class="kbd-hint"><kbd>Ctrl+L</kbd> Focus chat</span>
    <span class="kbd-hint"><kbd>A</kbd> Approve tool call</span>
    <span class="kbd-hint"><kbd>X</kbd> Reject tool call</span>
  </div>

  {#if task.status === 'review'}
    <div class="review-panel">
      <div class="review-header">
        <span class="review-title">Diff — {task.branch_name ?? 'branch'} vs main</span>
        <button
          class="merge-btn"
          onclick={merge}
          disabled={merging}
        >
          {#if merging}
            <span class="merge-spinner"></span> Merging…
          {:else}
            Merge & Complete
          {/if}
        </button>
      </div>

      {#if mergeError}
        <div class="merge-error">{mergeError}</div>
      {/if}

      {#if task.diff}
        <DiffViewer diff={task.diff} />
      {:else}
        <p class="no-diff">No diff available.</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .workspace-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-base);
    overflow: hidden;
  }

  .workspace-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 14px;
    background: var(--bg-titlebar);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .back-btn {
    background: transparent;
    border: 1px solid var(--border-light);
    border-radius: 4px;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 12px;
    padding: 4px 10px;
    transition: color 0.1s, border-color 0.1s;
  }

  .back-btn:hover {
    color: var(--text-primary);
    border-color: var(--accent);
  }

  .task-info {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: 1;
    min-width: 0;
  }

  .task-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .branch-name {
    font-size: 11px;
    color: var(--text-muted);
    font-family: 'JetBrains Mono', monospace;
    white-space: nowrap;
  }

  .status-badge {
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 10px;
    border: 1px solid var(--border-light);
    color: var(--text-secondary);
    white-space: nowrap;
  }

  .status-badge.status-in_progress {
    border-color: var(--accent);
    color: var(--accent);
  }

  .status-badge.status-review {
    border-color: var(--status-ok);
    color: var(--status-ok);
  }

  .workspace-body {
    display: flex;
    flex: 1;
    overflow: hidden;
    min-height: 0;
  }

  .chat-pane {
    flex: 3;
    min-width: 0;
    border-right: 1px solid var(--border);
    overflow: hidden;
  }

  .terminal-pane {
    flex: 2;
    min-width: 0;
    overflow: hidden;
  }

  .review-panel {
    flex-shrink: 0;
    border-top: 1px solid var(--border);
    padding: 12px 16px;
    background: var(--bg-card);
    max-height: 280px;
    overflow-y: auto;
  }

  .review-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }

  .review-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .merge-btn {
    background: var(--status-ok);
    border: none;
    border-radius: 6px;
    color: var(--text-primary);
    cursor: pointer;
    font-size: 13px;
    font-weight: 600;
    padding: 6px 16px;
    transition: opacity 0.1s;
  }

  .merge-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .merge-spinner {
    display: inline-block;
    width: 11px;
    height: 11px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: var(--text-primary);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
    vertical-align: middle;
    margin-right: 2px;
  }

  .merge-error {
    background: var(--error-bg);
    border: 1px solid var(--error);
    border-radius: 6px;
    color: var(--error);
    font-size: 12px;
    margin-bottom: 10px;
    padding: 8px 12px;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .no-diff {
    color: var(--text-muted);
    font-size: 13px;
    margin: 0;
  }

  .kbd-hints {
    display: flex;
    gap: 16px;
    padding: 4px 14px;
    background: var(--bg-titlebar);
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .kbd-hint {
    font-size: 11px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: 4px;
  }

  kbd {
    background: var(--bg-hover);
    border: 1px solid var(--border-light);
    border-radius: 3px;
    font-size: 10px;
    font-family: 'JetBrains Mono', monospace;
    padding: 1px 5px;
    color: var(--text-secondary);
  }
</style>
