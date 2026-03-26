use crate::claude::{stream_chat, ClaudeMessage, PendingToolCall, StreamError};
use crate::commands::settings::get_api_key;
use crate::commands::tools::{apply_diff_to_file, execute_command_in_worktree, read_file_from_worktree};
use crate::state::AppState;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

/// Maps task_id → pending tool calls in the current batch.
pub type ToolPendingState = Arc<Mutex<HashMap<String, Vec<PendingToolCall>>>>;

/// Maps task_id → accumulated tool_result blocks for the current batch.
/// All results are buffered here; a single combined user message is saved
/// to DB only when every call in the batch is resolved.
pub type ToolResultsBuffer = Arc<Mutex<HashMap<String, Vec<Value>>>>;

// ---------------------------------------------------------------------------
// Payload types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SendChatPayload {
    pub task_id: String,
    pub content: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    pub id: String,
    pub name: String,
    pub input: Value,
}

#[derive(Clone, Serialize)]
struct ToolCallsEvent {
    task_id: String,
    tool_calls: Vec<ToolCallInfo>,
}

#[derive(Debug, Deserialize)]
pub struct ApproveToolCallPayload {
    pub task_id: String,
    pub tool_use_id: String,
}

#[derive(Debug, Deserialize)]
pub struct RejectToolCallPayload {
    pub task_id: String,
    pub tool_use_id: String,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn load_claude_messages(
    db: &rusqlite::Connection,
    task_id: &str,
) -> Result<Vec<ClaudeMessage>, String> {
    let mut stmt = db
        .prepare(
            "SELECT role, content FROM messages
             WHERE task_id = ?1 AND role != 'system'
             ORDER BY created_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![task_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?;

    let pairs: Result<Vec<_>, _> = rows.collect();
    let pairs = pairs.map_err(|e: rusqlite::Error| e.to_string())?;

    let messages = pairs
        .into_iter()
        .map(|(role, content_str)| {
            let content_value: Value = if content_str.starts_with('[') {
                serde_json::from_str(&content_str).unwrap_or(Value::String(content_str))
            } else {
                Value::String(content_str)
            };
            ClaudeMessage {
                role,
                content: content_value,
            }
        })
        .collect();

    Ok(messages)
}

fn get_worktree_path(db: &rusqlite::Connection, task_id: &str) -> Result<String, String> {
    db.query_row(
        "SELECT worktree_path FROM tasks WHERE id = ?1",
        params![task_id],
        |row| row.get::<_, Option<String>>(0),
    )
    .map_err(|e| e.to_string())?
    .ok_or_else(|| format!("Task {} has no worktree_path set", task_id))
}

fn save_message(
    db: &rusqlite::Connection,
    task_id: &str,
    role: &str,
    content: &Value,
) -> Result<(), String> {
    let content_str = match content {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    let msg_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    db.execute(
        "INSERT INTO messages (id, task_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![msg_id, task_id, role, content_str, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Parse `@filepath` tokens from message content, read each file from the worktree,
/// and return augmented content with file blocks prepended. Falls back to original
/// content if no worktree is set or no files are found.
fn inject_file_context(db: &rusqlite::Connection, task_id: &str, content: &str) -> String {
    // Collect unique @filepath tokens (split on whitespace)
    let mut file_paths: Vec<&str> = content
        .split(|c: char| c.is_whitespace())
        .filter_map(|word| word.strip_prefix('@'))
        .filter(|p| !p.is_empty())
        .collect();
    file_paths.dedup();

    if file_paths.is_empty() {
        return content.to_string();
    }

    let worktree_path = match get_worktree_path(db, task_id) {
        Ok(p) => p,
        Err(_) => return content.to_string(),
    };

    let mut blocks = Vec::new();
    for file_path in file_paths {
        if let Ok(file_content) = read_file_from_worktree(&worktree_path, file_path, false) {
            blocks.push(format!("<file path=\"{}\">\n{}\n</file>", file_path, file_content));
        }
    }

    if blocks.is_empty() {
        return content.to_string();
    }

    format!("{}\n\n{}", blocks.join("\n\n"), content)
}

fn pending_to_event_list(calls: &[PendingToolCall]) -> Vec<ToolCallInfo> {
    calls
        .iter()
        .map(|c| ToolCallInfo {
            id: c.id.clone(),
            name: c.name.clone(),
            input: c.input.clone(),
        })
        .collect()
}

/// Process a ChatResult: save assistant message, then either emit `chat:complete`
/// or store pending tool calls and emit `chat:tool_calls_pending`.
async fn process_chat_result(
    result: crate::claude::ChatResult,
    task_id: String,
    app: AppHandle,
    db_arc: Arc<Mutex<rusqlite::Connection>>,
    tool_state: ToolPendingState,
    results_buffer: ToolResultsBuffer,
) -> Result<(), String> {
    // Save assistant message (full content block array or plain text)
    {
        let db = db_arc.lock().map_err(|e| e.to_string())?;
        save_message(&db, &task_id, "assistant", &result.full_content)?;
    }

    if result.tool_uses.is_empty() {
        let _ = app.emit(
            "chat:complete",
            crate::claude::StreamDone {
                task_id: task_id.clone(),
                full_text: result.text,
            },
        );
    } else {
        let event_list = pending_to_event_list(&result.tool_uses);
        // Initialize empty results buffer for this batch
        {
            let mut buf = results_buffer.lock().map_err(|e| e.to_string())?;
            buf.insert(task_id.clone(), Vec::new());
        }
        {
            let mut state = tool_state.lock().map_err(|e| e.to_string())?;
            state.insert(task_id.clone(), result.tool_uses);
        }
        let _ = app.emit(
            "chat:tool_calls_pending",
            ToolCallsEvent {
                task_id,
                tool_calls: event_list,
            },
        );
    }

    Ok(())
}

/// Called when all tool calls in a batch are resolved.
/// Saves ONE combined tool_result user message and continues the agentic loop.
async fn continue_after_tool_batch(
    task_id: String,
    all_results: Vec<Value>,
    app: AppHandle,
    db_arc: Arc<Mutex<rusqlite::Connection>>,
    tool_state: ToolPendingState,
    results_buffer: ToolResultsBuffer,
) -> Result<(), String> {
    // Save a single combined user message for all tool_results in this batch
    let combined_content = Value::Array(all_results);
    {
        let db = db_arc.lock().map_err(|e| e.to_string())?;
        save_message(&db, &task_id, "user", &combined_content)?;
    }

    // Load full history and continue streaming
    let api_key = get_api_key()?.ok_or("Anthropic API key not set.")?;
    let messages = {
        let db = db_arc.lock().map_err(|e| e.to_string())?;
        load_claude_messages(&db, &task_id)?
    };

    let app_clone = app.clone();
    let task_id_clone = task_id.clone();
    let db_arc_clone = db_arc.clone();
    let tool_state_clone = tool_state.clone();
    let results_buffer_clone = results_buffer.clone();

    tauri::async_runtime::spawn(async move {
        match stream_chat(api_key, task_id_clone.clone(), messages, app_clone.clone()).await {
            Ok(result) => {
                if let Err(e) = process_chat_result(
                    result,
                    task_id_clone.clone(),
                    app_clone.clone(),
                    db_arc_clone,
                    tool_state_clone,
                    results_buffer_clone,
                )
                .await
                {
                    eprintln!("Agentic loop error: {}", e);
                    let _ = app_clone.emit("chat:error", StreamError { task_id: task_id_clone, error: e });
                }
            }
            Err(e) => {
                eprintln!("Claude streaming error in agentic loop: {}", e);
                let _ = app_clone.emit("chat:error", StreamError { task_id: task_id_clone, error: e });
            }
        }
    });

    Ok(())
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn send_chat_message(
    payload: SendChatPayload,
    app: AppHandle,
    state: State<'_, AppState>,
    tool_state: State<'_, ToolPendingState>,
    results_buffer: State<'_, ToolResultsBuffer>,
) -> Result<(), String> {
    let api_key = get_api_key()?.ok_or("Anthropic API key not set. Please add it in Settings.")?;

    let messages = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let history = load_claude_messages(&db, &payload.task_id)?;
        // Save original content to DB (clean history without injected file blobs)
        save_message(&db, &payload.task_id, "user", &Value::String(payload.content.clone()))?;
        // Augment with @file context for Claude (not persisted)
        let augmented = inject_file_context(&db, &payload.task_id, &payload.content);
        let mut messages = history;
        messages.push(ClaudeMessage {
            role: "user".to_string(),
            content: Value::String(augmented),
        });
        messages
    };

    let task_id = payload.task_id.clone();
    let db_arc = state.inner().db.clone();
    let tool_state_arc = tool_state.inner().clone();
    let results_buffer_arc = results_buffer.inner().clone();

    tauri::async_runtime::spawn(async move {
        match stream_chat(api_key, task_id.clone(), messages, app.clone()).await {
            Ok(result) => {
                if let Err(e) = process_chat_result(
                    result,
                    task_id.clone(),
                    app.clone(),
                    db_arc,
                    tool_state_arc,
                    results_buffer_arc,
                )
                .await
                {
                    eprintln!("Error processing chat result: {}", e);
                    let _ = app.emit("chat:error", StreamError { task_id, error: e });
                }
            }
            Err(e) => {
                eprintln!("Claude streaming error: {}", e);
                let _ = app.emit("chat:error", StreamError { task_id, error: e });
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn approve_tool_call(
    payload: ApproveToolCallPayload,
    app: AppHandle,
    state: State<'_, AppState>,
    tool_state: State<'_, ToolPendingState>,
    results_buffer: State<'_, ToolResultsBuffer>,
) -> Result<(), String> {
    // Remove this call from pending state; determine atomically if it's the last.
    let (tool_call, is_last) = {
        let mut pending = tool_state.lock().map_err(|e| e.to_string())?;
        let calls = pending
            .get_mut(&payload.task_id)
            .ok_or_else(|| format!("No pending tool calls for task {}", payload.task_id))?;
        let pos = calls
            .iter()
            .position(|c| c.id == payload.tool_use_id)
            .ok_or_else(|| format!("Tool use id {} not found", payload.tool_use_id))?;
        let call = calls.remove(pos);
        let is_last = calls.is_empty();
        if is_last {
            pending.remove(&payload.task_id);
        }
        (call, is_last)
    };

    // Get worktree path
    let worktree_path = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        get_worktree_path(&db, &payload.task_id)?
    };

    // Execute the tool
    let result_text = match tool_call.name.as_str() {
        "execute_command" => {
            let command = tool_call.input["command"]
                .as_str()
                .ok_or("execute_command: missing 'command' field")?;
            execute_command_in_worktree(&worktree_path, command)?
        }
        "apply_diff" => {
            let file_path = tool_call.input["file_path"]
                .as_str()
                .ok_or("apply_diff: missing 'file_path' field")?;
            let old_content = tool_call.input["old_content"]
                .as_str()
                .ok_or("apply_diff: missing 'old_content' field")?;
            let new_content = tool_call.input["new_content"]
                .as_str()
                .ok_or("apply_diff: missing 'new_content' field")?;
            apply_diff_to_file(&worktree_path, file_path, old_content, new_content)?
        }
        "read_file" => {
            let file_path = tool_call.input["file_path"]
                .as_str()
                .ok_or("read_file: missing 'file_path' field")?;
            let full = tool_call.input["full"].as_bool().unwrap_or(false);
            read_file_from_worktree(&worktree_path, file_path, full)?
        }
        unknown => return Err(format!("Unknown tool: {}", unknown)),
    };

    // Buffer this tool result
    let tool_result_block = json!({
        "type": "tool_result",
        "tool_use_id": payload.tool_use_id,
        "content": result_text,
    });

    let all_results = {
        let mut buf = results_buffer.lock().map_err(|e| e.to_string())?;
        let entry = buf.entry(payload.task_id.clone()).or_default();
        entry.push(tool_result_block);
        if is_last {
            buf.remove(&payload.task_id).unwrap_or_default()
        } else {
            Vec::new() // more calls pending — don't continue yet
        }
    };

    if is_last {
        let db_arc = state.inner().db.clone();
        let tool_state_arc = tool_state.inner().clone();
        let results_buffer_arc = results_buffer.inner().clone();
        continue_after_tool_batch(
            payload.task_id,
            all_results,
            app,
            db_arc,
            tool_state_arc,
            results_buffer_arc,
        )
        .await?;
    }

    Ok(())
}

#[tauri::command]
pub async fn reject_tool_call(
    payload: RejectToolCallPayload,
    app: AppHandle,
    state: State<'_, AppState>,
    tool_state: State<'_, ToolPendingState>,
    results_buffer: State<'_, ToolResultsBuffer>,
) -> Result<(), String> {
    let is_last = {
        let mut pending = tool_state.lock().map_err(|e| e.to_string())?;
        let calls = pending
            .get_mut(&payload.task_id)
            .ok_or_else(|| format!("No pending tool calls for task {}", payload.task_id))?;
        let pos = calls
            .iter()
            .position(|c| c.id == payload.tool_use_id)
            .ok_or_else(|| format!("Tool use id {} not found", payload.tool_use_id))?;
        calls.remove(pos);
        let is_last = calls.is_empty();
        if is_last {
            pending.remove(&payload.task_id);
        }
        is_last
    };

    let rejection_block = json!({
        "type": "tool_result",
        "tool_use_id": payload.tool_use_id,
        "content": format!("User rejected this tool call. Reason: {}", payload.reason),
        "is_error": true,
    });

    let all_results = {
        let mut buf = results_buffer.lock().map_err(|e| e.to_string())?;
        let entry = buf.entry(payload.task_id.clone()).or_default();
        entry.push(rejection_block);
        if is_last {
            buf.remove(&payload.task_id).unwrap_or_default()
        } else {
            Vec::new()
        }
    };

    if is_last {
        let db_arc = state.inner().db.clone();
        let tool_state_arc = tool_state.inner().clone();
        let results_buffer_arc = results_buffer.inner().clone();
        continue_after_tool_batch(
            payload.task_id,
            all_results,
            app,
            db_arc,
            tool_state_arc,
            results_buffer_arc,
        )
        .await?;
    }

    Ok(())
}
