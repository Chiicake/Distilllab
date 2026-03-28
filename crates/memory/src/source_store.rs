use rusqlite::{Connection, Error, Result, params, types::Type};
use schema::{Source, SourceType};

pub fn insert_source(conn: &Connection, source: &Source) -> Result<()> {
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

pub fn list_sources(conn: &Connection) -> Result<Vec<Source>> {
    let mut stmt = conn.prepare(
        "SELECT id, source_type, title, created_at FROM sources ORDER BY created_at DESC",
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
            created_at: row.get(3)?,
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
            created_at: "2026-03-25T00:00:00Z".to_string(),
        };

        let source_two = Source {
            id: "source-2".to_string(),
            source_type: SourceType::Session,
            title: "Second Source".to_string(),
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
}
