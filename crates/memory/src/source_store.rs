use rusqlite::{params, types::Type, Connection, Error, Result};
use schema::{Source, SourceType};

pub fn insert_source(conn: &Connection, source: &Source) -> Result<()> {
    conn.execute(
        "INSERT INTO sources (id, source_type, title, run_id, origin_key, locator, content, metadata_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            source.id,
            source.source_type.as_str(),
            source.title,
            source.run_id,
            source.origin_key,
            source.locator,
            source.content,
            source.metadata_json,
            source.created_at
        ],
    )?;

    Ok(())
}

pub fn list_sources(conn: &Connection) -> Result<Vec<Source>> {
    let mut stmt = conn.prepare(
        "SELECT id, source_type, title, run_id, origin_key, locator, content, metadata_json, created_at FROM sources ORDER BY created_at DESC",
    )?;

    let source_iter = stmt.query_map([], |row| {
        let source_type_str: String = row.get(1)?;
        let source_type = SourceType::from_str(&source_type_str).ok_or_else(|| {
            Error::FromSqlConversionFailure(
                1,
                Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid source type: {source_type_str}"),
                )),
            )
        })?;

        Ok(Source {
            id: row.get(0)?,
            source_type,
            title: row.get(2)?,
            run_id: row.get(3)?,
            origin_key: row.get(4)?,
            locator: row.get(5)?,
            content: row.get(6)?,
            metadata_json: row.get(7)?,
            created_at: row.get(8)?,
        })
    })?;

    let mut sources = Vec::new();
    for source in source_iter {
        sources.push(source?);
    }

    Ok(sources)
}

pub fn get_source_by_run_origin(
    conn: &Connection,
    run_id: &str,
    origin_key: &str,
) -> Result<Option<Source>> {
    let mut stmt = conn.prepare(
        "SELECT id, source_type, title, run_id, origin_key, locator, content, metadata_json, created_at FROM sources WHERE run_id = ?1 AND origin_key = ?2 LIMIT 1",
    )?;

    let mut rows = stmt.query(params![run_id, origin_key])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };

    let source_type_str: String = row.get(1)?;
    let source_type = SourceType::from_str(&source_type_str).ok_or_else(|| {
        Error::FromSqlConversionFailure(
            1,
            Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid source type: {source_type_str}"),
            )),
        )
    })?;

    Ok(Some(Source {
        id: row.get(0)?,
        source_type,
        title: row.get(2)?,
        run_id: row.get(3)?,
        origin_key: row.get(4)?,
        locator: row.get(5)?,
        content: row.get(6)?,
        metadata_json: row.get(7)?,
        created_at: row.get(8)?,
    }))
}

pub fn list_sources_by_run(conn: &Connection, run_id: &str) -> Result<Vec<Source>> {
    let mut stmt = conn.prepare(
        "SELECT id, source_type, title, run_id, origin_key, locator, content, metadata_json, created_at FROM sources WHERE run_id = ?1 ORDER BY created_at DESC",
    )?;

    let source_iter = stmt.query_map([run_id], |row| {
        let source_type_str: String = row.get(1)?;
        let source_type = SourceType::from_str(&source_type_str).ok_or_else(|| {
            Error::FromSqlConversionFailure(
                1,
                Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid source type: {source_type_str}"),
                )),
            )
        })?;

        Ok(Source {
            id: row.get(0)?,
            source_type,
            title: row.get(2)?,
            run_id: row.get(3)?,
            origin_key: row.get(4)?,
            locator: row.get(5)?,
            content: row.get(6)?,
            metadata_json: row.get(7)?,
            created_at: row.get(8)?,
        })
    })?;

    let mut sources = Vec::new();
    for source in source_iter {
        sources.push(source?);
    }

    Ok(sources)
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

        let source = Source {
            id: "source-1".to_string(),
            source_type: SourceType::Document,
            title: "Test Source".to_string(),
            run_id: None,
            origin_key: None,
            locator: None,
            metadata_json: "{}".to_string(),
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

    #[test]
    fn lists_inserted_sources() {
        let conn = Connection::open_in_memory().expect("failed to open in-memory database");
        run_migrations(&conn).expect("failed to run migrations");

        let source_one = Source {
            id: "source-1".to_string(),
            source_type: SourceType::Document,
            title: "First Source".to_string(),
            run_id: None,
            origin_key: None,
            locator: None,
            metadata_json: "{}".to_string(),
            created_at: "2026-03-25T00:00:00Z".to_string(),
        };

        let source_two = Source {
            id: "source-2".to_string(),
            source_type: SourceType::Session,
            title: "Second Source".to_string(),
            run_id: None,
            origin_key: None,
            locator: None,
            metadata_json: "{}".to_string(),
            created_at: "2026-03-25T00:00:01Z".to_string(),
        };

        insert_source(&conn, &source_one).expect("failed to insert first source");
        insert_source(&conn, &source_two).expect("failed to insert second source");

        let sources = list_sources(&conn).expect("failed to list sources");

        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].id, "source-2");
        assert_eq!(sources[0].source_type.as_str(), "session");
        assert_eq!(sources[1].id, "source-1");
        assert_eq!(sources[1].source_type.as_str(), "document");
    }

    #[test]
    fn inserts_source_with_run_origin_locator_and_metadata_fields() {
        let conn = Connection::open_in_memory().expect("failed to open in-memory database");
        run_migrations(&conn).expect("failed to run migrations");

        let source = Source {
            id: "source-extended-1".to_string(),
            source_type: SourceType::Document,
            title: "Attachment Source".to_string(),
            run_id: Some("run-1".to_string()),
            origin_key: Some("attachment:attachment-1".to_string()),
            locator: Some("/tmp/attachments/file.md".to_string()),
            metadata_json: r#"{"mime_type":"text/markdown"}"#.to_string(),
            created_at: "2026-03-30T00:00:00Z".to_string(),
        };

        insert_source(&conn, &source).expect("failed to insert extended source");

        let row: (Option<String>, Option<String>, Option<String>, String) = conn
            .query_row(
                "SELECT run_id, origin_key, locator, metadata_json FROM sources WHERE id = ?1",
                ["source-extended-1"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("extended source row should exist");

        assert_eq!(row.0.as_deref(), Some("run-1"));
        assert_eq!(row.1.as_deref(), Some("attachment:attachment-1"));
        assert_eq!(row.2.as_deref(), Some("/tmp/attachments/file.md"));
        assert_eq!(row.3, r#"{"mime_type":"text/markdown"}"#);
    }

    #[test]
    fn finds_source_by_run_and_origin_key() {
        let conn = Connection::open_in_memory().expect("failed to open in-memory database");
        run_migrations(&conn).expect("failed to run migrations");

        let source = Source {
            id: "source-origin-1".to_string(),
            source_type: SourceType::Session,
            title: "Session Message Source".to_string(),
            run_id: Some("run-1".to_string()),
            origin_key: Some("session-message:session-1:abcd1234".to_string()),
            locator: None,
            metadata_json: "{}".to_string(),
            created_at: "2026-03-30T00:00:00Z".to_string(),
        };

        insert_source(&conn, &source).expect("failed to insert origin source");

        let found = get_source_by_run_origin(&conn, "run-1", "session-message:session-1:abcd1234")
            .expect("failed to query source by run origin")
            .expect("source should be found");

        assert_eq!(found.id, "source-origin-1");
        assert_eq!(found.run_id.as_deref(), Some("run-1"));
        assert_eq!(
            found.origin_key.as_deref(),
            Some("session-message:session-1:abcd1234")
        );
    }

    #[test]
    fn lists_sources_for_run_only() {
        let conn = Connection::open_in_memory().expect("failed to open in-memory database");
        run_migrations(&conn).expect("failed to run migrations");

        let run_one_source = Source {
            id: "source-run-1".to_string(),
            source_type: SourceType::Document,
            title: "Run One Source".to_string(),
            run_id: Some("run-1".to_string()),
            origin_key: Some("attachment:1".to_string()),
            locator: Some("/tmp/run-1.md".to_string()),
            metadata_json: "{}".to_string(),
            created_at: "2026-03-30T00:00:00Z".to_string(),
        };

        let run_two_source = Source {
            id: "source-run-2".to_string(),
            source_type: SourceType::Document,
            title: "Run Two Source".to_string(),
            run_id: Some("run-2".to_string()),
            origin_key: Some("attachment:2".to_string()),
            locator: Some("/tmp/run-2.md".to_string()),
            metadata_json: "{}".to_string(),
            created_at: "2026-03-30T00:00:01Z".to_string(),
        };

        insert_source(&conn, &run_one_source).expect("failed to insert run one source");
        insert_source(&conn, &run_two_source).expect("failed to insert run two source");

        let sources = list_sources_by_run(&conn, "run-1").expect("failed to list sources by run");

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].id, "source-run-1");
        assert_eq!(sources[0].run_id.as_deref(), Some("run-1"));
    }
}
