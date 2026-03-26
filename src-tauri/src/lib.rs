mod claude;
mod db;
mod git;
mod pty;
mod state;
pub mod commands;

use commands::{
    chat::{approve_tool_call, reject_tool_call, send_chat_message, ToolPendingState, ToolResultsBuffer},
    files::{list_worktree_files, read_file_content},
    messages::{insert_message, list_messages},
    pty::{pty_create, pty_kill, pty_resize, pty_write},
    repos::{add_repo, list_repos},
    settings::{get_api_key, set_api_key},
    tasks::{create_task, delete_task, list_tasks, update_task_status},
};
use pty::PtyMap;
use state::AppState;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;

            let db_path = db::db_path(&app_data_dir);
            let conn = db::open(&db_path)?;
            db::run_migrations(&conn)?;

            app.manage(AppState {
                db: Arc::new(Mutex::new(conn)),
            });

            app.manage(Arc::new(Mutex::new(
                HashMap::<String, crate::pty::PtySession>::new(),
            )) as PtyMap);

            app.manage(ToolPendingState::default());
            app.manage(ToolResultsBuffer::default());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_repos,
            add_repo,
            list_tasks,
            create_task,
            update_task_status,
            delete_task,
            list_messages,
            insert_message,
            pty_create,
            pty_write,
            pty_resize,
            pty_kill,
            send_chat_message,
            approve_tool_call,
            reject_tool_call,
            list_worktree_files,
            read_file_content,
            get_api_key,
            set_api_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
