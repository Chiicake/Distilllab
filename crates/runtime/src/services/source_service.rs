use crate::app::AppRuntime;
use chrono::Utc;
use memory::db::open_database;
use memory::migrations::run_migrations;
use memory::run_store::insert_run;
use memory::source_store::{
    get_source_by_run_origin, insert_source, list_sources as memory_list_sources,
    list_sources_by_run,
};
use schema::run::RunType;
use schema::{AttachmentRef, Run, RunState, Source, SourceType};
use uuid::Uuid;

type ServiceError = Box<dyn std::error::Error + Send + Sync>;

pub fn create_demo_source(runtime: &AppRuntime) -> Result<Source, ServiceError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let source = Source {
        id: format!("source-{}", Uuid::new_v4()),
        source_type: SourceType::Document,
        title: "Demo Source".to_string(),
        run_id: None,
        origin_key: None,
        locator: None,
        metadata_json: "{}".to_string(),
        created_at: Utc::now().to_string(),
    };

    insert_source(&conn, &source)?;

    let run = Run {
        id: format!("demo-source-run-{}", Uuid::new_v4()),
        run_type: RunType::Demo,
        status: RunState::Completed,
        primary_object_type: "source".to_string(),
        primary_object_id: source.id.to_string(),
        created_at: Utc::now().to_string(),
    };

    insert_run(&conn, &run)?;

    Ok(source)
}

pub fn list_sources(runtime: &AppRuntime) -> Result<Vec<Source>, ServiceError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let sources = memory_list_sources(&conn)?;
    Ok(sources)
}

pub fn find_source_for_run_origin(
    runtime: &AppRuntime,
    run_id: &str,
    origin_key: &str,
) -> Result<Option<Source>, ServiceError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let source = get_source_by_run_origin(&conn, run_id, origin_key)?;
    Ok(source)
}

pub fn create_message_source(
    runtime: &AppRuntime,
    run_id: &str,
    session_id: &str,
    message_text: &str,
    origin_key: &str,
) -> Result<Source, ServiceError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let source = Source {
        id: format!("source-{}", Uuid::new_v4()),
        source_type: SourceType::Session,
        title: "Session message".to_string(),
        run_id: Some(run_id.to_string()),
        origin_key: Some(origin_key.to_string()),
        locator: None,
        metadata_json: format!(
            r#"{{"session_id":"{}","message_length":{}}}"#,
            session_id,
            message_text.chars().count()
        ),
        created_at: Utc::now().to_string(),
    };

    insert_source(&conn, &source)?;
    Ok(source)
}

pub fn create_attachment_source(
    runtime: &AppRuntime,
    run_id: &str,
    session_id: &str,
    attachment: &AttachmentRef,
    origin_key: &str,
) -> Result<Source, ServiceError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let source = Source {
        id: format!("source-{}", Uuid::new_v4()),
        source_type: SourceType::Document,
        title: attachment.name.clone(),
        run_id: Some(run_id.to_string()),
        origin_key: Some(origin_key.to_string()),
        locator: Some(attachment.path_or_locator.clone()),
        metadata_json: format!(
            r#"{{"session_id":"{}","attachment_id":"{}","mime_type":"{}","size":{}}}"#,
            session_id, attachment.attachment_id, attachment.mime_type, attachment.size
        ),
        created_at: Utc::now().to_string(),
    };

    insert_source(&conn, &source)?;
    Ok(source)
}

pub fn list_sources_for_run(
    runtime: &AppRuntime,
    run_id: &str,
) -> Result<Vec<Source>, ServiceError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let sources = list_sources_by_run(&conn, run_id)?;
    Ok(sources)
}

#[cfg(test)]
mod tests {
    use super::{
        create_attachment_source, create_message_source, find_source_for_run_origin,
        list_sources_for_run,
    };
    use crate::app::AppRuntime;
    use schema::AttachmentRef;
    use uuid::Uuid;

    #[test]
    fn creates_message_source_for_run() {
        let db_path = format!("/tmp/distilllab-source-service-test-{}.db", Uuid::new_v4());
        let runtime = AppRuntime::new(db_path.clone());

        let source = create_message_source(
            &runtime,
            "run-1",
            "session-1",
            "Please distill these notes",
            "session-message:session-1:abcd1234",
        )
        .expect("failed to create message source");

        assert_eq!(source.source_type.as_str(), "session");
        assert_eq!(source.run_id.as_deref(), Some("run-1"));
        assert_eq!(
            source.origin_key.as_deref(),
            Some("session-message:session-1:abcd1234")
        );
        assert!(source.locator.is_none());
        assert!(source.metadata_json.contains("session-1"));

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn creates_attachment_source_with_controlled_copy_locator() {
        let db_path = format!("/tmp/distilllab-source-service-test-{}.db", Uuid::new_v4());
        let runtime = AppRuntime::new(db_path.clone());

        let attachment = AttachmentRef {
            attachment_id: "attachment-1".to_string(),
            kind: "file_path".to_string(),
            name: "notes.md".to_string(),
            mime_type: "text/markdown".to_string(),
            path_or_locator: "/tmp/distilllab/attachments/notes.md".to_string(),
            size: 512,
            metadata_json: "{}".to_string(),
        };

        let source = create_attachment_source(
            &runtime,
            "run-1",
            "session-1",
            &attachment,
            "attachment:attachment-1",
        )
        .expect("failed to create attachment source");

        assert_eq!(source.source_type.as_str(), "document");
        assert_eq!(source.title, "notes.md");
        assert_eq!(source.run_id.as_deref(), Some("run-1"));
        assert_eq!(
            source.origin_key.as_deref(),
            Some("attachment:attachment-1")
        );
        assert_eq!(
            source.locator.as_deref(),
            Some("/tmp/distilllab/attachments/notes.md")
        );
        assert!(source.metadata_json.contains("attachment-1"));

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn finds_source_for_run_origin_when_existing_source_was_created() {
        let db_path = format!("/tmp/distilllab-source-service-test-{}.db", Uuid::new_v4());
        let runtime = AppRuntime::new(db_path.clone());

        create_message_source(
            &runtime,
            "run-1",
            "session-1",
            "Please distill these notes",
            "session-message:session-1:abcd1234",
        )
        .expect("failed to create message source");

        let found =
            find_source_for_run_origin(&runtime, "run-1", "session-message:session-1:abcd1234")
                .expect("failed to find source for run origin")
                .expect("source should exist");

        assert_eq!(found.run_id.as_deref(), Some("run-1"));
        assert_eq!(found.source_type.as_str(), "session");

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn lists_sources_for_run_after_creating_multiple_sources() {
        let db_path = format!("/tmp/distilllab-source-service-test-{}.db", Uuid::new_v4());
        let runtime = AppRuntime::new(db_path.clone());

        create_message_source(
            &runtime,
            "run-1",
            "session-1",
            "Please distill these notes",
            "session-message:session-1:abcd1234",
        )
        .expect("failed to create message source");

        let attachment = AttachmentRef {
            attachment_id: "attachment-1".to_string(),
            kind: "file_path".to_string(),
            name: "notes.md".to_string(),
            mime_type: "text/markdown".to_string(),
            path_or_locator: "/tmp/distilllab/attachments/notes.md".to_string(),
            size: 512,
            metadata_json: "{}".to_string(),
        };

        create_attachment_source(
            &runtime,
            "run-1",
            "session-1",
            &attachment,
            "attachment:attachment-1",
        )
        .expect("failed to create attachment source");

        let sources =
            list_sources_for_run(&runtime, "run-1").expect("failed to list sources for run");

        assert_eq!(sources.len(), 2);

        let _ = std::fs::remove_file(db_path);
    }
}
