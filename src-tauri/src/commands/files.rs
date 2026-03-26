use crate::commands::tools::read_file_from_worktree;
use crate::state::AppState;
use rusqlite::params;
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

#[tauri::command]
pub fn list_worktree_files(
    task_id: String,
    query: String,
    state: State<AppState>,
) -> Result<Vec<String>, String> {
    let worktree_path = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        get_worktree_path(&db, &task_id)?
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

    let q = query.to_lowercase();
    let results: Vec<String> = if q.is_empty() {
        all_files.into_iter().take(20).collect()
    } else {
        all_files
            .into_iter()
            .filter(|f| f.to_lowercase().contains(&q))
            .take(20)
            .collect()
    };

    Ok(results)
}

#[tauri::command]
pub fn read_file_content(
    task_id: String,
    file_path: String,
    full: bool,
    state: State<AppState>,
) -> Result<String, String> {
    let worktree_path = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        get_worktree_path(&db, &task_id)?
    };

    read_file_from_worktree(&worktree_path, &file_path, full)
}
