use crate::state::AppState;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub id: String,
    pub task_id: String,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<String>,
    pub created_at: String,
}

#[tauri::command]
pub fn list_messages(task_id: String, state: State<AppState>) -> Result<Vec<Message>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT id, task_id, role, content, tool_calls, created_at
             FROM messages WHERE task_id = ?1 ORDER BY created_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let messages = stmt
        .query_map(params![task_id], |row| {
            Ok(Message {
                id: row.get(0)?,
                task_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                tool_calls: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(messages)
}

#[derive(Debug, Deserialize)]
pub struct InsertMessagePayload {
    pub task_id: String,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<String>,
}

#[tauri::command]
pub fn insert_message(
    payload: InsertMessagePayload,
    state: State<AppState>,
) -> Result<Message, String> {
    let valid_roles = ["user", "assistant", "system"];
    if !valid_roles.contains(&payload.role.as_str()) {
        return Err(format!(
            "Invalid role '{}'. Must be one of: user, assistant, system",
            payload.role
        ));
    }

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO messages (id, task_id, role, content, tool_calls, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, payload.task_id, payload.role, payload.content, payload.tool_calls, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(Message {
        id,
        task_id: payload.task_id,
        role: payload.role,
        content: payload.content,
        tool_calls: payload.tool_calls,
        created_at: now,
    })
}
