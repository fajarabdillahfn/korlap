# Product Requirements Document (PRD)
**Project:** Local AI Orchestrator & Kanban Workspace (Korlap Architecture)
**Status:** Draft / MVP Planning

## 1. Product Overview
A locally-hosted, AI-augmented developer workspace that unifies project planning (Kanban) with execution (AI-driven code generation and terminal operations). It serves as a personal work tool designed to reduce cognitive load by keeping context, planning, and coding within a single, unified interface.

## 2. Core Philosophy & Constraints
* **Privacy First:** All project data, git history, and Kanban tasks remain local.
* **Parallel Isolation:** Utilizes `git worktree` to ensure multiple AI agents can run tasks, compile code, and edit files simultaneously without cross-contamination. No limit on concurrent tasks.
* **Human-in-the-Loop:** The AI acts as an advisor and tool-user, but never executes terminal commands or writes to the file system without explicit user approval.
* **Mobile-Ready Foundation:** The architecture is designed for a future remote Android companion app, but the MVP uses Tauri IPC exclusively. REST API and WebSocket layers will be introduced in the mobile companion phase.
* **API-Driven AI:** Utilizes the Anthropic Claude API via user-provided API keys for MVP. Additional providers (e.g., Google Gemini) may be added post-MVP.

## 3. Technology Stack
* **Desktop Shell:** Tauri v2 (cross-platform, native WebView, lightweight, native PTY control).
* **Backend (Server):** Rust (Tauri core) using Tauri IPC commands for frontend-backend communication.
* **Frontend (Client):** Svelte 5 (using Runes for minimal reactivity cost) + TypeScript.
* **Runtime & Build:** Bun (fast installs, built-in TS).
* **Styling:** Tailwind CSS v4 (Zero config).
* **Terminal Emulator:** xterm.js.
* **Database:** Local SQLite (embedded via Tauri/Rust). All database access is exclusively through the Rust backend; the frontend never queries SQLite directly.

## 4. Key Features (MVP)
### 4.1. The Planner (Kanban View)
* Standard columns: `TODO`, `IN PROGRESS`, `REVIEW`, `DONE`.
* Drag-and-drop functionality.
* Moving a task to `IN PROGRESS` prompts the user to provide a branch name, then automatically provisions an isolated `git worktree` environment for that specific task.
* Multiple tasks can be `IN PROGRESS` simultaneously with no limit.
* **Status transitions are forward-only:** `TODO → IN PROGRESS → REVIEW → DONE`. Tasks cannot move backwards (e.g., no `REVIEW → IN PROGRESS`).
* **Task deletion:** Users can delete a task at any status. Deletion cleans up the associated worktree and branch (if they exist), removes all related messages, and removes the task from the board.
* **Archival:** `DONE` tasks are hidden from the board by default. A "Show Archived" toggle reveals completed tasks.

### 4.2. The Orchestrator (Work View)
* **O(1) Workspace Switching:** Chat panels and terminals stay in the DOM (`display: none`) when switching tasks, ensuring instant transitions without data refetching.
* **File Context System:** Users can type `@filename` to manually inject specific file contents into the AI's prompt context. An autocomplete dropdown searches the task's worktree file tree. File content is truncated by default (threshold determined by the backend based on file size/type); if the AI needs more, it requests the full file via `read_file`.
* **Execution Blocks:** Actionable UI elements displaying AI-proposed commands (e.g., `npx svelte-check`) with `[ Approve & Run ]` and `[ Reject ]` buttons.
* **Per-Task Terminals:** Each task has its own terminal instance(s) scoped to its worktree. Users can open multiple terminal tabs per task.

### 4.3. Backend Capabilities
* **Worktree Management:** Backend safely creates, routes commands to, and destroys hidden Git worktrees linked to specific task IDs.
* **Terminal Execution:** Safely spawn child processes within a specific worktree and stream `stdout`/`stderr` back to the UI via Tauri events.

## 5. Data Model (High-Level)
* **Repos:** Root directories for git repositories. Added via a file picker with git repository validation.
* **Tasks:** Individual Kanban cards linked to a specific `git worktree`.
* **Messages:** Chat history, including system prompts, AI responses, and tool-call JSON metadata. Persisted across app restarts with no automatic cleanup. When sending context to the Claude API, the backend includes all messages for the active task but trims older messages when approaching token limits.

## 6. Future Scope (Post-MVP)
* **Android Companion App:** A mobile interface connecting securely to the desktop's local server via a P2P tunnel (e.g., Tailscale) to manage tasks and approve AI actions remotely.
* **REST API & WebSocket Layer:** HTTP server (e.g., Axum) running alongside Tauri to serve the mobile companion app.
* **Additional AI Providers:** Google Gemini and other LLM providers.
* **Merge Conflict Resolution UI:** In-app conflict resolution tooling for the `REVIEW → DONE` transition.
* **Backward Status Transitions:** Allow moving tasks backwards (e.g., `REVIEW → IN PROGRESS`) for iterative workflows.

## 7. UI/UX Design Specifications & Theme Tokens
The interface must feel native, snappy, and heavily keyboard-optimized.

### 7.1. Layout Structure
The app uses a `[Sidebar | Main Area]` layout:
* **Sidebar:** Repository list, workspace navigation.
* **Main Area:** Switches between the **Planner** (Kanban board) and the **Orchestrator** (Work View with chat, terminals, and file context).

### 7.2. Identity & Typography
* **Typeface:** `Space Grotesk` for UI elements (geometric, purposeful). `JetBrains Mono` or `Fira Code` for terminal and code blocks.
* **Title Bar:** Minimalist. No explicit app name text in the title bar; the app identifies itself through aesthetics. Custom window controls.

### 7.3. Color System (Warm Dark Theme)
All blacks must be tinted with amber; avoid cold grays. **Never hardcode hex values**; strictly use CSS variables (Tailwind v4 tokens).

| Token Name | Usage |
| :--- | :--- |
| `--bg-base` | Main application background (deep amber-black). |
| `--bg-sidebar` | Left navigation/workspace list background. |
| `--bg-titlebar` | Top draggable area. |
| `--bg-card` | Kanban cards, UI panels, code blocks. |
| `--bg-hover` / `--bg-active` | Interactive element states. |
| `--border` / `--border-light` | Dividers and panel outlines. |
| `--text-primary` | Main reading text. |
| `--text-secondary` / `--text-muted` | Metadata, timestamps, placeholders. |
| `--accent` | Primary action buttons, active states (Warm amber/gold). |
| `--status-ok` | Success states (Muted olive/green). |
| `--error` / `--error-bg` | Failures, rejected actions. |
| `--diff-add` / `--diff-add-bg` | Git additions (Soft green). |
| `--diff-del` / `--diff-del-bg` | Git deletions (Soft red). |

## 8. Communication Architecture (MVP: Tauri IPC)
The MVP uses Tauri IPC commands and events exclusively. All frontend-backend communication flows through `invoke()` for request/response and Tauri event listeners for streaming.

### 8.1. Tauri IPC Commands (Request/Response)
| Command | Description | Parameters |
| :--- | :--- | :--- |
| `list_repos` | List all local tracked repositories. | - |
| `add_repo` | Register a new git repository via file picker. | `{ path: string }` |
| `list_tasks` | List all Kanban cards for a repo. | `{ repo_id: string }` |
| `create_task` | Create a new Kanban card. | `{ repo_id: string, title: string }` |
| `update_task_status` | Move a card forward (triggers worktree lifecycle). | `{ task_id: string, status: string, branch_name?: string }` |
| `delete_task` | Delete a task and clean up worktree/branch. | `{ task_id: string }` |
| `execute_patch` | Apply a unified diff within a task's worktree. | `{ task_id: string, file_path: string, diff: string }` |
| `execute_shell` | Spawn a child process in the task's worktree. | `{ task_id: string, cmd: string }` |
| `send_chat_message` | Send user prompt with optional file context. | `{ task_id: string, content: string, file_contexts?: FileContext[] }` |
| `read_file_content` | Read a file from the task's worktree. | `{ task_id: string, file_path: string, truncate?: boolean }` |
| `list_worktree_files` | List files in a task's worktree for `@` autocomplete. | `{ task_id: string, query?: string }` |

### 8.2. Tauri Events (Streaming / Push)
| Event | Direction | Description |
| :--- | :---: | :--- |
| `chat:stream_chunk` | Backend → Frontend | LLM text generation streaming. |
| `chat:tool_call` | Backend → Frontend | Pushes tool call JSON to trigger UI approval block. |
| `chat:complete` | Backend → Frontend | Signals the AI has finished its response (no more tool calls). |
| `terminal:stdout` | Backend → Frontend | Live streaming of terminal stdout. |
| `terminal:stderr` | Backend → Frontend | Live streaming of terminal stderr. |
| `terminal:exit` | Backend → Frontend | Terminal process exited with status code. |

## 9. Database Schema (SQLite)

**Table: `repos`**
* `id` (UUID, PK), `name` (String), `root_path` (String, unique).

**Table: `tasks`**
* `id` (UUID, PK), `repo_id` (UUID, FK → repos), `title` (String), `status` (String, default: `todo`), `branch_name` (String, Nullable — set by user when moving to `IN PROGRESS`), `worktree_path` (String, Nullable), `created_at` (Timestamp), `updated_at` (Timestamp).

**Table: `messages`**
* `id` (UUID, PK), `task_id` (UUID, FK → tasks), `role` (String: `user` | `assistant` | `system`), `content` (Text), `tool_calls` (JSON, Nullable), `created_at` (Timestamp).

## 10. Git Operations Flow (The Worktree Lifecycle)
This is the core mechanic allowing true parallel agent execution.

* **Initialization (`TODO` → `IN PROGRESS`):**
  1. User provides a branch name (e.g., `feat/toast-color`).
  2. Backend validates the branch name (no duplicates, valid git ref).
  3. Backend executes: `git worktree add ../.korlap-worktrees/<task_id> -b <branch_name>`.
  4. The task is now securely isolated.
* **Execution (While `IN PROGRESS`):**
  * All AI tool calls (`read_file`, `apply_diff`, `execute_command`) are forcefully routed to execute *only* inside `../.korlap-worktrees/<task_id>`. The main repository remains untouched.
  * The agentic loop continues until the user decides the task is done — there is no automatic iteration limit.
* **Review Preparation (`IN PROGRESS` → `REVIEW`):**
  * System captures the diff from the worktree against the main branch. User reviews changes in the unified UI.
* **Finalization (`REVIEW` → `DONE`):**
  1. System executes `git merge` using git's default merge strategy (typically recursive/ort with a merge commit).
  2. If the merge succeeds: executes `git worktree remove ../.korlap-worktrees/<task_id>` to delete the isolated folder, then visually archives the Kanban card.
  3. If the merge has conflicts: the task remains in `REVIEW` status. The user must resolve conflicts manually (via terminal or external tool). A built-in conflict resolution UI is planned for post-MVP.
* **Deletion (Any Status):**
  1. If a worktree exists: executes `git worktree remove ../.korlap-worktrees/<task_id>` (force if needed).
  2. If a branch was created: deletes the branch (`git branch -D <branch_name>`).
  3. Deletes all associated messages from the database.
  4. Removes the task record.

## 11. AI Tool Calling Schema
The backend intercepts these JSON requests from the Claude API and presents them to the user for approval.

* **`execute_command`:** Run a CLI command. Parameters: `command` (string), `rationale` (string). *Backend strictly executes this in the task's worktree path.*
* **`apply_diff`:** Apply a search-and-replace edit. Parameters: `file_path` (string), `search_block` (string), `replace_block` (string), `rationale` (string).
* **`read_file`:** Read file contents. Parameters: `file_path` (string), `rationale` (string).

## 12. Repo Registration Flow
1. User clicks "Add Repository" in the sidebar.
2. A native file picker dialog opens (via Tauri's dialog API).
3. User selects a directory.
4. Backend validates:
   * The directory exists.
   * The directory is a valid git repository (contains `.git`).
5. If valid: a new `repos` row is created and the repo appears in the sidebar.
6. If invalid: an error message is shown (e.g., "Selected directory is not a git repository").