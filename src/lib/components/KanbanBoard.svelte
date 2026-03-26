<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { appState } from '$lib/state.svelte';
  import { showToast } from '$lib/toast.svelte';
  import KanbanColumn from './KanbanColumn.svelte';

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

  interface TaskWithDiff {
    task: Task;
    diff: string | null;
  }

  let { repoId }: { repoId: string } = $props();

  let tasks = $state<Task[]>([]);
  let loading = $state(false);
  let showArchived = $state(false);
  let pendingDiff = $state<string | null>(null);

  let todoTasks = $derived(tasks.filter(t => t.status === 'todo'));
  let inProgressTasks = $derived(tasks.filter(t => t.status === 'in_progress'));
  let reviewTasks = $derived(tasks.filter(t => t.status === 'review'));
  let doneTasks = $derived(tasks.filter(t => t.status === 'done'));

  async function loadTasks(id: string) {
    loading = true;
    try {
      tasks = await invoke<Task[]>('list_tasks', { repo_id: id });
    } finally {
      loading = false;
    }
  }

  async function moveTask(taskId: string, newStatus: string, branchName?: string) {
    try {
      const result = await invoke<TaskWithDiff>('update_task_status', {
        task_id: taskId,
        status: newStatus,
        branch_name: branchName ?? null,
      });
      tasks = tasks.map(t => t.id === taskId ? result.task : t);
      if (result.diff) {
        pendingDiff = result.diff;
        // If this task is currently open in workspace, update its diff
        if (appState.activeTaskId === taskId && appState.activeTask) {
          appState.activeTask = { ...appState.activeTask, status: result.task.status, diff: result.diff };
        }
      }
    } catch (e) {
      showToast(String(e));
    }
  }

  function selectTask(task: Task) {
    const existing = tasks.find(t => t.id === task.id);
    const diff = pendingDiff && existing?.status === 'review' ? pendingDiff : null;
    appState.activeTask = { ...task, diff };
    appState.activeTaskId = task.id;
  }

  async function deleteTask(taskId: string) {
    try {
      await invoke('delete_task', { task_id: taskId });
      tasks = tasks.filter(t => t.id !== taskId);
    } catch (e) {
      showToast(String(e));
    }
  }

  function onTaskCreated(task: Task) {
    tasks = [...tasks, task];
  }

  $effect(() => {
    loadTasks(repoId);
  });
</script>

<div class="kanban-board">
  {#if loading}
    <div class="board-skeleton">
      {#each [1, 2, 3] as col (col)}
        <div class="skeleton-col">
          <div class="skeleton-col-header shimmer"></div>
          {#each [1, 2, 3] as card (card)}
            <div class="skeleton-card shimmer"></div>
          {/each}
        </div>
      {/each}
    </div>
  {/if}

  <div class="board-toolbar">
    <button
      class="archive-toggle"
      class:active={showArchived}
      onclick={() => { showArchived = !showArchived; }}
    >
      {showArchived ? 'Hide Archived' : 'Show Archived'}
    </button>
  </div>

  <div class="board-columns">
    <KanbanColumn
      title="TODO"
      status="todo"
      tasks={todoTasks}
      {repoId}
      onTaskCreated={onTaskCreated}
      onMoveTask={moveTask}
      onDeleteTask={deleteTask}
      onSelect={selectTask}
    />
    <KanbanColumn
      title="IN PROGRESS"
      status="in_progress"
      tasks={inProgressTasks}
      {repoId}
      onMoveTask={moveTask}
      onDeleteTask={deleteTask}
      onSelect={selectTask}
    />
    <KanbanColumn
      title="REVIEW"
      status="review"
      tasks={reviewTasks}
      {repoId}
      onMoveTask={moveTask}
      onDeleteTask={deleteTask}
      onSelect={selectTask}
    />
    {#if showArchived}
      <KanbanColumn
        title="DONE"
        status="done"
        tasks={doneTasks}
        {repoId}
        onMoveTask={moveTask}
        onDeleteTask={deleteTask}
        onSelect={selectTask}
      />
    {/if}
  </div>
</div>

<style>
  .kanban-board {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 16px;
    gap: 12px;
    overflow: hidden;
    position: relative;
  }

  .board-toolbar {
    display: flex;
    justify-content: flex-end;
    flex-shrink: 0;
  }

  .archive-toggle {
    background: var(--bg-card);
    border: 1px solid var(--border-light);
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: 12px;
    padding: 6px 12px;
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
  }

  .archive-toggle:hover,
  .archive-toggle.active {
    background: var(--bg-active);
    color: var(--text-primary);
  }

  .board-columns {
    display: flex;
    gap: 12px;
    flex: 1;
    overflow-x: auto;
    overflow-y: hidden;
  }

  .board-skeleton {
    display: flex;
    gap: 12px;
    padding-bottom: 12px;
    position: absolute;
    inset: 52px 16px 16px;
    pointer-events: none;
  }

  .skeleton-col {
    width: 260px;
    min-width: 260px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .skeleton-col-header {
    height: 40px;
    border-radius: 8px 8px 0 0;
  }

  .skeleton-card {
    height: 54px;
    border-radius: 6px;
  }

  @keyframes shimmer {
    0% { background-position: -400px 0; }
    100% { background-position: 400px 0; }
  }

  .shimmer {
    background: linear-gradient(
      90deg,
      var(--bg-card) 25%,
      var(--bg-hover) 50%,
      var(--bg-card) 75%
    );
    background-size: 800px 100%;
    animation: shimmer 1.4s ease-in-out infinite;
  }
</style>
