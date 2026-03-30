use schema::{AttachmentRef, SessionIntake};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunInput {
    pub session_id: String,
    pub trigger_message: String,
    pub attachment_refs: Vec<AttachmentRef>,
    pub current_object_type: Option<String>,
    pub current_object_id: Option<String>,
    pub decision_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistillRunStepPreview {
    pub step_key: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunHandoffPreview {
    pub run_type: String,
    pub primary_object_type: Option<String>,
    pub primary_object_id: Option<String>,
    pub summary: String,
    pub planned_steps: Vec<DistillRunStepPreview>,
}

#[derive(Debug, Clone)]
pub struct SessionIntakePreview {
    pub decision: agent::SessionAgentDecision,
    pub run_handoff_preview: Option<RunHandoffPreview>,
}

#[cfg(test)]
mod tests {
    use super::{AttachmentRef, RunInput, SessionIntake};

    #[test]
    fn session_intake_and_run_input_model_different_layers_of_input() {
        let attachment = AttachmentRef {
            attachment_id: "attachment-1".to_string(),
            kind: "file_path".to_string(),
            name: "runtime-notes.md".to_string(),
            mime_type: "text/markdown".to_string(),
            path_or_locator: "/tmp/runtime-notes.md".to_string(),
            size: 128,
            metadata_json: "{}".to_string(),
        };

        let intake = SessionIntake {
            session_id: "session-1".to_string(),
            user_message: "Please distill these work notes".to_string(),
            attachments: vec![attachment.clone()],
            current_object_type: None,
            current_object_id: None,
        };

        let run_input = RunInput {
            session_id: "session-1".to_string(),
            trigger_message: "Please distill these work notes".to_string(),
            attachment_refs: vec![attachment],
            current_object_type: None,
            current_object_id: None,
            decision_summary: "Distill work material via import_and_distill".to_string(),
        };

        assert_eq!(intake.session_id, "session-1");
        assert_eq!(intake.attachments.len(), 1);
        assert_eq!(run_input.attachment_refs.len(), 1);
        assert_eq!(
            run_input.decision_summary,
            "Distill work material via import_and_distill"
        );
    }
}
