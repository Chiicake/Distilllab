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
        {
            return Ok(SessionAgentDecision {
                intent: "import_material".to_string(),
                primary_object_type,
                primary_object_id,
                action_type: SessionActionType::CreateRun,
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
                intent: "deepen_understanding".to_string(),
                primary_object_type,
                primary_object_id,
                action_type: SessionActionType::CreateRun,
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
                intent: "compose_output".to_string(),
                primary_object_type,
                primary_object_id,
                action_type: SessionActionType::CreateRun,
                reply_text: "I will prepare a compose and verify run for this output request.".to_string(),
                suggested_run_type: Some("compose_and_verify".to_string()),
                session_summary: Some("Preparing to compose an output".to_string()),
            });
        }

        Ok(SessionAgentDecision {
            intent: "general_reply".to_string(),
            primary_object_type,
            primary_object_id,
            action_type: SessionActionType::DirectReply,
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

#[derive(Debug, Deserialize)]
struct StructuredSessionAgentDecision {
    intent: String,
    action_type: String,
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
        let mut messages = vec![OpenAiCompatibleChatMessage {
            role: "system".to_string(),
            content: session_agent_definition().system_prompt,
        }];

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
            intent: parsed.intent,
            primary_object_type: parsed.primary_object_type,
            primary_object_id: parsed.primary_object_id,
            action_type,
            reply_text: parsed.reply_text,
            suggested_run_type: parsed.suggested_run_type,
            session_summary: parsed.session_summary,
        })
    }
}

#[async_trait]
impl SessionAgent for LlmSessionAgent {
    async fn decide(&self, input: SessionAgentInput) -> Result<SessionAgentDecision, AgentError> {
        let messages = self.build_chat_messages(&input);

        let request = OpenAiCompatibleChatRequest {
            model: self.config.model.clone(),
            messages,
        };

        let response = send_chat_completion_request(&self.client, &self.config, &request).await?;

        let reply_text = response.first_message_content().ok_or_else(|| {
            AgentError::Response("llm response did not contain assistant content".to_string())
        })?;

        if let Some(structured_decision) = Self::parse_structured_decision(reply_text) {
            return Ok(structured_decision);
        }

        Ok(SessionAgentDecision {
            intent: "llm_direct_reply".to_string(),
            primary_object_type: None,
            primary_object_id: None,
            action_type: SessionActionType::DirectReply,
            reply_text: reply_text.to_string(),
            suggested_run_type: None,
            session_summary: Some("LLM replied to the current session message".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BasicSessionAgent, LlmSessionAgent, SessionActionType, SessionAgent,
        SessionAgentDecision, SessionAgentInput, session_agent_definition,
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
        assert_eq!(decision.intent, "general_reply");
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
        assert_eq!(decision.intent, "import_material");
        assert_eq!(
            decision.suggested_run_type,
            Some("import_and_distill".to_string())
        );
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
        assert_eq!(decision.intent, "deepen_understanding");
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
        assert_eq!(decision.intent, "compose_output");
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

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
        assert_eq!(messages[1].content, "Hello from the user");
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

        assert_eq!(messages.len(), 4);
        assert_eq!(messages[1].role, "user");
        assert_eq!(messages[1].content, "Earlier user question");
        assert_eq!(messages[2].role, "assistant");
        assert_eq!(messages[2].content, "Earlier assistant reply");
        assert_eq!(messages[3].role, "user");
        assert_eq!(messages[3].content, "Current user follow-up");
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
        assert_eq!(decision.intent, "llm_direct_reply");
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
        assert_eq!(decision.intent, "import_material");
        assert_eq!(decision.suggested_run_type.as_deref(), Some("import_and_distill"));
        assert_eq!(decision.reply_text, "I will start an import and distill run.");
    }
}
