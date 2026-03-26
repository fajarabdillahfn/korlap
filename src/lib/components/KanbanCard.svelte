<script lang="ts">
  interface Task {
    id: string;
    repo_id: string;
    title: string;
    status: string;
    branch_name: string | null;
    worktree_path: string | null;
    created_at: string;
    updated_at: string;
  }

  let {
    task,
    onMoveTask,
    onDeleteTask,
    onSelect,
  }: {
    task: Task;
    onMoveTask: (taskId: string, newStatus: string, branchName?: string) => Promise<void>;
    onDeleteTask: (taskId: string) => void;
    onSelect: (task: Task) => void;
  } = $props();

  let showActions = $state(false);
  let confirmDelete = $state(false);
  let moving = $state(false);

  const nextStatus: Record<string, string> = {
    todo: 'in_progress',
    in_progress: 'review',
    review: 'done',
  };

  const nextLabel: Record<string, string> = {
    todo: 'Start',
    in_progress: 'Review',
    review: 'Done',
  };

  async function handleMove() {
    const next = nextStatus[task.status];
    if (!next || moving) return;

    let branchName: string | undefined;
    if (task.status === 'todo') {
      const branch = prompt('Enter branch name (e.g. feat/my-feature):');
      if (!branch?.trim()) return;
      branchName = branch.trim();
    }

    moving = true;
    try {
      await onMoveTask(task.id, next, branchName);
    } finally {
      moving = false;
    }
  }

  function handleDelete() {
    if (!confirmDelete) {
      confirmDelete = true;
      setTimeout(() => { confirmDelete = false; }, 3000);
      return;
    }
    onDeleteTask(task.id);
  }
</script>

<div
  class="card"
  role="button"
  tabindex="0"
  onmouseenter={() => { showActions = true; }}
  onmouseleave={() => { if (!confirmDelete) showActions = false; }}
  onclick={() => onSelect(task)}
  onkeydown={(e) => { if (e.key === 'Enter') onSelect(task); }}
>
  <div class="card-title">{task.title}</div>

  {#if task.branch_name}
    <div class="card-branch">{task.branch_name}</div>
  {/if}

  {#if showActions}
    <div class="card-actions">
      {#if nextStatus[task.status]}
        <button
          class="action-btn move-btn"
          class:loading={moving}
          onclick={(e) => { e.stopPropagation(); handleMove(); }}
          disabled={moving}
        >
          {#if moving}
            <span class="btn-spinner"></span>
          {:else}
            {nextLabel[task.status]}
          {/if}
        </button>
      {/if}
      <button
        class="action-btn delete-btn"
        class:confirm={confirmDelete}
        onclick={(e) => { e.stopPropagation(); handleDelete(); }}
      >
        {confirmDelete ? 'Confirm?' : 'Delete'}
      </button>
    </div>
  {/if}
</div>

<style>
  .card {
    background: var(--bg-base);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 10px 12px;
    cursor: default;
    transition: border-color 0.1s;
    position: relative;
  }

  .card:hover {
    border-color: var(--border-light);
  }

  .card-title {
    font-size: 13px;
    color: var(--text-primary);
    line-height: 1.4;
    word-break: break-word;
  }

  .card-branch {
    font-size: 11px;
    color: var(--text-muted);
    margin-top: 4px;
    font-family: 'JetBrains Mono', monospace;
  }

  .card-actions {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }

  .action-btn {
    font-size: 11px;
    padding: 3px 8px;
    border-radius: 4px;
    border: 1px solid var(--border-light);
    background: var(--bg-card);
    color: var(--text-secondary);
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
  }

  .move-btn:hover {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--bg-base);
  }

  .delete-btn:hover,
  .delete-btn.confirm {
    background: var(--error-bg);
    border-color: var(--error);
    color: var(--error);
  }

  .move-btn.loading {
    cursor: not-allowed;
    opacity: 0.7;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .btn-spinner {
    display: inline-block;
    width: 10px;
    height: 10px;
    border: 2px solid var(--border-light);
    border-top-color: var(--bg-base);
    border-radius: 50%;
    animation: spin 0.5s linear infinite;
  }
</style>
