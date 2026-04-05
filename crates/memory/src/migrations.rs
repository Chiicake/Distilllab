use rusqlite::{Connection, Result};

// 初始化 Distilllab 表结构
pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS sources (
            id TEXT PRIMARY KEY,
            source_type TEXT NOT NULL,
            title TEXT NOT NULL,
            run_id TEXT,
            origin_key TEXT,
            locator TEXT,
            content TEXT,
            metadata_json TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            manual_title TEXT,
            pinned INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL,
            current_intent TEXT NOT NULL,
            current_object_type TEXT NOT NULL,
            current_object_id TEXT NOT NULL,
            summary TEXT NOT NULL,
            started_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            last_user_message_at TEXT NOT NULL,
            last_run_at TEXT NOT NULL,
            last_compacted_at TEXT NOT NULL,
            metadata_json TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS session_messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            run_id TEXT,
            message_type TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            data_json TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_session_messages_session_id_created_at
            ON session_messages (session_id, created_at);
        CREATE TABLE IF NOT EXISTS chunks (
            id TEXT PRIMARY KEY,
            source_id TEXT NOT NULL,
            sequence INTEGER NOT NULL,
            content TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS work_items (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            work_item_type TEXT NOT NULL,
            title TEXT NOT NULL,
            summary TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            summary TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS assets (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            asset_type TEXT NOT NULL,
            title TEXT NOT NULL,
            summary TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS runs (
            id TEXT PRIMARY KEY,
            run_type TEXT NOT NULL,
            status TEXT NOT NULL,
            primary_object_type TEXT NOT NULL,
            primary_object_id TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        "#,
    )?;

    add_column_if_missing(conn, "sources", "run_id", "TEXT")?;
    add_column_if_missing(conn, "sources", "origin_key", "TEXT")?;
    add_column_if_missing(conn, "sources", "locator", "TEXT")?;
    add_column_if_missing(conn, "sources", "content", "TEXT")?;
    add_column_if_missing(
        conn,
        "sources",
        "metadata_json",
        "TEXT NOT NULL DEFAULT '{}' ",
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sources_run_id_origin_key ON sources (run_id, origin_key)",
        [],
    )?;
    add_column_if_missing(conn, "sessions", "manual_title", "TEXT")?;
    add_column_if_missing(conn, "sessions", "pinned", "INTEGER NOT NULL DEFAULT 0")?;

    Ok(())
}

fn add_column_if_missing(
    conn: &Connection,
    table_name: &str,
    column_name: &str,
    column_definition: &str,
) -> Result<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table_name})"))?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;

    for column in columns {
        if column? == column_name {
            return Ok(());
        }
    }

    conn.execute(
        &format!("ALTER TABLE {table_name} ADD COLUMN {column_name} {column_definition}"),
        [],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_creates_tables() {
        let conn = Connection::open_in_memory().expect("failed to open in-memory database");
        run_migrations(&conn).expect("failed to run migrations");
        let sources_exists: String = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'sources'",
                [],
                |row| row.get(0),
            )
            .expect("sources table should exist");
        let sessions_exists: String = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'sessions'",
                [],
                |row| row.get(0),
            )
            .expect("sessions table should exist");
        let runs_exists: String = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'runs'",
                [],
                |row| row.get(0),
            )
            .expect("runs table should exist");
        let chunks_exists: String = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'chunks'",
                [],
                |row| row.get(0),
            )
            .expect("chunks table should exist");
        let work_items_exists: String = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'work_items'",
                [],
                |row| row.get(0),
            )
            .expect("work_items table should exist");
        let projects_exists: String = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'projects'",
                [],
                |row| row.get(0),
            )
            .expect("projects table should exist");
        let assets_exists: String = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'assets'",
                [],
                |row| row.get(0),
            )
            .expect("assets table should exist");
        let session_messages_exists: String = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'session_messages'",
                [],
                |row| row.get(0),
            )
            .expect("session_messages table should exist");
        assert_eq!(sources_exists, "sources");
        assert_eq!(sessions_exists, "sessions");
        assert_eq!(session_messages_exists, "session_messages");
        assert_eq!(runs_exists, "runs");
        assert_eq!(chunks_exists, "chunks");
        assert_eq!(work_items_exists, "work_items");
        assert_eq!(projects_exists, "projects");
        assert_eq!(assets_exists, "assets");
    }
}
