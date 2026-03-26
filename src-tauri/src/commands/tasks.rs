use crate::state::AppState;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: String,
    pub repo_id: String,
    pub title: String,
    pub status: String,
    pub branch_name: Option<String>,
    pub worktree_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskWithDiff {
    pub task: Task,
    pub diff: Option<String>, // Only set when transitioning to 'review'
}

#[tauri::command]
pub fn list_tasks(repo_id: String, state: State<AppState>) -> Result<Vec<Task>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT id, repo_id, title, status, branch_name, worktree_path, created_at, updated_at
             FROM tasks WHERE repo_id = ?1 ORDER BY created_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let tasks = stmt
        .query_map(params![repo_id], |row| {
            Ok(Task {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                title: row.get(2)?,
                status: row.get(3)?,
                branch_name: row.get(4)?,
                worktree_path: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(tasks)
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskPayload {
    pub repo_id: String,
    pub title: String,
}

#[tauri::command]
pub fn create_task(payload: CreateTaskPayload, state: State<AppState>) -> Result<Task, String> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO tasks (id, repo_id, title, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'todo', ?4, ?5)",
        params![id, payload.repo_id, payload.title, now, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(Task {
        id,
        repo_id: payload.repo_id,
        title: payload.title,
        status: "todo".to_string(),
        branch_name: None,
        worktree_path: None,
        created_at: now.clone(),
        updated_at: now,
    })
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskStatusPayload {
    pub task_id: String,
    pub status: String,
    pub branch_name: Option<String>,
}

#[tauri::command]
pub fn update_task_status(
    payload: UpdateTaskStatusPayload,
    state: State<AppState>,
) -> Result<TaskWithDiff, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Fetch current task + repo root_path in one join
    let current: (String, Option<String>, String) = db
        .query_row(
            "SELECT t.status, t.branch_name, r.root_path
             FROM tasks t JOIN repos r ON t.repo_id = r.id
             WHERE t.id = ?1",
            params![payload.task_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => "Task not found".to_string(),
            other => other.to_string(),
        })?;

    let current_status = current.0.as_str();
    let repo_root_path = current.2;

    // Validate forward-only transition
    let valid = matches!(
        (current_status, payload.status.as_str()),
        ("todo", "in_progress") | ("in_progress", "review") | ("review", "done")
    );
    if !valid {
        return Err(format!(
            "Invalid status transition: {} → {}",
            current_status, payload.status
        ));
    }

    let now = chrono::Utc::now().to_rfc3339();

    if payload.status == "in_progress" {
        let branch_name = payload
            .branch_name
            .as_deref()
            .filter(|s| !s.is_empty())
            .ok_or("branch_name is required when moving to in_progress")?;

        // Build absolute worktree path: <repo_parent>/.korlap-worktrees/<task_id>
        let repo_root = std::path::PathBuf::from(&repo_root_path);
        let worktree_base = repo_root
            .parent()
            .unwrap_or(&repo_root)
            .join(".korlap-worktrees");
        let worktree_path = worktree_base
            .join(&payload.task_id)
            .to_string_lossy()
            .to_string();

        db.execute(
            "UPDATE tasks SET status = ?1, branch_name = ?2, worktree_path = ?3, updated_at = ?4
             WHERE id = ?5",
            params![payload.status, branch_name, worktree_path, now, payload.task_id],
        )
        .map_err(|e| e.to_string())?;

        // Create worktree directory parent
        let worktree_parent = std::path::Path::new(&worktree_path).parent();
        if let Some(parent) = worktree_parent {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        // Create git worktree
        if let Err(git_err) = crate::git::run_git(&repo_root_path, &[
            "worktree", "add", &worktree_path, "-b", branch_name,
        ]) {
            // Roll back DB update — surface rollback failure alongside git error
            let rollback_note = db.execute(
                "UPDATE tasks SET status = 'todo', branch_name = NULL, worktree_path = NULL, updated_at = ?1 WHERE id = ?2",
                params![chrono::Utc::now().to_rfc3339(), payload.task_id],
            ).err().map(|e| format!(" (rollback also failed: {})", e)).unwrap_or_default();
            return Err(format!("Failed to create git worktree: {}{}", git_err, rollback_note));
        }
    } else {
        db.execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![payload.status, now, payload.task_id],
        )
        .map_err(|e| e.to_string())?;
    }

    // Capture diff when transitioning to 'review'
    let diff = if payload.status == "review" {
        let branch_name: Option<String> = db.query_row(
            "SELECT branch_name FROM tasks WHERE id = ?1",
            params![payload.task_id],
            |row| row.get(0),
        ).ok().flatten();

        if let Some(ref branch) = branch_name {
            crate::git::run_git(&repo_root_path, &[
                "diff", &format!("HEAD...{}", branch),
            ]).ok()
        } else {
            None
        }
    } else {
        None
    };

    // Worktree + branch cleanup when transitioning to 'done'
    if payload.status == "done" {
        let (branch_name, worktree_path_opt): (Option<String>, Option<String>) = db.query_row(
            "SELECT branch_name, worktree_path FROM tasks WHERE id = ?1",
            params![payload.task_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|e| e.to_string())?;

        if let (Some(ref branch), Some(ref wt_path)) = (&branch_name, &worktree_path_opt) {
            // Guard: ensure repo is on a named branch (not detached HEAD)
            let head_ref = crate::git::run_git(&repo_root_path, &[
                "rev-parse", "--abbrev-ref", "HEAD",
            ]).map_err(|e| format!("Cannot determine current branch: {}", e))?;
            if head_ref == "HEAD" {
                return Err("Cannot merge: repository is in detached HEAD state. Check out a branch first.".to_string());
            }

            // Merge the branch into current HEAD
            if let Err(merge_err) = crate::git::run_git(&repo_root_path, &[
                "merge", branch,
            ]) {
                // Merge conflict or failure — roll back to 'review'
                let _ = db.execute(
                    "UPDATE tasks SET status = 'review', updated_at = ?1 WHERE id = ?2",
                    params![chrono::Utc::now().to_rfc3339(), payload.task_id],
                );
                return Err(format!("Merge failed: {}. Task remains in REVIEW.", merge_err));
            }

            // Remove the worktree and the feature branch
            let _ = crate::git::run_git(&repo_root_path, &[
                "worktree", "remove", "--force", wt_path,
            ]);
            let _ = crate::git::run_git(&repo_root_path, &[
                "branch", "-D", branch,
            ]);
        }
    }

    // Return updated task
    let updated_task = db.query_row(
        "SELECT id, repo_id, title, status, branch_name, worktree_path, created_at, updated_at
         FROM tasks WHERE id = ?1",
        params![payload.task_id],
        |row| {
            Ok(Task {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                title: row.get(2)?,
                status: row.get(3)?,
                branch_name: row.get(4)?,
                worktree_path: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(TaskWithDiff { task: updated_task, diff })
}

#[derive(Debug, Deserialize)]
pub struct DeleteTaskPayload {
    pub task_id: String,
}

#[tauri::command]
pub fn delete_task(payload: DeleteTaskPayload, state: State<AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Fetch task info + repo root before deleting
    let info: Option<(Option<String>, Option<String>, String)> = db.query_row(
        "SELECT t.branch_name, t.worktree_path, r.root_path
         FROM tasks t JOIN repos r ON t.repo_id = r.id
         WHERE t.id = ?1",
        params![payload.task_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).ok();

    let rows = db
        .execute("DELETE FROM tasks WHERE id = ?1", params![payload.task_id])
        .map_err(|e| e.to_string())?;

    if rows == 0 {
        return Err("Task not found".to_string());
    }

    // Best-effort git cleanup (errors are non-fatal)
    if let Some((branch_name, worktree_path, repo_root)) = info {
        if let Some(ref wt_path) = worktree_path {
            let _ = crate::git::run_git(&repo_root, &[
                "worktree", "remove", "--force", wt_path,
            ]);
        }
        if let Some(ref branch) = branch_name {
            let _ = crate::git::run_git(&repo_root, &[
                "branch", "-D", branch,
            ]);
        }
    }

    Ok(())
}
