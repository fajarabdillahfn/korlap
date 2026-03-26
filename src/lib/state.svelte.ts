export interface ActiveTask {
  id: string;
  title: string;
  status: string;
  branch_name: string | null;
  worktree_path: string | null;
  diff: string | null;
}

export const appState = $state({
  activeRepoId: null as string | null,
  activeTaskId: null as string | null,
  activeTask: null as ActiveTask | null,
});
