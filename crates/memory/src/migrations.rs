use rusqlite::{Connection, Result};

// 初始化 Distilllab 表结构
pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS sources (
            id TEXT PRIMARY KEY,
            source_type TEXT NOT NULL,
            title TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
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
        assert_eq!(sources_exists, "sources");
        assert_eq!(runs_exists, "runs");
        assert_eq!(chunks_exists, "chunks");
        assert_eq!(work_items_exists, "work_items");
    }
}
