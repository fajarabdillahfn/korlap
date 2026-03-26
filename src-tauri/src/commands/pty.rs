use crate::pty::{PtyMap, spawn_pty};
use crate::state::AppState;
use rusqlite::params;
use std::io::Write;
use tauri::{AppHandle, State};

#[derive(serde::Deserialize)]
pub struct PtyCreatePayload {
    pub task_id: String,
    pub terminal_id: String,
}

#[tauri::command]
pub fn pty_create(
    payload: PtyCreatePayload,
    app: AppHandle,
    state: State<AppState>,
    pty_map: State<PtyMap>,
) -> Result<(), String> {
    // Look up worktree_path for this task
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let worktree_path: Option<String> = db
        .query_row(
            "SELECT worktree_path FROM tasks WHERE id = ?1",
            params![payload.task_id],
            |row| row.get(0),
        )
        .map_err(|_| "Task not found".to_string())?;

    let cwd = worktree_path
        .filter(|p| std::path::Path::new(p).is_dir())
        .ok_or_else(|| "Worktree path not set or does not exist".to_string())?;

    drop(db); // Release DB lock before spawning PTY

    spawn_pty(app, pty_map, payload.terminal_id, cwd)
}

#[derive(serde::Deserialize)]
pub struct PtyWritePayload {
    pub terminal_id: String,
    pub data: String,
}

#[tauri::command]
pub fn pty_write(payload: PtyWritePayload, pty_map: State<PtyMap>) -> Result<(), String> {
    // Clone the writer Arc so we release the PtyMap lock before doing blocking I/O.
    let writer = {
        let map = pty_map.lock().map_err(|e| e.to_string())?;
        let session = map
            .get(&payload.terminal_id)
            .ok_or_else(|| format!("No terminal: {}", payload.terminal_id))?;
        std::sync::Arc::clone(&session.writer)
    }; // PtyMap lock released here
    let mut guard = writer.lock().map_err(|e| e.to_string())?;
    guard
        .write_all(payload.data.as_bytes())
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct PtyResizePayload {
    pub terminal_id: String,
    pub cols: u16,
    pub rows: u16,
}

#[tauri::command]
pub fn pty_resize(_payload: PtyResizePayload, _pty_map: State<PtyMap>) -> Result<(), String> {
    // portable-pty resize requires master handle; store master in session if needed.
    // For MVP, return Ok — resize support can be added later.
    Ok(())
}

#[derive(serde::Deserialize)]
pub struct PtyKillPayload {
    pub terminal_id: String,
}

#[tauri::command]
pub fn pty_kill(payload: PtyKillPayload, pty_map: State<PtyMap>) -> Result<(), String> {
    let mut map = pty_map.lock().map_err(|e| e.to_string())?;
    if let Some(mut session) = map.remove(&payload.terminal_id) {
        let _ = session.child.kill();
    }
    Ok(())
}
