use rusqlite::{Connection, Result, params};
use schema::SourceRecord;

pub fn insert_source(conn: &Connection, source: &SourceRecord) -> Result<()> {
    conn.execute(
        "INSERT INTO sources (id, source_type, title, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![
            source.id,
            source.source_type.as_str(),
            source.title,
            source.created_at
        ],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::run_migrations;
    use rusqlite::Connection;
    use schema::SourceType;

    #[test]
    fn inserts_source_record() {
        let conn = Connection::open_in_memory().expect("failed to open in-memory database");
        run_migrations(&conn).expect("failed to run migrations");

        let source = SourceRecord {
            id: "source-1".to_string(),
            source_type: SourceType::Document,
            title: "Test Source".to_string(),
            created_at: "2026-03-25T00:00:00Z".to_string(),
        };

        insert_source(&conn, &source).expect("failed to insert source");

        let inserted_title: String = conn
            .query_row(
                "SELECT title FROM sources WHERE id = ?1",
                ["source-1"],
                |row| row.get(0),
            )
            .expect("source row should exist");

        assert_eq!(inserted_title, "Test Source");
    }
}
