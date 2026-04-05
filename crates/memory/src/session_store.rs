use rusqlite::{params, types::Type, Connection, Error, Result};
use schema::{Session, SessionStatus};

pub fn insert_session(conn: &Connection, session: &Session) -> Result<()> {
    conn.execute(
        "INSERT INTO sessions (id, title, manual_title, pinned, status, current_intent, current_object_type, current_object_id, summary, started_at, updated_at, last_user_message_at, last_run_at, last_compacted_at, metadata_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            session.id,
            session.title,
            session.manual_title,
            if session.pinned { 1 } else { 0 },
            session.status.as_str(),
            session.current_intent,
            session.current_object_type,
            session.current_object_id,
            session.summary,
            session.started_at,
            session.updated_at,
            session.last_user_message_at,
            session.last_run_at,
            session.last_compacted_at,
            session.metadata_json,
        ],
    )?;

    Ok(())
}

pub fn list_sessions(conn: &Connection) -> Result<Vec<Session>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, manual_title, pinned, status, current_intent, current_object_type, current_object_id, summary, started_at, updated_at, last_user_message_at, last_run_at, last_compacted_at, metadata_json FROM sessions ORDER BY pinned DESC, updated_at DESC",
    )?;

    let session_iter = stmt.query_map([], |row| {
        let status_str: String = row.get(4)?;
        let status = SessionStatus::from_str(&status_str).ok_or_else(|| {
            Error::FromSqlConversionFailure(
                4,
                Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid session status: {status_str}"),
                )),
            )
        })?;

        Ok(Session {
            id: row.get(0)?,
            title: row.get(1)?,
            manual_title: row.get(2)?,
            pinned: row.get::<_, i64>(3)? != 0,
            status,
            current_intent: row.get(5)?,
            current_object_type: row.get(6)?,
            current_object_id: row.get(7)?,
            summary: row.get(8)?,
            started_at: row.get(9)?,
            updated_at: row.get(10)?,
            last_user_message_at: row.get(11)?,
            last_run_at: row.get(12)?,
            last_compacted_at: row.get(13)?,
            metadata_json: row.get(14)?,
        })
    })?;

    let mut sessions = Vec::new();
    for session in session_iter {
        sessions.push(session?);
    }

    Ok(sessions)
}

pub fn get_session_by_id(conn: &Connection, session_id: &str) -> Result<Option<Session>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, manual_title, pinned, status, current_intent, current_object_type, current_object_id, summary, started_at, updated_at, last_user_message_at, last_run_at, last_compacted_at, metadata_json FROM sessions WHERE id = ?1",
    )?;

    let mut rows = stmt.query([session_id])?;
    if let Some(row) = rows.next()? {
        let status_str: String = row.get(4)?;
        let status = SessionStatus::from_str(&status_str).ok_or_else(|| {
            Error::FromSqlConversionFailure(
                4,
                Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid session status: {status_str}"),
                )),
            )
        })?;

        return Ok(Some(Session {
            id: row.get(0)?,
            title: row.get(1)?,
            manual_title: row.get(2)?,
            pinned: row.get::<_, i64>(3)? != 0,
            status,
            current_intent: row.get(5)?,
            current_object_type: row.get(6)?,
            current_object_id: row.get(7)?,
            summary: row.get(8)?,
            started_at: row.get(9)?,
            updated_at: row.get(10)?,
            last_user_message_at: row.get(11)?,
            last_run_at: row.get(12)?,
            last_compacted_at: row.get(13)?,
            metadata_json: row.get(14)?,
        }));
    }

    Ok(None)
}

pub fn update_session(conn: &Connection, session: &Session) -> Result<()> {
    conn.execute(
        "UPDATE sessions SET title = ?2, manual_title = ?3, pinned = ?4, status = ?5, current_intent = ?6, current_object_type = ?7, current_object_id = ?8, summary = ?9, started_at = ?10, updated_at = ?11, last_user_message_at = ?12, last_run_at = ?13, last_compacted_at = ?14, metadata_json = ?15 WHERE id = ?1",
        params![
            session.id,
            session.title,
            session.manual_title,
            if session.pinned { 1 } else { 0 },
            session.status.as_str(),
            session.current_intent,
            session.current_object_type,
            session.current_object_id,
            session.summary,
            session.started_at,
            session.updated_at,
            session.last_user_message_at,
            session.last_run_at,
            session.last_compacted_at,
            session.metadata_json,
        ],
    )?;

    Ok(())
}

pub fn delete_session(conn: &Connection, session_id: &str) -> Result<()> {
    conn.execute("DELETE FROM sessions WHERE id = ?1", [session_id])?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::run_migrations;
    use rusqlite::Connection;

    #[test]
    fn inserts_and_lists_sessions() {
        let conn = Connection::open_in_memory().expect("failed to open in-memory database");
        run_migrations(&conn).expect("failed to run migrations");

        let session = Session {
            id: "session-1".to_string(),
            title: "Demo Session".to_string(),
            manual_title: None,
            pinned: false,
            status: SessionStatus::Active,
            current_intent: "inspect".to_string(),
            current_object_type: "source".to_string(),
            current_object_id: "source-1".to_string(),
            summary: "Demo session summary".to_string(),
            started_at: "2026-03-25T00:00:00Z".to_string(),
            updated_at: "2026-03-25T00:00:00Z".to_string(),
            last_user_message_at: "2026-03-25T00:00:00Z".to_string(),
            last_run_at: "2026-03-25T00:00:00Z".to_string(),
            last_compacted_at: "2026-03-25T00:00:00Z".to_string(),
            metadata_json: "{}".to_string(),
        };

        insert_session(&conn, &session).expect("failed to insert session");

        let sessions = list_sessions(&conn).expect("failed to list sessions");

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "session-1");
        assert_eq!(sessions[0].status.as_str(), "active");
    }

    #[test]
    fn gets_session_by_id() {
        let conn = Connection::open_in_memory().expect("failed to open in-memory database");
        run_migrations(&conn).expect("failed to run migrations");

        let session = Session {
            id: "session-lookup".to_string(),
            title: "Lookup Session".to_string(),
            manual_title: None,
            pinned: false,
            status: SessionStatus::Active,
            current_intent: "idle".to_string(),
            current_object_type: "none".to_string(),
            current_object_id: "none".to_string(),
            summary: "Lookup summary".to_string(),
            started_at: "2026-03-29T00:00:00Z".to_string(),
            updated_at: "2026-03-29T00:00:00Z".to_string(),
            last_user_message_at: "2026-03-29T00:00:00Z".to_string(),
            last_run_at: "2026-03-29T00:00:00Z".to_string(),
            last_compacted_at: "2026-03-29T00:00:00Z".to_string(),
            metadata_json: "{}".to_string(),
        };

        insert_session(&conn, &session).expect("failed to insert session");

        let loaded = get_session_by_id(&conn, "session-lookup")
            .expect("query should succeed")
            .expect("session should exist");

        assert_eq!(loaded.id, "session-lookup");
        assert_eq!(loaded.title, "Lookup Session");
    }

    #[test]
    fn updates_existing_session() {
        let conn = Connection::open_in_memory().expect("failed to open in-memory database");
        run_migrations(&conn).expect("failed to run migrations");

        let mut session = Session {
            id: "session-update".to_string(),
            title: "Update Session".to_string(),
            manual_title: None,
            pinned: false,
            status: SessionStatus::Active,
            current_intent: "idle".to_string(),
            current_object_type: "none".to_string(),
            current_object_id: "none".to_string(),
            summary: "Before update".to_string(),
            started_at: "2026-03-29T00:00:00Z".to_string(),
            updated_at: "2026-03-29T00:00:00Z".to_string(),
            last_user_message_at: "2026-03-29T00:00:00Z".to_string(),
            last_run_at: "2026-03-29T00:00:00Z".to_string(),
            last_compacted_at: "2026-03-29T00:00:00Z".to_string(),
            metadata_json: "{}".to_string(),
        };

        insert_session(&conn, &session).expect("failed to insert session");

        session.current_intent = "general_reply".to_string();
        session.summary = "After update".to_string();
        session.updated_at = "2026-03-29T00:01:00Z".to_string();

        update_session(&conn, &session).expect("update should succeed");

        let loaded = get_session_by_id(&conn, "session-update")
            .expect("query should succeed")
            .expect("session should exist");

        assert_eq!(loaded.current_intent, "general_reply");
        assert_eq!(loaded.summary, "After update");
        assert_eq!(loaded.updated_at, "2026-03-29T00:01:00Z");
    }

    #[test]
    fn deletes_existing_session() {
        let conn = Connection::open_in_memory().expect("failed to open in-memory database");
        run_migrations(&conn).expect("failed to run migrations");

        let session = Session {
            id: "session-delete".to_string(),
            title: "Delete Session".to_string(),
            manual_title: None,
            pinned: false,
            status: SessionStatus::Active,
            current_intent: "idle".to_string(),
            current_object_type: "none".to_string(),
            current_object_id: "none".to_string(),
            summary: "Delete summary".to_string(),
            started_at: "2026-03-29T00:00:00Z".to_string(),
            updated_at: "2026-03-29T00:00:00Z".to_string(),
            last_user_message_at: "2026-03-29T00:00:00Z".to_string(),
            last_run_at: "2026-03-29T00:00:00Z".to_string(),
            last_compacted_at: "2026-03-29T00:00:00Z".to_string(),
            metadata_json: "{}".to_string(),
        };

        insert_session(&conn, &session).expect("failed to insert session");
        delete_session(&conn, "session-delete").expect("failed to delete session");

        let loaded = get_session_by_id(&conn, "session-delete").expect("query should succeed");

        assert!(loaded.is_none());
    }
}
