use rusqlite::{params, Connection, Result};
use schema::Chunk;

pub fn insert_chunk(conn: &Connection, chunk: &Chunk) -> Result<()> {
    conn.execute(
        "INSERT INTO chunks (id, source_id, sequence, content) VALUES (?1, ?2, ?3, ?4)",
        params![chunk.id, chunk.source_id, chunk.sequence, chunk.content],
    )?;

    Ok(())
}

pub fn list_chunks_by_source(conn: &Connection, source_id: &str) -> Result<Vec<Chunk>> {
    let mut stmt = conn.prepare(
        "SELECT id, source_id, sequence, content FROM chunks WHERE source_id = ?1 ORDER BY sequence ASC",
    )?;

    let chunk_iter = stmt.query_map([source_id], |row| {
        Ok(Chunk {
            id: row.get(0)?,
            source_id: row.get(1)?,
            sequence: row.get(2)?,
            content: row.get(3)?,
        })
    })?;

    let mut chunks = Vec::new();
    for chunk in chunk_iter {
        chunks.push(chunk?);
    }

    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::run_migrations;
    use rusqlite::Connection;

    #[test]
    fn inserts_and_lists_chunks_by_source() {
        let conn = Connection::open_in_memory().expect("failed to open in-memory database");
        run_migrations(&conn).expect("failed to run migrations");

        let chunk_one = Chunk {
            id: "chunk-1".to_string(),
            source_id: "source-1".to_string(),
            sequence: 0,
            content: "First chunk".to_string(),
        };

        let chunk_two = Chunk {
            id: "chunk-2".to_string(),
            source_id: "source-1".to_string(),
            sequence: 1,
            content: "Second chunk".to_string(),
        };

        insert_chunk(&conn, &chunk_one).expect("failed to insert first chunk");
        insert_chunk(&conn, &chunk_two).expect("failed to insert second chunk");

        let chunks = list_chunks_by_source(&conn, "source-1").expect("failed to list chunks");

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].id, "chunk-1");
        assert_eq!(chunks[1].id, "chunk-2");
        assert_eq!(chunks[0].content, "First chunk");
        assert_eq!(chunks[1].content, "Second chunk");
    }
}
