<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open as openDialog } from '@tauri-apps/plugin-dialog';
  import { appState } from '$lib/state.svelte';

  interface Repo {
    id: string;
    name: string;
    root_path: string;
  }

  let repos = $state<Repo[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let addError = $state<string | null>(null);
  let adding = $state(false);

  async function loadRepos() {
    loading = true;
    try {
      repos = await invoke<Repo[]>('list_repos');
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function addRepo() {
    addError = null;
    const selected = await openDialog({ directory: true, multiple: false });
    if (!selected || Array.isArray(selected)) return;

    adding = true;
    try {
      const repo = await invoke<Repo>('add_repo', { path: selected });
      repos = [...repos, repo].sort((a, b) => a.name.localeCompare(b.name));
      appState.activeRepoId = repo.id;
    } catch (e) {
      addError = String(e);
    } finally {
      adding = false;
    }
  }

  $effect(() => { loadRepos(); });
</script>

<aside class="sidebar">
  <div class="sidebar-header">
    <span class="sidebar-label">Repositories</span>
  </div>

  <div class="sidebar-repos">
    {#if loading}
      <div class="skeleton-list">
        {#each [1, 2, 3] as i (i)}
          <div class="skeleton-repo shimmer"></div>
        {/each}
      </div>
    {:else if error}
      <p class="sidebar-error">{error}</p>
    {:else if repos.length === 0}
      <p class="sidebar-hint">No repositories yet.</p>
    {:else}
      {#each repos as repo (repo.id)}
        <button
          class="repo-item"
          class:active={appState.activeRepoId === repo.id}
          onclick={() => { appState.activeRepoId = repo.id; }}
          title={repo.root_path}
        >
          <span class="repo-name">{repo.name}</span>
        </button>
      {/each}
    {/if}
  </div>

  {#if addError}
    <p class="sidebar-error add-error">{addError}</p>
  {/if}

  <div class="sidebar-footer">
    <button class="add-repo-btn" onclick={addRepo} disabled={adding}>
      {adding ? 'Adding…' : '+ Add Repository'}
    </button>
  </div>
</aside>

<style>
  .sidebar {
    width: 220px;
    min-width: 220px;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar-header {
    padding: 16px 12px 8px;
    border-bottom: 1px solid var(--border);
  }

  .sidebar-label {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .sidebar-repos {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }

  .repo-item {
    display: block;
    width: 100%;
    padding: 8px 12px;
    background: transparent;
    border: none;
    text-align: left;
    cursor: pointer;
    color: var(--text-secondary);
    border-radius: 0;
    transition: background 0.1s, color 0.1s;
  }

  .repo-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .repo-item.active {
    background: var(--bg-active);
    color: var(--text-primary);
  }

  .repo-name {
    font-size: 13px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: block;
  }

  .sidebar-hint {
    padding: 12px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .skeleton-list {
    padding: 8px 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .skeleton-repo {
    height: 32px;
    border-radius: 4px;
  }

  @keyframes shimmer {
    0% { background-position: -200px 0; }
    100% { background-position: 200px 0; }
  }

  .shimmer {
    background: linear-gradient(
      90deg,
      var(--bg-hover) 25%,
      var(--bg-active) 50%,
      var(--bg-hover) 75%
    );
    background-size: 400px 100%;
    animation: shimmer 1.2s ease-in-out infinite;
  }

  .sidebar-error {
    padding: 8px 12px;
    font-size: 12px;
    color: var(--error);
  }

  .add-error {
    border-top: 1px solid var(--border);
  }

  .sidebar-footer {
    padding: 8px;
    border-top: 1px solid var(--border);
  }

  .add-repo-btn {
    width: 100%;
    padding: 8px;
    background: var(--bg-card);
    border: 1px solid var(--border-light);
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: 12px;
    cursor: pointer;
    transition: background 0.1s, color 0.1s, border-color 0.1s;
  }

  .add-repo-btn:hover:not(:disabled) {
    background: var(--accent);
    color: var(--bg-base);
    border-color: var(--accent);
  }

  .add-repo-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
