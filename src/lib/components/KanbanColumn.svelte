<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { showToast } from '$lib/toast.svelte';
  import KanbanCard from './KanbanCard.svelte';

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
    title,
    status,
    tasks,
    repoId,
    onTaskCreated,
    onMoveTask,
    onDeleteTask,
    onSelect,
  }: {
    title: string;
    status: string;
    tasks: Task[];
    repoId: string;
    onTaskCreated: ((task: Task) => void) | undefined;
    onMoveTask: (taskId: string, newStatus: string, branchName?: string) => Promise<void>;
    onDeleteTask: (taskId: string) => void;
    onSelect: (task: Task) => void;
  } = $props();

  let newTaskTitle = $state('');
  let adding = $state(false);

  async function addTask() {
    const trimmed = newTaskTitle.trim();
    if (!trimmed) return;

    adding = true;
    try {
      const task = await invoke<Task>('create_task', {
        repo_id: repoId,
        title: trimmed,
      });
      onTaskCreated?.(task);
      newTaskTitle = '';
    } catch (e) {
      showToast(String(e));
    } finally {
      adding = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') addTask();
  }
</script>

<div class="column">
  <div class="column-header">
    <span class="column-title">{title}</span>
    <span class="column-count">{tasks.length}</span>
  </div>

  <div class="column-cards">
    {#each tasks as task (task.id)}
      <KanbanCard
        {task}
        {onMoveTask}
        {onDeleteTask}
        {onSelect}
      />
    {/each}
  </div>

  {#if status === 'todo'}
    <div class="add-task-area">
      <input
        class="add-task-input"
        type="text"
        placeholder="Add a task…"
        bind:value={newTaskTitle}
        onkeydown={handleKeydown}
        disabled={adding}
      />
      <button class="add-task-btn" onclick={addTask} disabled={adding || !newTaskTitle.trim()}>
        Add
      </button>
    </div>
  {/if}
</div>

<style>
  .column {
    width: 260px;
    min-width: 260px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .column-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px 10px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .column-title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .column-count {
    font-size: 11px;
    color: var(--text-muted);
    background: var(--bg-hover);
    border-radius: 10px;
    padding: 1px 7px;
  }

  .column-cards {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .add-task-area {
    display: flex;
    gap: 6px;
    padding: 8px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .add-task-input {
    flex: 1;
    background: var(--bg-base);
    border: 1px solid var(--border-light);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 12px;
    padding: 6px 8px;
    outline: none;
    transition: border-color 0.1s;
  }

  .add-task-input::placeholder {
    color: var(--text-muted);
  }

  .add-task-input:focus {
    border-color: var(--accent);
  }

  .add-task-btn {
    background: var(--accent);
    border: none;
    border-radius: 6px;
    color: var(--bg-base);
    font-size: 12px;
    font-weight: 600;
    padding: 6px 12px;
    cursor: pointer;
    transition: opacity 0.1s;
  }

  .add-task-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
