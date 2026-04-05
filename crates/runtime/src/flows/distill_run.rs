use crate::app::AppRuntime;
use crate::contracts::{
    DistillRunStepPreview, MaterializeFailure, MaterializeSkip, MaterializeSourcesResult,
    MaterializedSourceRef, RunHandoffPreview, RunInput, SourceOriginKind,
};
use crate::runs::import_and_distill_step_definitions;
use crate::services::{
    create_attachment_source, create_message_source, find_source_for_run_origin,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

type FlowError = Box<dyn std::error::Error + Send + Sync>;

pub fn build_import_and_distill_handoff_preview(
    primary_object_type: Option<String>,
    primary_object_id: Option<String>,
) -> RunHandoffPreview {
    RunHandoffPreview {
        run_type: "import_and_distill".to_string(),
        primary_object_type,
        primary_object_id,
        summary: "Previewing the import-and-distill workflow for this work material.".to_string(),
        planned_steps: import_and_distill_step_definitions()
            .iter()
            .map(|step| DistillRunStepPreview {
                step_key: step.step_key.to_string(),
                summary: step.summary.to_string(),
            })
            .collect(),
    }
}

pub fn execute_materialize_sources(
    runtime: &AppRuntime,
    run_id: &str,
    run_input: RunInput,
) -> Result<MaterializeSourcesResult, FlowError> {
    let mut created_sources = Vec::new();
    let mut skipped_sources = Vec::new();
    let mut failed_sources = Vec::new();

    let trimmed_message = run_input.trigger_message.trim();
    if !trimmed_message.is_empty() {
        let origin_key = origin_key_for_message(&run_input.session_id, trimmed_message);
        if find_source_for_run_origin(runtime, run_id, &origin_key)?.is_some() {
            skipped_sources.push(MaterializeSkip {
                origin_key,
                reason: "already_materialized_for_run".to_string(),
            });
        } else {
            let source = create_message_source(
                runtime,
                run_id,
                &run_input.session_id,
                trimmed_message,
                &origin_key,
            )?;

            created_sources.push(MaterializedSourceRef {
                source_id: source.id,
                source_kind: source.source_type.as_str().to_string(),
                origin_kind: SourceOriginKind::SessionMessage,
                origin_key,
                attachment_id: None,
                display_name: source.title,
            });
        }
    }

    for attachment in &run_input.attachment_refs {
        let origin_key = origin_key_for_attachment(&attachment.attachment_id);

        if !std::path::Path::new(&attachment.path_or_locator).exists() {
            failed_sources.push(MaterializeFailure {
                origin_key,
                reason: "missing_controlled_copy".to_string(),
                detail: Some(attachment.path_or_locator.clone()),
            });
            continue;
        }

        if find_source_for_run_origin(runtime, run_id, &origin_key)?.is_some() {
            skipped_sources.push(MaterializeSkip {
                origin_key,
                reason: "already_materialized_for_run".to_string(),
            });
            continue;
        }

        let source = create_attachment_source(
            runtime,
            run_id,
            &run_input.session_id,
            attachment,
            &origin_key,
        )?;

        created_sources.push(MaterializedSourceRef {
            source_id: source.id,
            source_kind: source.source_type.as_str().to_string(),
            origin_kind: SourceOriginKind::Attachment,
            origin_key,
            attachment_id: Some(attachment.attachment_id.clone()),
            display_name: source.title,
        });
    }

    let summary = format!(
        "materialized {} sources ({} skipped, {} failed)",
        created_sources.len(),
        skipped_sources.len(),
        failed_sources.len()
    );
    let can_continue = !created_sources.is_empty();

    Ok(MaterializeSourcesResult {
        run_id: run_id.to_string(),
        created_sources,
        skipped_sources,
        failed_sources,
        summary,
        can_continue,
    })
}

fn origin_key_for_message(session_id: &str, message_text: &str) -> String {
    let digest = short_sha256_hex(message_text.as_bytes());
    format!("session-message:{session_id}:{digest}")
}

fn origin_key_for_attachment(attachment_id: &str) -> String {
    format!("attachment:{attachment_id}")
}

fn short_sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("{:08x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::{
        build_import_and_distill_handoff_preview, execute_materialize_sources,
        origin_key_for_attachment, origin_key_for_message,
    };
    use crate::app::AppRuntime;
    use crate::contracts::RunInput;
    use schema::AttachmentRef;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn builds_import_and_distill_preview_with_materialize_step_first() {
        let preview = build_import_and_distill_handoff_preview(None, None);

        assert_eq!(preview.run_type, "import_and_distill");
        assert_eq!(preview.planned_steps[0].step_key, "materialize_sources");
    }

    #[test]
    fn materializes_message_and_attachment_sources_for_run() {
        let db_path = format!("/tmp/distilllab-materialize-test-{}.db", Uuid::new_v4());
        let runtime = AppRuntime::new(db_path.clone());
        let attachment_path = format!(
            "/tmp/distilllab-materialize-attachment-{}.md",
            Uuid::new_v4()
        );
        fs::write(&attachment_path, "# Notes\nhello").expect("failed to write attachment fixture");

        let run_input = RunInput {
            session_id: "session-1".to_string(),
            trigger_message: "Please distill these work notes".to_string(),
            attachment_refs: vec![AttachmentRef {
                attachment_id: "attachment-1".to_string(),
                kind: "file_path".to_string(),
                name: "notes.md".to_string(),
                mime_type: "text/markdown".to_string(),
                path_or_locator: attachment_path.clone(),
                size: 64,
                metadata_json: "{}".to_string(),
            }],
            current_object_type: None,
            current_object_id: None,
            decision_summary: "Distill work material via import_and_distill".to_string(),
        };

        let result = execute_materialize_sources(&runtime, "run-1", run_input)
            .expect("failed to materialize sources");

        assert_eq!(result.created_sources.len(), 2);
        assert_eq!(result.skipped_sources.len(), 0);
        assert_eq!(result.failed_sources.len(), 0);
        assert!(result.can_continue);

        let _ = fs::remove_file(attachment_path);
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn skips_empty_message_when_no_text_source_should_be_created() {
        let db_path = format!("/tmp/distilllab-materialize-test-{}.db", Uuid::new_v4());
        let runtime = AppRuntime::new(db_path.clone());

        let run_input = RunInput {
            session_id: "session-1".to_string(),
            trigger_message: "   ".to_string(),
            attachment_refs: vec![],
            current_object_type: None,
            current_object_id: None,
            decision_summary: "Distill work material via import_and_distill".to_string(),
        };

        let result = execute_materialize_sources(&runtime, "run-1", run_input)
            .expect("failed to materialize sources");

        assert_eq!(result.created_sources.len(), 0);
        assert_eq!(result.failed_sources.len(), 0);
        assert!(!result.can_continue);

        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn records_failure_for_missing_attachment_copy() {
        let db_path = format!("/tmp/distilllab-materialize-test-{}.db", Uuid::new_v4());
        let runtime = AppRuntime::new(db_path.clone());

        let run_input = RunInput {
            session_id: "session-1".to_string(),
            trigger_message: "".to_string(),
            attachment_refs: vec![AttachmentRef {
                attachment_id: "attachment-1".to_string(),
                kind: "file_path".to_string(),
                name: "missing.md".to_string(),
                mime_type: "text/markdown".to_string(),
                path_or_locator: "/tmp/does-not-exist.md".to_string(),
                size: 64,
                metadata_json: "{}".to_string(),
            }],
            current_object_type: None,
            current_object_id: None,
            decision_summary: "Distill work material via import_and_distill".to_string(),
        };

        let result = execute_materialize_sources(&runtime, "run-1", run_input)
            .expect("failed to materialize sources");

        assert_eq!(result.created_sources.len(), 0);
        assert_eq!(result.failed_sources.len(), 1);
        assert_eq!(result.failed_sources[0].reason, "missing_controlled_copy");

        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn skips_duplicate_message_and_attachment_origins_for_same_run() {
        let db_path = format!("/tmp/distilllab-materialize-test-{}.db", Uuid::new_v4());
        let runtime = AppRuntime::new(db_path.clone());
        let attachment_path = format!(
            "/tmp/distilllab-materialize-attachment-{}.md",
            Uuid::new_v4()
        );
        fs::write(&attachment_path, "# Notes\nhello").expect("failed to write attachment fixture");

        let run_input = RunInput {
            session_id: "session-1".to_string(),
            trigger_message: "Please distill these work notes".to_string(),
            attachment_refs: vec![AttachmentRef {
                attachment_id: "attachment-1".to_string(),
                kind: "file_path".to_string(),
                name: "notes.md".to_string(),
                mime_type: "text/markdown".to_string(),
                path_or_locator: attachment_path.clone(),
                size: 64,
                metadata_json: "{}".to_string(),
            }],
            current_object_type: None,
            current_object_id: None,
            decision_summary: "Distill work material via import_and_distill".to_string(),
        };

        let first = execute_materialize_sources(&runtime, "run-1", run_input.clone())
            .expect("first materialization should succeed");
        let second = execute_materialize_sources(&runtime, "run-1", run_input)
            .expect("second materialization should succeed");

        assert_eq!(first.created_sources.len(), 2);
        assert_eq!(second.created_sources.len(), 0);
        assert_eq!(second.skipped_sources.len(), 2);

        let _ = fs::remove_file(attachment_path);
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn builds_stable_origin_keys() {
        let message_key = origin_key_for_message("session-1", "hello world");
        let attachment_key = origin_key_for_attachment("attachment-1");

        assert!(message_key.starts_with("session-message:session-1:"));
        assert_eq!(attachment_key, "attachment:attachment-1");
    }
}
