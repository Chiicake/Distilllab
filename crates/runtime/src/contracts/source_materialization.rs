use crate::contracts::RunInput;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceOriginKind {
    SessionMessage,
    Attachment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaterializeDedupePolicy {
    SkipExistingForRun,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializeSourcesInput {
    pub run_id: String,
    pub run_input: RunInput,
    pub allow_empty_message_source: bool,
    pub dedupe_policy: MaterializeDedupePolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializedSourceRef {
    pub source_id: String,
    pub source_kind: String,
    pub origin_kind: SourceOriginKind,
    pub origin_key: String,
    pub attachment_id: Option<String>,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializeSkip {
    pub origin_key: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializeFailure {
    pub origin_key: String,
    pub reason: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializeSourcesResult {
    pub run_id: String,
    pub created_sources: Vec<MaterializedSourceRef>,
    pub skipped_sources: Vec<MaterializeSkip>,
    pub failed_sources: Vec<MaterializeFailure>,
    pub summary: String,
    pub can_continue: bool,
}

#[cfg(test)]
mod tests {
    use super::{
        MaterializeDedupePolicy, MaterializeFailure, MaterializeSkip, MaterializeSourcesInput,
        MaterializeSourcesResult, MaterializedSourceRef, SourceOriginKind,
    };
    use schema::AttachmentRef;

    use crate::contracts::RunInput;

    #[test]
    fn materialize_sources_input_carries_run_and_run_input_context() {
        let attachment = AttachmentRef {
            attachment_id: "attachment-1".to_string(),
            kind: "file_path".to_string(),
            name: "runtime-notes.md".to_string(),
            mime_type: "text/markdown".to_string(),
            path_or_locator: "/tmp/runtime-notes.md".to_string(),
            size: 128,
            metadata_json: "{}".to_string(),
        };

        let input = MaterializeSourcesInput {
            run_id: "run-1".to_string(),
            run_input: RunInput {
                session_id: "session-1".to_string(),
                trigger_message: "Import these notes".to_string(),
                attachment_refs: vec![attachment],
                current_object_type: None,
                current_object_id: None,
                decision_summary: "Distill work material via import_and_distill".to_string(),
            },
            allow_empty_message_source: false,
            dedupe_policy: MaterializeDedupePolicy::SkipExistingForRun,
        };

        assert_eq!(input.run_id, "run-1");
        assert_eq!(input.run_input.session_id, "session-1");
        assert_eq!(input.run_input.attachment_refs.len(), 1);
        assert!(!input.allow_empty_message_source);
        assert!(matches!(
            input.dedupe_policy,
            MaterializeDedupePolicy::SkipExistingForRun
        ));
    }

    #[test]
    fn materialize_sources_result_tracks_created_skipped_and_failed_origins() {
        let result = MaterializeSourcesResult {
            run_id: "run-1".to_string(),
            created_sources: vec![MaterializedSourceRef {
                source_id: "source-1".to_string(),
                source_kind: "attachment_file".to_string(),
                origin_kind: SourceOriginKind::Attachment,
                origin_key: "attachment:attachment-1".to_string(),
                attachment_id: Some("attachment-1".to_string()),
                display_name: "runtime-notes.md".to_string(),
            }],
            skipped_sources: vec![MaterializeSkip {
                origin_key: "session-message:session-1:abcd1234".to_string(),
                reason: "already_materialized_for_run".to_string(),
            }],
            failed_sources: vec![MaterializeFailure {
                origin_key: "attachment:attachment-2".to_string(),
                reason: "missing_controlled_copy".to_string(),
                detail: Some("/tmp/missing.md".to_string()),
            }],
            summary: "materialized 1 sources (1 skipped, 1 failed)".to_string(),
            can_continue: true,
        };

        assert_eq!(result.run_id, "run-1");
        assert_eq!(result.created_sources.len(), 1);
        assert_eq!(result.skipped_sources.len(), 1);
        assert_eq!(result.failed_sources.len(), 1);
        assert!(result.can_continue);
        assert_eq!(
            result.created_sources[0].origin_key,
            "attachment:attachment-1"
        );
    }
}
