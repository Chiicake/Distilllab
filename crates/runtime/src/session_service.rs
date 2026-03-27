use crate::app::AppRuntime;
use chrono::Utc;
use memory::db::open_database;
use memory::migrations::run_migrations;
use memory::session_store::{insert_session, list_sessions as memory_list_sessions};
use schema::{Session, SessionStatus};
use uuid::Uuid;

pub fn create_demo_session(runtime: &AppRuntime) -> Result<Session, Box<dyn std::error::Error>> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let now = Utc::now().to_string();
    let session = Session {
        id: format!("session-{}", Uuid::new_v4()),
        title: "Demo Session".to_string(),
        status: SessionStatus::Active,
        current_intent: "idle".to_string(),
        current_object_type: "none".to_string(),
        current_object_id: "none".to_string(),
        summary: "Demo session created".to_string(),
        started_at: now.clone(),
        updated_at: now.clone(),
        last_user_message_at: now.clone(),
        last_run_at: now.clone(),
        last_compacted_at: now,
        metadata_json: "{}".to_string(),
    };

    insert_session(&conn, &session)?;
    Ok(session)
}

pub fn list_sessions(runtime: &AppRuntime) -> Result<Vec<Session>, Box<dyn std::error::Error>> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let sessions = memory_list_sessions(&conn)?;
    Ok(sessions)
}
