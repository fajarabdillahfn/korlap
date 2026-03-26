use crate::commands::tools::read_file_from_worktree;
use crate::state::AppState;
use rusqlite::params;
use serde::Deserialize;
use std::process::Command;
use tauri::State;

fn get_worktree_path(db: &rusqlite::Connection, task_id: &str) -> Result<String, String> {
    db.query_row(
        "SELECT worktree_path FROM tasks WHERE id = ?1",
        params![task_id],
        |row| row.get::<_, Option<String>>(0),
    )
    .map_err(|e| e.to_string())?
    .filter(|p| std::path::Path::new(p).is_dir())
    .ok_or_else(|| format!("Task {} has no active worktree", task_id))
}

#[derive(Deserialize)]
pub struct ListWorktreeFilesPayload {
    pub task_id: String,
    pub query: String,
}

/// List files tracked by git in the task's worktree.
/// Respects .gitignore automatically (git ls-files only shows tracked/staged files).
/// Returns at most 20 matches.
#[tauri::command]
pub fn list_worktree_files(
    payload: ListWorktreeFilesPayload,
    state: State<AppState>,
) -> Result<Vec<String>, String> {
    let worktree_path = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        get_worktree_path(&db, &payload.task_id)?
    };

    let output = Command::new("git")
        .args(["ls-files"])
        .current_dir(&worktree_path)
        .output()
        .map_err(|e| e.to_string())?;

    let all_files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    let query = payload.query.to_lowercase();
    let results: Vec<String> = if query.is_empty() {
        all_files.into_iter().take(20).collect()
    } else {
        all_files
            .into_iter()
            .filter(|f| f.to_lowercase().contains(&query))
            .take(20)
            .collect()
    };

    Ok(results)
}

#[derive(Deserialize)]
pub struct ReadFileContentPayload {
    pub task_id: String,
    pub file_path: String,
    pub full: bool,
}

/// Read a file from the task's worktree. Uses the same path-security logic as the tool handler.
#[tauri::command]
pub fn read_file_content(
    payload: ReadFileContentPayload,
    state: State<AppState>,
) -> Result<String, String> {
    let worktree_path = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        get_worktree_path(&db, &payload.task_id)?
    };

    read_file_from_worktree(&worktree_path, &payload.file_path, payload.full)
}
