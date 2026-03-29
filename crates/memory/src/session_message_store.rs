use rusqlite::{params, types::Type, Connection, Error, Result};
use schema::{SessionMessage, SessionMessageRole};

pub fn insert_session_message(conn: &Connection, message: &SessionMessage) -> Result<()> {
    conn.execute(
        "INSERT INTO session_messages (id, session_id, run_id, message_type, role, content, data_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            message.id,
            message.session_id,
            message.run_id,
            message.message_type,
            message.role.as_str(),
            message.content,
            message.data_json,
            message.created_at,
        ],
    )?;

    Ok(())
}

pub fn list_session_messages_for_session(
    conn: &Connection,
    session_id: &str,
) -> Result<Vec<SessionMessage>> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, run_id, message_type, role, content, data_json, created_at FROM session_messages WHERE session_id = ?1 ORDER BY created_at ASC, id ASC",
    )?;

    let message_iter = stmt.query_map([session_id], |row| {
        let role_str: String = row.get(4)?;
        let role = SessionMessageRole::from_str(&role_str).ok_or_else(|| {
            Error::FromSqlConversionFailure(
                4,
                Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid session message role: {role_str}"),
                )),
            )
        })?;

        Ok(SessionMessage {
            id: row.get(0)?,
            session_id: row.get(1)?,
            run_id: row.get(2)?,
            message_type: row.get(3)?,
            role,
            content: row.get(5)?,
            data_json: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;

    let mut messages = Vec::new();
    for message in message_iter {
        messages.push(message?);
    }

    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::run_migrations;
    use schema::{SessionMessage, SessionMessageRole};

    #[test]
    fn inserts_and_lists_session_messages_for_a_session() {
        let conn = Connection::open_in_memory().expect("failed to open in-memory database");
        run_migrations(&conn).expect("failed to run migrations");

        let message = SessionMessage {
            id: "message-1".to_string(),
            session_id: "session-1".to_string(),
            run_id: None,
            message_type: "user_message".to_string(),
            role: SessionMessageRole::User,
            content: "Hello Distilllab".to_string(),
            data_json: "{}".to_string(),
            created_at: "2026-03-29T00:00:00Z".to_string(),
        };

        insert_session_message(&conn, &message).expect("failed to insert session message");

        let messages = list_session_messages_for_session(&conn, "session-1")
            .expect("failed to list session messages");

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].id, "message-1");
        assert_eq!(messages[0].role.as_str(), "user");
        assert_eq!(messages[0].content, "Hello Distilllab");
    }
}
