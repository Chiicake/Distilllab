use crate::{
    send_chat_completion_request, AgentDefinition, AgentError, LlmProviderConfig,
    OpenAiCompatibleChatMessage, OpenAiCompatibleChatRequest,
};
use async_trait::async_trait;
use serde::Deserialize;
use schema::{Session, SessionMessage, SessionMessageRole};

pub fn session_agent_definition() -> AgentDefinition {
    AgentDefinition {
        id: "agent-session".to_string(),
        key: "session_agent".to_string(),
        name: "Session Agent".to_string(),
        kind: "session".to_string(),
        description: "Distilllab homepage entry agent that understands user intent and decides the next high-level action.".to_string(),
        responsibility_summary: "Reads the current session and recent timeline, identifies user intent and primary object, then decides whether to reply directly, ask for clarification, or create a run. It does not execute the workflow itself.".to_string(),
        status: "active".to_string(),
        system_prompt: "You are the Session Agent for Distilllab. Understand the current session state, identify user intent, and decide the next high-level action. Respond with valid JSON only. Do not include markdown fences or extra explanation. The JSON object must contain these fields: intent, action_type, reply_text, primary_object_type, primary_object_id, suggested_run_type, session_summary, tool_call_key. Intent must be one of: general_reply, import_material, deepen_understanding, compose_output. action_type must be one of: direct_reply, request_clarification, tool_call, create_run. Use tool_call_key only when action_type is tool_call. Use suggested_run_type only when action_type is create_run. Use null for optional fields when unknown.".to_string(),
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
    ToolCall,
    CreateRun,
}

impl SessionActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionActionType::DirectReply => "direct_reply",
            SessionActionType::RequestClarification => "request_clarification",
            SessionActionType::ToolCall => "tool_call",
            SessionActionType::CreateRun => "create_run",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionIntent {
    GeneralReply,
    ImportMaterial,
    DeepenUnderstanding,
    ComposeOutput,
}

impl SessionIntent {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionIntent::GeneralReply => "general_reply",
            SessionIntent::ImportMaterial => "import_material",
            SessionIntent::DeepenUnderstanding => "deepen_understanding",
            SessionIntent::ComposeOutput => "compose_output",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "general_reply" | "llm_direct_reply" => Some(SessionIntent::GeneralReply),
            "import_material" => Some(SessionIntent::ImportMaterial),
            "deepen_understanding" => Some(SessionIntent::DeepenUnderstanding),
            "compose_output" => Some(SessionIntent::ComposeOutput),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionAgentInput {
    pub session: Session,
    pub recent_messages: Vec<SessionMessage>,
    pub user_message: String,
}

#[derive(Debug, Clone)]
pub struct SessionAgentDecision {
    pub intent: SessionIntent,
    pub primary_object_type: Option<String>,
    pub primary_object_id: Option<String>,
    pub action_type: SessionActionType,
    pub tool_call_key: Option<String>,
    pub reply_text: String,
    pub suggested_run_type: Option<String>,
    pub session_summary: Option<String>,
}

#[async_trait]
pub trait SessionAgent {
    async fn decide(&self, input: SessionAgentInput) -> Result<SessionAgentDecision, AgentError>;
}

pub struct BasicSessionAgent;

#[async_trait]
impl SessionAgent for BasicSessionAgent {
    async fn decide(&self, input: SessionAgentInput) -> Result<SessionAgentDecision, AgentError> {
        let normalized_message = input.user_message.to_lowercase();
        let primary_object_type = match input.session.current_object_type.as_str() {
            "none" => None,
            other => Some(other.to_string()),
        };
        let primary_object_id = match input.session.current_object_id.as_str() {
            "none" => None,
            other => Some(other.to_string()),
        };

        if normalized_message.contains("import")
            || normalized_message.contains("upload")
            || normalized_message.contains("bring in")
            || normalized_message.contains("load these")
            || normalized_message.contains("here are my notes")
            || normalized_message.contains("these files")
            || normalized_message.contains("distill them")
            || normalized_message.contains("distillation run")
        {
            return Ok(SessionAgentDecision {
                intent: SessionIntent::ImportMaterial,
                primary_object_type,
                primary_object_id,
                action_type: SessionActionType::CreateRun,
                tool_call_key: None,
                reply_text: "I will start an import and distill run.".to_string(),
                suggested_run_type: Some("import_and_distill".to_string()),
                session_summary: Some("Preparing to import material".to_string()),
            });
        }

        if normalized_message.contains("deepen")
            || normalized_message.contains("follow-up")
            || normalized_message.contains("clarify")
            || normalized_message.contains("ask questions")
        {
            return Ok(SessionAgentDecision {
                intent: SessionIntent::DeepenUnderstanding,
                primary_object_type,
                primary_object_id,
                action_type: SessionActionType::CreateRun,
                tool_call_key: None,
                reply_text: "I will start a deepening run to explore this topic further.".to_string(),
                suggested_run_type: Some("deepening".to_string()),
                session_summary: Some("Preparing to deepen understanding".to_string()),
            });
        }

        if normalized_message.contains("write")
            || normalized_message.contains("summary")
            || normalized_message.contains("article")
            || normalized_message.contains("report")
            || normalized_message.contains("compose")
        {
            return Ok(SessionAgentDecision {
                intent: SessionIntent::ComposeOutput,
                primary_object_type,
                primary_object_id,
                action_type: SessionActionType::CreateRun,
                tool_call_key: None,
                reply_text: "I will prepare a compose and verify run for this output request.".to_string(),
                suggested_run_type: Some("compose_and_verify".to_string()),
                session_summary: Some("Preparing to compose an output".to_string()),
            });
        }

        Ok(SessionAgentDecision {
            intent: SessionIntent::GeneralReply,
            primary_object_type,
            primary_object_id,
            action_type: SessionActionType::DirectReply,
            tool_call_key: None,
            reply_text: "Hello! I am ready to help with your Distilllab session.".to_string(),
            suggested_run_type: None,
            session_summary: Some("General session assistance".to_string()),
        })
    }
}

pub struct LlmSessionAgent {
    pub client: reqwest::Client,
    pub config: LlmProviderConfig,
}

#[derive(Debug, Clone)]
pub struct LlmSessionAgentDebugResult {
    pub raw_output: String,
    pub decision: SessionAgentDecision,
}

#[derive(Debug, Deserialize)]
struct StructuredSessionAgentDecision {
    intent: String,
    action_type: String,
    tool_call_key: Option<String>,
    reply_text: String,
    primary_object_type: Option<String>,
    primary_object_id: Option<String>,
    suggested_run_type: Option<String>,
    session_summary: Option<String>,
}

impl LlmSessionAgent {
    pub fn new(config: LlmProviderConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    fn build_chat_messages(&self, input: &SessionAgentInput) -> Vec<OpenAiCompatibleChatMessage> {
        let system_context = format!(
            "{}\nCurrent session context:\nsession_id: {}\nsession_title: {}\ncurrent_intent: {}\ncurrent_object_type: {}\ncurrent_object_id: {}\nsession_summary: {}",
            session_agent_definition().system_prompt,
            input.session.id,
            input.session.title,
            input.session.current_intent,
            input.session.current_object_type,
            input.session.current_object_id,
            input.session.summary,
        );

        let mut messages = vec![OpenAiCompatibleChatMessage {
            role: "system".to_string(),
            content: system_context,
        }];

        messages.extend(self.few_shot_examples());

        for recent_message in &input.recent_messages {
            let role = match recent_message.role {
                SessionMessageRole::User => "user",
                SessionMessageRole::Assistant => "assistant",
                SessionMessageRole::System => "system",
            };

            messages.push(OpenAiCompatibleChatMessage {
                role: role.to_string(),
                content: recent_message.content.clone(),
            });
        }

        messages.push(OpenAiCompatibleChatMessage {
            role: "user".to_string(),
            content: input.user_message.clone(),
        });

        messages
    }

    fn few_shot_examples(&self) -> Vec<OpenAiCompatibleChatMessage> {
        vec![
            OpenAiCompatibleChatMessage {
                role: "user".to_string(),
                content: "Please import these notes into Distilllab".to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "assistant".to_string(),
                content: r#"{"intent":"import_material","action_type":"create_run","reply_text":"I will start an import and distill run.","primary_object_type":null,"primary_object_id":null,"suggested_run_type":"import_and_distill","session_summary":"Preparing to import material","tool_call_key":null}"#.to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "user".to_string(),
                content: "Write a concise article from this project".to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "assistant".to_string(),
                content: r#"{"intent":"compose_output","action_type":"create_run","reply_text":"I will prepare a compose and verify run for this output request.","primary_object_type":"project","primary_object_id":"project-current","suggested_run_type":"compose_and_verify","session_summary":"Preparing to compose an output","tool_call_key":null}"#.to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "user".to_string(),
                content: "Please deepen this topic and ask follow-up questions".to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "assistant".to_string(),
                content: r#"{"intent":"deepen_understanding","action_type":"create_run","reply_text":"I will start a deepening run to explore this topic further.","primary_object_type":"asset","primary_object_id":"asset-current","suggested_run_type":"deepening","session_summary":"Preparing to deepen understanding","tool_call_key":null}"#.to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "user".to_string(),
                content: "Search memory for related notes before answering".to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "assistant".to_string(),
                content: r#"{"intent":"general_reply","action_type":"tool_call","reply_text":"I will look up related notes before replying.","primary_object_type":null,"primary_object_id":null,"suggested_run_type":null,"session_summary":"Preparing a memory lookup before answering","tool_call_key":"search_memory"}"#.to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "user".to_string(),
                content: "What did we do so far?".to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "assistant".to_string(),
                content: r#"{"intent":"general_reply","action_type":"direct_reply","reply_text":"Here is a concise summary of the current session.","primary_object_type":null,"primary_object_id":null,"suggested_run_type":null,"session_summary":"Providing a direct session summary","tool_call_key":null}"#.to_string(),
            },
        ]
    }

    fn parse_action_type(action_type: &str) -> Option<SessionActionType> {
        match action_type {
            "direct_reply" => Some(SessionActionType::DirectReply),
            "request_clarification" => Some(SessionActionType::RequestClarification),
            "create_run" => Some(SessionActionType::CreateRun),
            _ => None,
        }
    }

    fn parse_structured_decision(reply_text: &str) -> Option<SessionAgentDecision> {
        let parsed = serde_json::from_str::<StructuredSessionAgentDecision>(reply_text).ok()?;
        let action_type = Self::parse_action_type(parsed.action_type.as_str())?;

        Some(SessionAgentDecision {
            intent: SessionIntent::from_str(parsed.intent.as_str())?,
            primary_object_type: parsed.primary_object_type,
            primary_object_id: parsed.primary_object_id,
            action_type,
            tool_call_key: parsed.tool_call_key,
            reply_text: parsed.reply_text,
            suggested_run_type: parsed.suggested_run_type,
            session_summary: parsed.session_summary,
        })
    }

    fn fallback_direct_reply_decision(reply_text: &str) -> SessionAgentDecision {
        SessionAgentDecision {
            intent: SessionIntent::GeneralReply,
            primary_object_type: None,
            primary_object_id: None,
            action_type: SessionActionType::DirectReply,
            tool_call_key: None,
            reply_text: reply_text.to_string(),
            suggested_run_type: None,
            session_summary: Some("LLM replied to the current session message".to_string()),
        }
    }

    pub async fn decide_with_debug(
        &self,
        input: SessionAgentInput,
    ) -> Result<LlmSessionAgentDebugResult, AgentError> {
        let messages = self.build_chat_messages(&input);

        let request = OpenAiCompatibleChatRequest {
            model: self.config.model.clone(),
            messages,
        };

        let response = send_chat_completion_request(&self.client, &self.config, &request).await?;

        let raw_output = response
            .first_message_content()
            .ok_or_else(|| {
                AgentError::Response("llm response did not contain assistant content".to_string())
            })?
            .to_string();

        let decision = Self::parse_structured_decision(&raw_output)
            .unwrap_or_else(|| Self::fallback_direct_reply_decision(&raw_output));

        Ok(LlmSessionAgentDebugResult {
            raw_output,
            decision,
        })
    }
}

#[async_trait]
impl SessionAgent for LlmSessionAgent {
    async fn decide(&self, input: SessionAgentInput) -> Result<SessionAgentDecision, AgentError> {
        Ok(self.decide_with_debug(input).await?.decision)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BasicSessionAgent, LlmSessionAgent, SessionActionType, SessionAgent,
        SessionAgentDecision, SessionAgentInput, SessionIntent, session_agent_definition,
    };
    use crate::LlmProviderConfig;
    use schema::{Session, SessionMessage, SessionMessageRole, SessionStatus};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

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
            intent: SessionIntent::ImportMaterial,
            primary_object_type: Some("source".to_string()),
            primary_object_id: None,
            action_type: SessionActionType::CreateRun,
            tool_call_key: None,
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

    #[tokio::test]
    async fn basic_session_agent_returns_structured_direct_reply_decision() {
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
            .await
            .expect("basic session agent should decide");

        assert_eq!(decision.action_type, SessionActionType::DirectReply);
        assert_eq!(decision.intent, SessionIntent::GeneralReply);
        assert_eq!(decision.tool_call_key, None);
        assert_eq!(
            decision.reply_text,
            "Hello! I am ready to help with your Distilllab session."
        );
    }

    #[tokio::test]
    async fn basic_session_agent_returns_create_run_for_import_requests() {
        let agent = BasicSessionAgent;

        let input = SessionAgentInput {
            session: Session {
                id: "session-1".to_string(),
                title: "Import Session".to_string(),
                status: SessionStatus::Active,
                current_intent: "idle".to_string(),
                current_object_type: "none".to_string(),
                current_object_id: "none".to_string(),
                summary: "Testing import routing".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![],
            user_message: "Please import these notes into Distilllab".to_string(),
        };

        let decision = agent
            .decide(input)
            .await
            .expect("basic session agent should decide");

        assert_eq!(decision.action_type, SessionActionType::CreateRun);
        assert_eq!(decision.intent, SessionIntent::ImportMaterial);
        assert_eq!(decision.tool_call_key, None);
        assert_eq!(
            decision.suggested_run_type,
            Some("import_and_distill".to_string())
        );
    }

    #[tokio::test]
    async fn basic_session_agent_treats_here_are_my_notes_as_import_material_intake() {
        let agent = BasicSessionAgent;

        let input = SessionAgentInput {
            session: Session {
                id: "session-1".to_string(),
                title: "Notes Intake Session".to_string(),
                status: SessionStatus::Active,
                current_intent: "idle".to_string(),
                current_object_type: "none".to_string(),
                current_object_id: "none".to_string(),
                summary: "Testing notes intake routing".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![],
            user_message: "Here are my notes, distill them".to_string(),
        };

        let decision = agent
            .decide(input)
            .await
            .expect("basic session agent should decide");

        assert_eq!(decision.intent, SessionIntent::ImportMaterial);
        assert_eq!(decision.action_type, SessionActionType::CreateRun);
        assert_eq!(decision.tool_call_key, None);
        assert_eq!(decision.suggested_run_type.as_deref(), Some("import_and_distill"));
    }

    #[tokio::test]
    async fn basic_session_agent_treats_file_based_request_as_import_material_intake() {
        let agent = BasicSessionAgent;

        let input = SessionAgentInput {
            session: Session {
                id: "session-1".to_string(),
                title: "File Intake Session".to_string(),
                status: SessionStatus::Active,
                current_intent: "idle".to_string(),
                current_object_type: "none".to_string(),
                current_object_id: "none".to_string(),
                summary: "Testing file intake routing".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![],
            user_message: "Use these files to create a distillation run".to_string(),
        };

        let decision = agent
            .decide(input)
            .await
            .expect("basic session agent should decide");

        assert_eq!(decision.intent, SessionIntent::ImportMaterial);
        assert_eq!(decision.action_type, SessionActionType::CreateRun);
        assert_eq!(decision.tool_call_key, None);
        assert_eq!(decision.suggested_run_type.as_deref(), Some("import_and_distill"));
    }

    #[tokio::test]
    async fn basic_session_agent_routes_deepening_requests_to_deepening_run() {
        let agent = BasicSessionAgent;

        let input = SessionAgentInput {
            session: Session {
                id: "session-1".to_string(),
                title: "Deepening Session".to_string(),
                status: SessionStatus::Active,
                current_intent: "idle".to_string(),
                current_object_type: "asset".to_string(),
                current_object_id: "asset-1".to_string(),
                summary: "Testing deepening routing".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![],
            user_message: "Please deepen this topic and ask follow-up questions".to_string(),
        };

        let decision = agent
            .decide(input)
            .await
            .expect("basic session agent should decide");

        assert_eq!(decision.action_type, SessionActionType::CreateRun);
        assert_eq!(decision.intent, SessionIntent::DeepenUnderstanding);
        assert_eq!(decision.primary_object_type.as_deref(), Some("asset"));
        assert_eq!(decision.primary_object_id.as_deref(), Some("asset-1"));
        assert_eq!(decision.suggested_run_type.as_deref(), Some("deepening"));
    }

    #[tokio::test]
    async fn basic_session_agent_routes_composition_requests_to_compose_and_verify_run() {
        let agent = BasicSessionAgent;

        let input = SessionAgentInput {
            session: Session {
                id: "session-1".to_string(),
                title: "Compose Session".to_string(),
                status: SessionStatus::Active,
                current_intent: "idle".to_string(),
                current_object_type: "project".to_string(),
                current_object_id: "project-1".to_string(),
                summary: "Testing composition routing".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![],
            user_message: "Write a summary article from these materials".to_string(),
        };

        let decision = agent
            .decide(input)
            .await
            .expect("basic session agent should decide");

        assert_eq!(decision.action_type, SessionActionType::CreateRun);
        assert_eq!(decision.intent, SessionIntent::ComposeOutput);
        assert_eq!(decision.primary_object_type.as_deref(), Some("project"));
        assert_eq!(decision.primary_object_id.as_deref(), Some("project-1"));
        assert_eq!(decision.suggested_run_type.as_deref(), Some("compose_and_verify"));
    }

    #[test]
    fn llm_session_agent_builds_minimal_system_and_user_messages() {
        let agent = LlmSessionAgent::new(LlmProviderConfig {
            provider_kind: "openai_compatible".to_string(),
            base_url: "http://localhost:11434/v1".to_string(),
            model: "qwen-test".to_string(),
            api_key: None,
        });

        let input = SessionAgentInput {
            session: Session {
                id: "session-1".to_string(),
                title: "LLM Session".to_string(),
                status: SessionStatus::Active,
                current_intent: "idle".to_string(),
                current_object_type: "none".to_string(),
                current_object_id: "none".to_string(),
                summary: "Testing llm session agent message assembly".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![],
            user_message: "Hello from the user".to_string(),
        };

        let messages = agent.build_chat_messages(&input);

        assert!(messages.len() >= 4);
        assert_eq!(messages[0].role, "system");
        assert!(messages[0].content.contains("Respond with valid JSON only"));
        assert!(messages[0].content.contains("action_type"));
        assert!(messages[0].content.contains("create_run"));
        assert_eq!(messages.last().expect("user message should exist").role, "user");
        assert_eq!(
            messages.last().expect("user message should exist").content,
            "Hello from the user"
        );
    }

    #[test]
    fn llm_session_agent_includes_recent_messages_before_current_user_message() {
        let agent = LlmSessionAgent::new(LlmProviderConfig {
            provider_kind: "openai_compatible".to_string(),
            base_url: "http://localhost:11434/v1".to_string(),
            model: "qwen-test".to_string(),
            api_key: None,
        });

        let input = SessionAgentInput {
            session: Session {
                id: "session-1".to_string(),
                title: "LLM Context Session".to_string(),
                status: SessionStatus::Active,
                current_intent: "idle".to_string(),
                current_object_type: "none".to_string(),
                current_object_id: "none".to_string(),
                summary: "Testing llm session agent context assembly".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![
                SessionMessage {
                    id: "message-1".to_string(),
                    session_id: "session-1".to_string(),
                    run_id: None,
                    message_type: "user_message".to_string(),
                    role: SessionMessageRole::User,
                    content: "Earlier user question".to_string(),
                    data_json: "{}".to_string(),
                    created_at: "2026-03-28T00:00:00Z".to_string(),
                },
                SessionMessage {
                    id: "message-2".to_string(),
                    session_id: "session-1".to_string(),
                    run_id: None,
                    message_type: "assistant_message".to_string(),
                    role: SessionMessageRole::Assistant,
                    content: "Earlier assistant reply".to_string(),
                    data_json: "{}".to_string(),
                    created_at: "2026-03-28T00:00:01Z".to_string(),
                },
            ],
            user_message: "Current user follow-up".to_string(),
        };

        let messages = agent.build_chat_messages(&input);

        assert!(messages.len() >= 6);
        let recent_user_index = messages
            .iter()
            .position(|message| message.content == "Earlier user question")
            .expect("recent user message should exist");
        let recent_assistant_index = messages
            .iter()
            .position(|message| message.content == "Earlier assistant reply")
            .expect("recent assistant message should exist");
        let current_user_index = messages
            .iter()
            .position(|message| message.content == "Current user follow-up")
            .expect("current user message should exist");

        assert_eq!(messages[recent_user_index].role, "user");
        assert_eq!(messages[recent_assistant_index].role, "assistant");
        assert!(recent_user_index < recent_assistant_index);
        assert!(recent_assistant_index < current_user_index);
    }

    #[test]
    fn llm_session_agent_includes_few_shot_examples_before_live_context() {
        let agent = LlmSessionAgent::new(LlmProviderConfig {
            provider_kind: "openai_compatible".to_string(),
            base_url: "http://localhost:11434/v1".to_string(),
            model: "qwen-test".to_string(),
            api_key: None,
        });

        let input = SessionAgentInput {
            session: Session {
                id: "session-1".to_string(),
                title: "Few Shot Session".to_string(),
                status: SessionStatus::Active,
                current_intent: "idle".to_string(),
                current_object_type: "none".to_string(),
                current_object_id: "none".to_string(),
                summary: "Testing few-shot message assembly".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![],
            user_message: "Please import these notes".to_string(),
        };

        let messages = agent.build_chat_messages(&input);

        assert!(messages.len() >= 4);
        assert_eq!(messages[1].role, "user");
        assert!(messages[1].content.contains("import these notes"));
        assert_eq!(messages[2].role, "assistant");
        assert!(messages[2].content.contains("\"action_type\":\"create_run\""));
        assert!(messages[2].content.contains("\"suggested_run_type\":\"import_and_distill\""));
    }

    #[test]
    fn llm_session_agent_few_shot_examples_cover_deepening_and_tool_call_paths() {
        let agent = LlmSessionAgent::new(LlmProviderConfig {
            provider_kind: "openai_compatible".to_string(),
            base_url: "http://localhost:11434/v1".to_string(),
            model: "qwen-test".to_string(),
            api_key: None,
        });

        let examples = agent.few_shot_examples();
        let joined = examples
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(joined.contains("\"intent\":\"deepen_understanding\""));
        assert!(joined.contains("\"action_type\":\"tool_call\""));
        assert!(joined.contains("\"tool_call_key\":"));
    }

    #[test]
    fn llm_session_agent_includes_current_session_object_clues_in_system_context() {
        let agent = LlmSessionAgent::new(LlmProviderConfig {
            provider_kind: "openai_compatible".to_string(),
            base_url: "http://localhost:11434/v1".to_string(),
            model: "qwen-test".to_string(),
            api_key: None,
        });

        let input = SessionAgentInput {
            session: Session {
                id: "session-1".to_string(),
                title: "Source Review Session".to_string(),
                status: SessionStatus::Active,
                current_intent: "review_source".to_string(),
                current_object_type: "source".to_string(),
                current_object_id: "source-1".to_string(),
                summary: "The session is focused on one imported source".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![],
            user_message: "Summarize the current source".to_string(),
        };

        let messages = agent.build_chat_messages(&input);

        assert!(messages[0].content.contains("current_object_type: source"));
        assert!(messages[0].content.contains("current_object_id: source-1"));
        assert!(messages[0].content.contains("session_summary: The session is focused on one imported source"));
    }

    #[tokio::test]
    async fn llm_session_agent_returns_direct_reply_from_llm_response() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let address = listener
            .local_addr()
            .expect("listener should have local addr");

        tokio::spawn(async move {
            let (mut stream, _) = listener
                .accept()
                .await
                .expect("server should accept connection");
            let mut buffer = [0_u8; 4096];
            let _ = stream
                .read(&mut buffer)
                .await
                .expect("server should read request");

            let response_body = r#"{
                "choices": [
                    {
                        "message": {
                            "role": "assistant",
                            "content": "Hello from fake llm"
                        }
                    }
                ]
            }"#;

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );

            stream
                .write_all(response.as_bytes())
                .await
                .expect("server should write response");
        });

        let agent = LlmSessionAgent::new(LlmProviderConfig {
            provider_kind: "openai_compatible".to_string(),
            base_url: format!("http://{}", address),
            model: "gpt-test".to_string(),
            api_key: None,
        });

        let input = SessionAgentInput {
            session: Session {
                id: "session-1".to_string(),
                title: "LLM Reply Session".to_string(),
                status: SessionStatus::Active,
                current_intent: "idle".to_string(),
                current_object_type: "none".to_string(),
                current_object_id: "none".to_string(),
                summary: "Testing llm session agent decision".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![],
            user_message: "Say hello".to_string(),
        };

        let decision = agent.decide(input).await.expect("llm session agent should decide");

        assert_eq!(decision.action_type, SessionActionType::DirectReply);
        assert_eq!(decision.intent, SessionIntent::GeneralReply);
        assert_eq!(decision.reply_text, "Hello from fake llm");
    }

    #[tokio::test]
    async fn llm_session_agent_parses_structured_create_run_decision_from_json_response() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let address = listener
            .local_addr()
            .expect("listener should have local addr");

        tokio::spawn(async move {
            let (mut stream, _) = listener
                .accept()
                .await
                .expect("server should accept connection");
            let mut buffer = [0_u8; 4096];
            let _ = stream
                .read(&mut buffer)
                .await
                .expect("server should read request");

            let response_body = r#"{
                "choices": [
                    {
                        "message": {
                            "role": "assistant",
                            "content": "{\"intent\":\"import_material\",\"action_type\":\"create_run\",\"reply_text\":\"I will start an import and distill run.\",\"suggested_run_type\":\"import_and_distill\",\"session_summary\":\"Preparing to import material\"}"
                        }
                    }
                ]
            }"#;

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );

            stream
                .write_all(response.as_bytes())
                .await
                .expect("server should write response");
        });

        let agent = LlmSessionAgent::new(LlmProviderConfig {
            provider_kind: "openai_compatible".to_string(),
            base_url: format!("http://{}", address),
            model: "gpt-test".to_string(),
            api_key: None,
        });

        let input = SessionAgentInput {
            session: Session {
                id: "session-1".to_string(),
                title: "Structured LLM Reply Session".to_string(),
                status: SessionStatus::Active,
                current_intent: "idle".to_string(),
                current_object_type: "none".to_string(),
                current_object_id: "none".to_string(),
                summary: "Testing llm structured decision parsing".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![],
            user_message: "Import these notes".to_string(),
        };

        let decision = agent.decide(input).await.expect("llm session agent should decide");

        assert_eq!(decision.action_type, SessionActionType::CreateRun);
        assert_eq!(decision.intent, SessionIntent::ImportMaterial);
        assert_eq!(decision.suggested_run_type.as_deref(), Some("import_and_distill"));
        assert_eq!(decision.reply_text, "I will start an import and distill run.");
    }

    #[tokio::test]
    async fn llm_session_agent_debug_result_preserves_raw_output_and_parsed_decision() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let address = listener
            .local_addr()
            .expect("listener should have local addr");

        tokio::spawn(async move {
            let (mut stream, _) = listener
                .accept()
                .await
                .expect("server should accept connection");
            let mut buffer = [0_u8; 4096];
            let _ = stream
                .read(&mut buffer)
                .await
                .expect("server should read request");

            let raw_json = "{\"intent\":\"general_reply\",\"action_type\":\"direct_reply\",\"reply_text\":\"Here is the answer.\",\"primary_object_type\":null,\"primary_object_id\":null,\"suggested_run_type\":null,\"session_summary\":\"Providing a direct answer\"}";
            let encoded_raw_json = serde_json::to_string(raw_json).expect("raw json should encode");
            let response_body = format!(
                "{{\"choices\":[{{\"message\":{{\"role\":\"assistant\",\"content\":{encoded_raw_json}}}}}]}}"
            );

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );

            stream
                .write_all(response.as_bytes())
                .await
                .expect("server should write response");
        });

        let agent = LlmSessionAgent::new(LlmProviderConfig {
            provider_kind: "openai_compatible".to_string(),
            base_url: format!("http://{}", address),
            model: "gpt-test".to_string(),
            api_key: None,
        });

        let input = SessionAgentInput {
            session: Session {
                id: "session-1".to_string(),
                title: "Debug Session".to_string(),
                status: SessionStatus::Active,
                current_intent: "idle".to_string(),
                current_object_type: "none".to_string(),
                current_object_id: "none".to_string(),
                summary: "Testing llm debug result".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![],
            user_message: "Hello".to_string(),
        };

        let result = agent
            .decide_with_debug(input)
            .await
            .expect("llm session agent debug should decide");

        assert!(result.raw_output.contains("\"intent\":\"general_reply\""));
        assert_eq!(result.decision.intent, SessionIntent::GeneralReply);
        assert_eq!(result.decision.reply_text, "Here is the answer.");
    }

    #[test]
    fn session_action_type_supports_tool_call_variant() {
        assert_eq!(SessionActionType::ToolCall.as_str(), "tool_call");
    }

    #[test]
    fn session_agent_definition_system_prompt_requires_fixed_json_contract() {
        let definition = session_agent_definition();

        assert!(definition.system_prompt.contains("Respond with valid JSON only"));
        assert!(definition.system_prompt.contains("intent"));
        assert!(definition.system_prompt.contains("Intent must be one of"));
        assert!(definition.system_prompt.contains("general_reply"));
        assert!(definition.system_prompt.contains("import_material"));
        assert!(definition.system_prompt.contains("deepen_understanding"));
        assert!(definition.system_prompt.contains("compose_output"));
        assert!(definition.system_prompt.contains("action_type"));
        assert!(definition.system_prompt.contains("reply_text"));
        assert!(definition.system_prompt.contains("suggested_run_type"));
        assert!(definition.system_prompt.contains("direct_reply"));
        assert!(definition.system_prompt.contains("request_clarification"));
        assert!(definition.system_prompt.contains("create_run"));
    }
}
