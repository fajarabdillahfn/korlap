use rusqlite::{Connection, Result};
use std::path::PathBuf;

pub fn db_path(app_data_dir: &PathBuf) -> PathBuf {
    app_data_dir.join("korlap.db")
}

pub fn open(path: &PathBuf) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    Ok(conn)
}

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS repos (
            id          TEXT PRIMARY KEY NOT NULL,
            name        TEXT NOT NULL,
            root_path   TEXT NOT NULL UNIQUE
        );

        CREATE TABLE IF NOT EXISTS tasks (
            id              TEXT PRIMARY KEY NOT NULL,
            repo_id         TEXT NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
            title           TEXT NOT NULL,
            status          TEXT NOT NULL DEFAULT 'todo',
            branch_name     TEXT,
            worktree_path   TEXT,
            created_at      TEXT NOT NULL,
            updated_at      TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS messages (
            id          TEXT PRIMARY KEY NOT NULL,
            task_id     TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            role        TEXT NOT NULL,
            content     TEXT NOT NULL,
            tool_calls  TEXT,
            created_at  TEXT NOT NULL
        );
    ")?;
    Ok(())
}
