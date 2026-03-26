<script lang="ts">
  import { appState } from '$lib/state.svelte';
  import KanbanBoard from '$lib/components/KanbanBoard.svelte';
  import WorkspaceView from '$lib/components/WorkspaceView.svelte';

  function handleGlobalKeydown(e: KeyboardEvent) {
    const target = e.target as HTMLElement;
    const inInput = target.tagName === 'TEXTAREA' || target.tagName === 'INPUT';

    // Escape: back to Kanban board (when in workspace and not typing)
    if (e.key === 'Escape' && !inInput && appState.activeTaskId) {
      appState.activeTaskId = null;
      appState.activeTask = null;
      return;
    }

    // Ctrl/Cmd+L: focus chat input
    if ((e.ctrlKey || e.metaKey) && e.key === 'l') {
      e.preventDefault();
      document.dispatchEvent(new CustomEvent('korlap:focus-chat'));
    }
  }
</script>

<svelte:window onkeydown={handleGlobalKeydown} />

<div class="workspace">
  {#if !appState.activeRepoId}
    <div class="empty-state">
      <p>Select a repository from the sidebar to get started.</p>
    </div>
  {:else}
    <!-- Keep KanbanBoard always mounted for O(1) switching -->
    <div class="view" class:hidden={!!appState.activeTaskId}>
      <KanbanBoard repoId={appState.activeRepoId} />
    </div>
    {#if appState.activeTask}
      <div class="view" class:hidden={!appState.activeTaskId}>
        <WorkspaceView task={appState.activeTask} />
      </div>
    {/if}
  {/if}
</div>

<style>
  .workspace {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .view {
    height: 100%;
    overflow: hidden;
  }

  .view.hidden {
    display: none;
  }

  .empty-state {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    font-size: 14px;
  }
</style>
