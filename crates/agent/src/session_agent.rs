use crate::{AgentDefinition, AgentError};

pub fn session_agent_definition() -> AgentDefinition {
    AgentDefinition {
        id: "agent-session".to_string(),
        key: "session_agent".to_string(),
        name: "Session Agent".to_string(),
        kind: "session".to_string(),
        description: "Distilllab homepage entry agent that understands user intent and decides the next high-level action.".to_string(),
        responsibility_summary: "Reads the current session and recent timeline, identifies user intent and primary object, then decides whether to reply directly, ask for clarification, or create a run. It does not execute the workflow itself.".to_string(),
        status: "active".to_string(),
        system_prompt: "You are the Session Agent for Distilllab. Understand the current session state, identify user intent, and decide the next high-level action in a structured way.".to_string(),
        default_model_profile: "reasoning_default".to_string(),
        allowed_tool_keys: vec![
            "list_sources".to_string(),
            "list_projects".to_string(),
            "list_runs".to_string(),
            "get_session".to_string(),
            "get_project".to_string(),
            "get_asset".to_string(),
        ],
        input_object_types: vec![
            "session".to_string(),
            "session_message".to_string(),
            "source".to_string(),
            "project".to_string(),
            "asset".to_string(),
            "run".to_string(),
        ],
        output_object_types: vec!["session_message".to_string(), "run".to_string()],
        can_create_run_types: vec![
            "import_and_distill".to_string(),
            "deepening".to_string(),
            "compose_and_verify".to_string(),
        ],
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionActionType {
    DirectReply,
    RequestClarification,
    CreateRun,
}

#[derive(Debug, Clone)]
pub struct SessionAgentDecision {
    pub intent: String,
    pub primary_object_type: Option<String>,
    pub primary_object_id: Option<String>,
    pub action_type: SessionActionType,
    pub reply_text: String,
    pub suggested_run_type: Option<String>,
    pub session_summary: Option<String>,
}

pub trait SessionAgent {
    fn decide(&self, user_message: &str) -> Result<SessionAgentDecision, AgentError>;
}

#[cfg(test)]
mod tests {
    use super::{session_agent_definition, SessionActionType, SessionAgentDecision};

    #[test]
    fn session_agent_definition_exposes_expected_defaults() {
        let definition = session_agent_definition();

        assert_eq!(definition.key, "session_agent");
        assert_eq!(definition.kind, "session");
        assert_eq!(definition.default_model_profile, "reasoning_default");
        assert!(definition
            .can_create_run_types
            .contains(&"import_and_distill".to_string()));
    }

    #[test]
    fn session_agent_decision_uses_structured_action_type() {
        let decision = SessionAgentDecision {
            intent: "import_material".to_string(),
            primary_object_type: Some("source".to_string()),
            primary_object_id: None,
            action_type: SessionActionType::CreateRun,
            reply_text: "I will start an import and distill run.".to_string(),
            suggested_run_type: Some("import_and_distill".to_string()),
            session_summary: Some("Preparing to import material".to_string()),
        };

        assert_eq!(decision.action_type, SessionActionType::CreateRun);
    }
}
