# Korlap

A locally-hosted, AI-augmented developer workspace that unifies project planning (Kanban) with AI-driven code execution — all on your machine, all under your control.

## What it does

Korlap combines a Kanban board with an AI orchestrator in a single desktop app. Each task gets its own isolated `git worktree`, its own chat thread with Claude, and its own terminal — switching between them is instant.

**The Planner** — Kanban board with four columns (`TODO → IN PROGRESS → REVIEW → DONE`). Moving a task to IN PROGRESS prompts for a branch name and automatically provisions a dedicated git worktree. Multiple tasks can be in progress simultaneously.

**The Orchestrator** — Per-task workspace with:
- Chat with Claude (streaming, history persisted in SQLite)
- `@filename` autocomplete to inject file contents into the AI prompt
- Tool call approval flow — Claude proposes commands/edits, you approve or reject each one
- Per-task terminal tabs scoped to the worktree
- Diff viewer and one-click merge when a task moves to REVIEW

## Tech stack

| Layer | Technology |
|---|---|
| Desktop shell | Tauri v2 |
| Backend | Rust (Tauri IPC commands) |
| Frontend | Svelte 5 (Runes) + TypeScript |
| Build | Bun |
| Database | SQLite (embedded, Rust-only access) |
| Terminal | xterm.js + portable-pty |
| AI | Anthropic Claude API (user-provided key) |

## Getting started

### Prerequisites

- [Rust](https://rustup.rs/)
- [Bun](https://bun.sh/)
- [Git](https://git-scm.com/)
- Tauri v2 prerequisites for your OS — see [tauri.app/start/prerequisites](https://tauri.app/start/prerequisites/)

#### Windows

- Install [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the "Desktop development with C++" workload
- WebView2 is pre-installed on Windows 10/11 (ships with Edge)

#### Linux

- Install WebKitGTK — see the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your distro
- **WSL is not supported** — WebKitGTK has rendering issues under WSLg. Build on Windows natively instead.

### Install and run

```sh
bun install
bun tauri dev
```

### Build for production

```sh
bun tauri build
```

## Configuration

On first launch, go to **Settings** and paste your [Anthropic API key](https://console.anthropic.com/). The key is stored locally and never leaves your machine.

## Keyboard shortcuts

| Shortcut | Action |
|---|---|
| `Esc` | Back to Kanban board |
| `Ctrl+L` | Focus chat input |
| `A` | Approve first pending tool call |
| `X` | Reject first pending tool call |
| `Enter` | Send chat message |
| `Shift+Enter` | New line in chat |
| `@` | Open file autocomplete in chat |

## Design principles

- **Privacy first** — all data stays local; no telemetry
- **Human-in-the-loop** — Claude never runs commands or writes files without your approval
- **Parallel isolation** — each task is a separate git worktree; no cross-contamination
- **O(1) switching** — chat panels and terminals stay in the DOM, switching tasks is instant
