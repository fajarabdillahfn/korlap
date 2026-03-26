use crate::state::AppState;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Repo {
    pub id: String,
    pub name: String,
    pub root_path: String,
}

#[tauri::command]
pub fn list_repos(state: State<AppState>) -> Result<Vec<Repo>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare("SELECT id, name, root_path FROM repos ORDER BY name ASC")
        .map_err(|e| e.to_string())?;

    let repos = stmt
        .query_map([], |row| {
            Ok(Repo {
                id: row.get(0)?,
                name: row.get(1)?,
                root_path: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(repos)
}

#[tauri::command]
pub fn add_repo(path: String, state: State<AppState>) -> Result<Repo, String> {
    let root_path = path;

    // Validate: directory exists
    let path = std::path::Path::new(&root_path);
    if !path.is_dir() {
        return Err(format!("Path does not exist or is not a directory: {}", root_path));
    }

    // Validate: is a git repository
    if !path.join(".git").exists() {
        return Err("Selected directory is not a git repository".to_string());
    }

    // Derive a display name from the directory name
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let id = Uuid::new_v4().to_string();

    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO repos (id, name, root_path) VALUES (?1, ?2, ?3)",
        params![id, name, root_path],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint failed") {
            "This repository is already added".to_string()
        } else {
            e.to_string()
        }
    })?;

    Ok(Repo { id, name, root_path })
}
