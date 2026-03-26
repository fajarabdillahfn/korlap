# Korlap Implementation Strategy

**Reference:** [PRD.md](/PRD.md)
**Approach:** Bottom-up, phase-gated. Each phase produces a working increment that can be verified before moving on.

---

## Phase 1: Project Scaffolding

> Goal: A Tauri v2 app that opens a styled empty window with custom titlebar.

| # | Task | Agent | Description |
|---|------|-------|-------------|
| 1.1 | Initialize Tauri v2 + Svelte 5 + Bun | `general-purpose` | `bun create tauri-app` with Svelte 5 + TypeScript template. Verify `bun tauri dev` opens a window. |
| 1.2 | Configure Tailwind CSS v4 | `general-purpose` | Install and configure Tailwind v4 with zero-config. Verify utility classes work in a test component. |
| 1.3 | Define theme tokens | `general-purpose` | Create CSS variables for the full color system from PRD §7.3 (`--bg-base`, `--accent`, etc.) with actual hex values in `app.css`. Import `Space Grotesk` and `JetBrains Mono` fonts. |
| 1.4 | Custom titlebar | `general-purpose` | Disable native titlebar in `tauri.conf.json`. Build a draggable custom titlebar component with minimize/maximize/close buttons using Tauri's window API. |

**Verify:** App launches with warm dark theme, custom titlebar, correct fonts.

---

## Phase 2: Database & Backend Core

> Goal: SQLite database with full CRUD accessible via Tauri IPC.

| # | Task | Agent | Description |
|---|------|-------|-------------|
| 2.1 | SQLite setup + migrations | `general-purpose` | Add `rusqlite` (or `sqlx` with SQLite) to Cargo dependencies. Create migration system. Write initial migration for `repos`, `tasks`, `messages` tables per PRD §9. |
| 2.2 | Repo IPC commands | `general-purpose` | Implement `list_repos`, `add_repo` Tauri commands. `add_repo` validates the path is a git repo (checks for `.git` directory). |
| 2.3 | Task IPC commands | `general-purpose` | Implement `list_tasks`, `create_task`, `update_task_status`, `delete_task` Tauri commands. `update_task_status` enforces forward-only transitions (`todo → in_progress → review → done`). `delete_task` cleans up worktree/branch/messages. |
| 2.4 | Message IPC commands | `general-purpose` | Implement message CRUD: insert message, list messages by task_id (ordered by `created_at`). |

**Verify:** Call each IPC command from the Svelte dev console. Data persists across app restarts.

---

## Phase 3: Sidebar & Repo Management

> Goal: Users can add/select repositories via the sidebar.

| # | Task | Agent | Description |
|---|------|-------|-------------|
| 3.1 | Sidebar layout | `general-purpose` | Build the `[Sidebar | Main Area]` shell layout. Sidebar shows repo list. Main area is a placeholder. |
| 3.2 | Add Repository flow | `general-purpose` | "Add Repository" button opens Tauri's native file picker dialog. On selection, invokes `add_repo`. Shows error toast if not a git repo. |
| 3.3 | Repo selection state | `general-purpose` | Clicking a repo in the sidebar sets it as the active repo. Store active repo in Svelte state (rune). Main area reacts to selection. |

**Verify:** Can add multiple repos, switch between them, repos persist after restart.

---

## Phase 4: Kanban Board (The Planner)

> Goal: Fully functional Kanban board with drag-and-drop and task lifecycle.

| # | Task | Agent | Description |
|---|------|-------|-------------|
| 4.1 | Kanban column layout | `general-purpose` | Four columns: `TODO`, `IN PROGRESS`, `REVIEW`, `DONE`. `DONE` column hidden by default with "Show Archived" toggle. Render task cards from `list_tasks`. |
| 4.2 | Create task | `general-purpose` | "Add Task" button/input at the top of the `TODO` column. Creates a task via `create_task` IPC. |
| 4.3 | Drag-and-drop | `general-purpose` | Implement drag-and-drop between columns. Enforce forward-only transitions (prevent dragging backwards). On drop to `IN PROGRESS`, show a modal prompting for branch name. |
| 4.4 | Task deletion | `general-purpose` | Delete button on task cards. Confirmation dialog. Calls `delete_task` IPC (handles worktree/branch cleanup on backend). |
| 4.5 | Show Archived toggle | `general-purpose` | Toggle button that reveals/hides the `DONE` column with completed tasks. |

**Verify:** Create tasks, drag through all columns, delete tasks, archive toggle works.

---

## Phase 5: Git Worktree Lifecycle

> Goal: Moving a task to `IN PROGRESS` creates an isolated worktree; `DONE` merges and cleans up.

| # | Task | Agent | Description |
|---|------|-------|-------------|
| 5.1 | Worktree creation | `general-purpose` | When `update_task_status` receives `in_progress` + `branch_name`: validate branch name (valid git ref, no duplicates), run `git worktree add ../.korlap-worktrees/<task_id> -b <branch_name>`, update task record with `worktree_path` and `branch_name`. |
| 5.2 | Worktree removal (DONE) | `general-purpose` | When status → `done`: run `git merge <branch_name>` in the main repo. On success: `git worktree remove`, archive task. On conflict: keep status as `review`, return error to frontend. |
| 5.3 | Worktree cleanup (delete) | `general-purpose` | When `delete_task` is called and worktree exists: force-remove worktree (`git worktree remove --force`), delete branch (`git branch -D`), delete messages, delete task. |
| 5.4 | Review diff capture | `general-purpose` | When status → `review`: run `git diff main...<branch_name>` and return the unified diff to the frontend for display. |

**Verify:** Create a task, move to IN PROGRESS (worktree appears on disk), make a change in worktree, move to REVIEW (diff shown), move to DONE (merged into main, worktree removed). Also verify delete cleans up.

---

## Phase 6: Terminal Integration

> Goal: Per-task xterm.js terminals that execute commands inside the task's worktree.

| # | Task | Agent | Description |
|---|------|-------|-------------|
| 6.1 | xterm.js setup | `general-purpose` | Install xterm.js + fit addon. Create a `Terminal` Svelte component that renders a terminal instance. |
| 6.2 | PTY backend | `general-purpose` | Implement PTY spawning in Rust using `portable-pty` or Tauri's shell API. Each PTY is scoped to a task's `worktree_path` as its cwd. Stream stdout/stderr via Tauri events (`terminal:stdout`, `terminal:stderr`). Handle stdin from frontend. |
| 6.3 | Per-task terminal tabs | `general-purpose` | Each task has a terminal panel with tab support. "New Tab" button spawns a new PTY in the same worktree. Terminal instances persist in DOM when switching tasks (`display: none`). |
| 6.4 | Terminal lifecycle | `general-purpose` | Emit `terminal:exit` event when process exits. Show exit code in UI. Clean up PTY resources when task is deleted. |

**Verify:** Open a task, type commands in terminal, output streams live. Switch tasks — terminal state preserved. Multiple tabs work.

---

## Phase 7: Chat UI & Claude API

> Goal: Users can chat with Claude, messages stream in real-time, history persists.

| # | Task | Agent | Description |
|---|------|-------|-------------|
| 7.1 | Chat panel UI | `general-purpose` | Build a chat panel component: message list (scrollable), input area at bottom. Render user/assistant/system messages with distinct styling. Support markdown rendering in assistant messages. |
| 7.2 | API key management | `general-purpose` | Settings screen or first-run prompt for the user's Anthropic API key. Store encrypted/securely (Tauri's secure storage or OS keychain). |
| 7.3 | Claude API client | `general-purpose` | Rust HTTP client (`reqwest`) that calls the Anthropic Messages API. Sends tool definitions (§11). Handles streaming responses (SSE). Emits `chat:stream_chunk` events to frontend as tokens arrive. |
| 7.4 | Message persistence | `general-purpose` | Save each user message and completed assistant response to the `messages` table. On task load, hydrate chat from DB. |
| 7.5 | Context window management | `general-purpose` | Before each API call: load all messages for the task, calculate approximate token count, trim oldest messages (preserving system prompt) if approaching the model's context limit. |
| 7.6 | Chat ↔ Task integration | `general-purpose` | `send_chat_message` IPC: accepts `task_id`, `content`, optional `file_contexts`. Constructs the full prompt (system prompt + history + file context + user message), calls Claude, streams response back. |

**Verify:** Send messages, see streaming response, reload app — history preserved. Long conversations get trimmed correctly.

---

## Phase 8: AI Tool Calling & Approval Flow

> Goal: Claude can propose tool calls, user approves/rejects, results feed back into the loop.

| # | Task | Agent | Description |
|---|------|-------|-------------|
| 8.1 | Tool call parsing | `general-purpose` | Parse `tool_use` blocks from Claude's streaming response. Emit `chat:tool_call` event with the tool name, parameters, and rationale. |
| 8.2 | Execution blocks UI | `general-purpose` | Render tool calls as actionable cards in the chat: show command/diff/file with `[ Approve & Run ]` and `[ Reject ]` buttons. Show rationale. |
| 8.3 | `execute_command` handler | `general-purpose` | On approval: run the command in the task's worktree via PTY. Capture output. Feed result back to Claude as a `tool_result`. |
| 8.4 | `apply_diff` handler | `general-purpose` | On approval: apply search-and-replace edit to the specified file in the worktree. Return success/failure to Claude. Show the diff in the chat. |
| 8.5 | `read_file` handler | `general-purpose` | On approval: read the file from the worktree. Return contents to Claude (truncated or full based on backend heuristic). |
| 8.6 | Agentic loop | `general-purpose` | After feeding a tool result back to Claude, continue the conversation. Claude may issue more tool calls. Loop continues until Claude responds with plain text (no tool calls) or the user stops it. Emit `chat:complete` when done. |
| 8.7 | Reject handling | `general-purpose` | On reject: send a `tool_result` with an error/rejection message back to Claude so it can adjust its approach. |

**Verify:** Claude proposes a command → approve → output shown → Claude responds. Claude proposes a diff → approve → file changed. Reject a tool call → Claude adjusts. Multi-step agentic loop works end-to-end.

---

## Phase 9: File Context System

> Goal: `@filename` autocomplete injects file content into the AI prompt.

| # | Task | Agent | Description |
|---|------|-------|-------------|
| 9.1 | `list_worktree_files` IPC | `general-purpose` | Backend command that lists files in the task's worktree. Supports a `query` parameter for fuzzy filtering. Respects `.gitignore`. |
| 9.2 | `@` autocomplete UI | `general-purpose` | Detect `@` typed in the chat input. Show a dropdown of matching files from the worktree (debounced search). On selection, insert `@filepath` tag into the input. |
| 9.3 | `read_file_content` IPC | `general-purpose` | Backend command that reads a file from the worktree. `truncate` flag controls whether to return truncated (backend-determined threshold) or full content. |
| 9.4 | Context injection | `general-purpose` | When `send_chat_message` includes `file_contexts`, the backend reads each file (truncated by default), and injects them into the prompt as labeled code blocks before the user message. |

**Verify:** Type `@` → see file list → select file → send message → Claude sees file content in context.

---

## Phase 10: Review & Merge UI

> Goal: Users can review diffs and trigger merge from the Kanban board.

| # | Task | Agent | Description |
|---|------|-------|-------------|
| 10.1 | Diff viewer component | `general-purpose` | Build a unified diff viewer component (syntax-highlighted, additions/deletions colored with theme tokens `--diff-add`, `--diff-del`). |
| 10.2 | Review panel | `general-purpose` | When a task is in `REVIEW`, show the diff viewer in the Orchestrator's main area. "Merge & Complete" button triggers the `update_task_status(done)` flow. |
| 10.3 | Merge conflict feedback | `general-purpose` | If merge fails due to conflicts, show an error message listing conflicted files. Task stays in `REVIEW`. User resolves via terminal. |

**Verify:** Move task to REVIEW → see diff → click Merge → task archived (or conflict error shown).

---

## Phase 11: Workspace Switching & Polish

> Goal: O(1) switching between tasks, Planner ↔ Orchestrator navigation, final UX polish.

| # | Task | Agent | Description |
|---|------|-------|-------------|
| 11.1 | O(1) workspace switching | `general-purpose` | Keep chat panels, terminals, and work view state in DOM with `display: none` when inactive. Clicking a task in the sidebar or Kanban board instantly shows its workspace. |
| 11.2 | Planner ↔ Orchestrator toggle | `general-purpose` | Navigation control (tabs or toggle) in the main area to switch between the Kanban board and the Work View for the active task. |
| 11.3 | Keyboard shortcuts | `general-purpose` | Define keyboard shortcuts: switch tasks, toggle Planner/Orchestrator, focus chat input, approve/reject tool calls. |
| 11.4 | Error toasts | `general-purpose` | Global toast notification system for errors (failed git operations, API errors, validation failures). Uses `--error` / `--error-bg` tokens. |
| 11.5 | Loading states | `general-purpose` | Skeleton loaders and spinners for: repo loading, task list fetch, chat streaming, worktree creation, merge operation. |

**Verify:** Full end-to-end flow: add repo → create task → move to IN PROGRESS → chat with Claude → approve tool calls → terminal usage → move to REVIEW → review diff → merge → done. All transitions are smooth.

---

## Dependency Graph

```
Phase 1 (Scaffolding)
  └── Phase 2 (Database & Backend)
        ├── Phase 3 (Sidebar & Repos)
        │     └── Phase 4 (Kanban Board)
        │           └── Phase 5 (Worktree Lifecycle)
        │                 ├── Phase 6 (Terminal)
        │                 ├── Phase 7 (Chat & Claude API)
        │                 │     └── Phase 8 (Tool Calling)
        │                 │           └── Phase 9 (File Context)
        │                 └── Phase 10 (Review & Merge)
        └───────────────────── Phase 11 (Polish — after all above)
```

Phases 6, 7, and 10 can be worked on in parallel once Phase 5 is complete. Phase 8 depends on both 6 and 7. Phase 9 depends on 8. Phase 11 is last.

---

## How to Use This Document

Each task is designed to be given to an AI agent as a self-contained prompt. Example:

```
Implement task 2.1: SQLite setup + migrations.
Reference PRD.md §9 for the schema.
Use rusqlite with bundled SQLite.
Create a migration system that runs on app startup.
Write the initial migration for repos, tasks, and messages tables.
```

For tasks that modify both Rust backend and Svelte frontend, the agent handles both in one session. For complex tasks, the agent should follow the TDD workflow (write tests first).
