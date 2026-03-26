use crate::pty::{PtyMap, spawn_pty};
use crate::state::AppState;
use rusqlite::params;
use std::io::Write;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn pty_create(
    task_id: String,
    terminal_id: String,
    app: AppHandle,
    state: State<AppState>,
    pty_map: State<PtyMap>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let worktree_path: Option<String> = db
        .query_row(
            "SELECT worktree_path FROM tasks WHERE id = ?1",
            params![task_id],
            |row| row.get(0),
        )
        .map_err(|_| "Task not found".to_string())?;

    let cwd = worktree_path
        .filter(|p| std::path::Path::new(p).is_dir())
        .ok_or_else(|| "Worktree path not set or does not exist".to_string())?;

    drop(db);

    spawn_pty(app, pty_map, terminal_id, cwd)
}

#[tauri::command]
pub fn pty_write(terminal_id: String, data: String, pty_map: State<PtyMap>) -> Result<(), String> {
    let writer = {
        let map = pty_map.lock().map_err(|e| e.to_string())?;
        let session = map
            .get(&terminal_id)
            .ok_or_else(|| format!("No terminal: {}", terminal_id))?;
        std::sync::Arc::clone(&session.writer)
    };
    let mut guard = writer.lock().map_err(|e| e.to_string())?;
    guard.write_all(data.as_bytes()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pty_resize(
    _terminal_id: String,
    _cols: u16,
    _rows: u16,
    _pty_map: State<PtyMap>,
) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn pty_kill(terminal_id: String, pty_map: State<PtyMap>) -> Result<(), String> {
    let mut map = pty_map.lock().map_err(|e| e.to_string())?;
    if let Some(mut session) = map.remove(&terminal_id) {
        let _ = session.child.kill();
    }
    Ok(())
}
