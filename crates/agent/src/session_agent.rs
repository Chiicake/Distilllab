use crate::{AgentDefinition, AgentError};
use schema::{Session, SessionMessage};

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
pub struct SessionAgentInput {
    pub session: Session,
    pub recent_messages: Vec<SessionMessage>,
    pub user_message: String,
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
    fn decide(&self, input: SessionAgentInput) -> Result<SessionAgentDecision, AgentError>;
}

pub struct BasicSessionAgent;

impl SessionAgent for BasicSessionAgent {
    fn decide(&self, _input: SessionAgentInput) -> Result<SessionAgentDecision, AgentError> {
        Ok(SessionAgentDecision {
            intent: "general_reply".to_string(),
            primary_object_type: None,
            primary_object_id: None,
            action_type: SessionActionType::DirectReply,
            reply_text: "Hello! I am ready to help with your Distilllab session.".to_string(),
            suggested_run_type: None,
            session_summary: Some("General session assistance".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BasicSessionAgent, SessionActionType, SessionAgent, SessionAgentDecision,
        SessionAgentInput, session_agent_definition,
    };
    use schema::{Session, SessionMessage, SessionMessageRole, SessionStatus};

    #[test]
    fn session_agent_definition_exposes_expected_defaults() {
        let definition = session_agent_definition();

        assert_eq!(definition.key, "session_agent");
        assert_eq!(definition.kind, "session");
        assert_eq!(definition.default_model_profile, "reasoning_default");
        assert!(
            definition
                .can_create_run_types
                .contains(&"import_and_distill".to_string())
        );
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

    #[test]
    fn session_agent_input_preserves_structured_session_context() {
        let session = Session {
            id: "session-1".to_string(),
            title: "Test Session".to_string(),
            status: SessionStatus::Active,
            current_intent: "idle".to_string(),
            current_object_type: "none".to_string(),
            current_object_id: "none".to_string(),
            summary: "Testing session agent input".to_string(),
            started_at: "2026-03-28T00:00:00Z".to_string(),
            updated_at: "2026-03-28T00:00:00Z".to_string(),
            last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
            last_run_at: "2026-03-28T00:00:00Z".to_string(),
            last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
            metadata_json: "{}".to_string(),
        };

        let recent_messages = vec![SessionMessage {
            id: "message-1".to_string(),
            session_id: "session-1".to_string(),
            run_id: None,
            message_type: "user_message".to_string(),
            role: SessionMessageRole::User,
            content: "Help me import my notes".to_string(),
            data_json: "{}".to_string(),
            created_at: "2026-03-28T00:00:00Z".to_string(),
        }];

        let input = SessionAgentInput {
            session,
            recent_messages,
            user_message: "Import these notes".to_string(),
        };

        assert_eq!(input.session.id, "session-1");
        assert_eq!(input.recent_messages.len(), 1);
        assert_eq!(input.user_message, "Import these notes");
    }

    #[test]
    fn basic_session_agent_returns_structured_direct_reply_decision() {
        let agent = BasicSessionAgent;

        let input = SessionAgentInput {
            session: Session {
                id: "session-1".to_string(),
                title: "Test Session".to_string(),
                status: SessionStatus::Active,
                current_intent: "idle".to_string(),
                current_object_type: "none".to_string(),
                current_object_id: "none".to_string(),
                summary: "Testing basic session agent".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![],
            user_message: "Hello Distilllab".to_string(),
        };

        let decision = agent
            .decide(input)
            .expect("basic session agent should decide");

        assert_eq!(decision.action_type, SessionActionType::DirectReply);
        assert_eq!(decision.intent, "general_reply");
        assert_eq!(
            decision.reply_text,
            "Hello! I am ready to help with your Distilllab session."
        );
    }
}
