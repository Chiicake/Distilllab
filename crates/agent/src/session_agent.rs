use crate::{
    send_chat_completion_request, stream_chat_completion_request, AgentDefinition, AgentError,
    LlmProviderConfig, OpenAiCompatibleChatMessage, OpenAiCompatibleChatRequest,
    SkillSelection, ToolInvocation,
};
use async_trait::async_trait;
use serde::Deserialize;
use schema::{Session, SessionIntake, SessionMessage, SessionMessageRole};

pub fn session_agent_definition() -> AgentDefinition {
    AgentDefinition {
        id: "agent-session".to_string(),
        key: "session_agent".to_string(),
        name: "Session Agent".to_string(),
        kind: "session".to_string(),
        description: "Distilllab session-level planning agent that understands user intent and decides the next action for the current session.".to_string(),
        responsibility_summary: "Reads the current session and recent timeline, identifies user intent and primary object, decides the next action, and can choose what to do after actions succeed or fail. It does not execute the workflow itself.".to_string(),
        status: "active".to_string(),
        system_prompt: "You are the Session Agent for Distilllab. Distilllab's primary goal is to distill work content and working process materials into reusable knowledge objects, not to act as a generic note organizer. You are the session-level planner for the current session: understand the current session state, identify user intent, decide the next high-level action, and consider post-action follow-up and failure handling at the session level. Respond with valid JSON only. Do not include markdown fences or extra explanation. The JSON object must contain these fields: intent, action_type, reply_text, primary_object_type, primary_object_id, suggested_run_type, session_summary, tool_invocation, skill_selection, should_continue_planning, failure_hint. Intent must be one of: general_reply, distill_material, deepen_understanding, compose_output. action_type must be one of: direct_reply, request_clarification, tool_call, skill_call, create_run, stop. If intent is distill_material, action_type must be create_run or request_clarification, and create_run should normally use suggested_run_type import_and_distill. Use tool_invocation only when action_type is tool_call. tool_invocation must contain tool_name, arguments, reasoning_summary, and expected_follow_up, where arguments is a JSON object. Prefer list_attachments when you need to discover which attachments are available. Prefer read_attachment_excerpt or read_text when the user asks what a current attachment contains. For read_attachment_excerpt and read_text, prefer arguments using attachment_index when current_intake_attachments are present; use attachment_id only when needed. Prefer web_fetch when the user asks about a URL or webpage and the answer requires reading remote content. If one tool result is insufficient to satisfy the request, you may return another tool_call on the next planning turn instead of answering prematurely. Use skill_selection only when action_type is skill_call. skill_selection must contain skill_key, reasoning_summary, and expected_outcome. Use suggested_run_type only when action_type is create_run. Set should_continue_planning to true when the session should expect a follow-up planning turn after the chosen action finishes, otherwise false. Use failure_hint to summarize what the planner should consider if the chosen action fails. Use null for optional fields when unknown.".to_string(),
        default_model_profile: "reasoning_default".to_string(),
        allowed_tool_keys: vec![
            "list_sources".to_string(),
            "list_projects".to_string(),
            "list_runs".to_string(),
            "get_session".to_string(),
            "get_project".to_string(),
            "get_asset".to_string(),
            "search_memory".to_string(),
            "read_attachment_excerpt".to_string(),
            "read_text".to_string(),
            "list_attachments".to_string(),
            "web_fetch".to_string(),
        ],
        allowed_skill_keys: vec![
            "import_and_distill_skill".to_string(),
            "deepen_asset_skill".to_string(),
            "compose_and_verify_skill".to_string(),
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
    SkillCall,
    CreateRun,
    Stop,
}

impl SessionActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionActionType::DirectReply => "direct_reply",
            SessionActionType::RequestClarification => "request_clarification",
            SessionActionType::ToolCall => "tool_call",
            SessionActionType::SkillCall => "skill_call",
            SessionActionType::CreateRun => "create_run",
            SessionActionType::Stop => "stop",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCreationRequest {
    pub run_type: String,
    pub reasoning_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionNextAction {
    DirectReply,
    RequestClarification,
    ToolCall(ToolInvocation),
    SkillCall(SkillSelection),
    CreateRun(RunCreationRequest),
    Stop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionIntent {
    GeneralReply,
    DistillMaterial,
    DeepenUnderstanding,
    ComposeOutput,
}

impl SessionIntent {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionIntent::GeneralReply => "general_reply",
            SessionIntent::DistillMaterial => "distill_material",
            SessionIntent::DeepenUnderstanding => "deepen_understanding",
            SessionIntent::ComposeOutput => "compose_output",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "general_reply" | "llm_direct_reply" => Some(SessionIntent::GeneralReply),
            "distill_material" => Some(SessionIntent::DistillMaterial),
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
    pub intake: SessionIntake,
}

#[derive(Debug, Clone)]
pub struct SessionAgentDecision {
    pub intent: SessionIntent,
    pub primary_object_type: Option<String>,
    pub primary_object_id: Option<String>,
    pub action_type: SessionActionType,
    pub next_action: SessionNextAction,
    pub tool_invocation: Option<ToolInvocation>,
    pub skill_selection: Option<SkillSelection>,
    pub run_creation: Option<RunCreationRequest>,
    pub reply_text: String,
    pub suggested_run_type: Option<String>,
    pub session_summary: Option<String>,
    pub should_continue_planning: bool,
    pub failure_hint: Option<String>,
}

#[async_trait]
pub trait SessionAgent {
    async fn decide(&self, input: SessionAgentInput) -> Result<SessionAgentDecision, AgentError>;
}

pub struct BasicSessionAgent;

#[async_trait]
impl SessionAgent for BasicSessionAgent {
    async fn decide(&self, input: SessionAgentInput) -> Result<SessionAgentDecision, AgentError> {
        let normalized_message = input.intake.user_message.to_lowercase();
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
            || (normalized_message.contains("distill") && normalized_message.contains("work notes"))
            || (normalized_message.contains("distill") && extract_first_url(&input.intake.user_message).is_some())
            || (!input.intake.attachments.is_empty()
                && (normalized_message.contains("distill")
                    || normalized_message.contains("提炼")
                    || normalized_message.contains("提取")))
        {
            return Ok(SessionAgentDecision {
                intent: SessionIntent::DistillMaterial,
                primary_object_type: primary_object_type.or(Some("material".to_string())),
                primary_object_id,
                action_type: SessionActionType::CreateRun,
                next_action: SessionNextAction::CreateRun(RunCreationRequest {
                    run_type: "import_and_distill".to_string(),
                    reasoning_summary: Some("Distill material should enter the import_and_distill workflow.".to_string()),
                }),
                tool_invocation: None,
                skill_selection: None,
                run_creation: Some(RunCreationRequest {
                    run_type: "import_and_distill".to_string(),
                    reasoning_summary: Some("Distill material should enter the import_and_distill workflow.".to_string()),
                }),
                reply_text: "I will start a distill run for this work material.".to_string(),
                suggested_run_type: Some("import_and_distill".to_string()),
                session_summary: Some("Preparing to distill work material".to_string()),
                should_continue_planning: true,
                failure_hint: Some("clarify_or_stop".to_string()),
            });
        }

        if !input.intake.attachments.is_empty()
            && (normalized_message.contains("attachment")
                || normalized_message.contains("attachments")
                || normalized_message.contains("附件"))
            && (normalized_message.contains("available")
                || normalized_message.contains("which")
                || normalized_message.contains("list")
                || normalized_message.contains("有哪些")
                || normalized_message.contains("哪些"))
        {
            let tool_invocation = ToolInvocation::new("list_attachments")
                .with_reasoning("Need to inspect the current attachment list before answering.")
                .with_expected_follow_up("Summarize which attachments are available.");

            return Ok(SessionAgentDecision {
                intent: SessionIntent::GeneralReply,
                primary_object_type,
                primary_object_id,
                action_type: SessionActionType::ToolCall,
                next_action: SessionNextAction::ToolCall(tool_invocation.clone()),
                tool_invocation: Some(tool_invocation),
                skill_selection: None,
                run_creation: None,
                reply_text: "I will inspect the current attachments before answering.".to_string(),
                suggested_run_type: None,
                session_summary: Some("Preparing an attachment lookup before answering".to_string()),
                should_continue_planning: true,
                failure_hint: Some("reply_or_clarify".to_string()),
            });
        }

        if !input.intake.attachments.is_empty()
            && (normalized_message.contains("read")
                || normalized_message.contains("open")
                || normalized_message.contains("text")
                || normalized_message.contains("内容")
                || normalized_message.contains("读取"))
            && (normalized_message.contains("attachment")
                || normalized_message.contains("file")
                || normalized_message.contains("附件"))
        {
            let tool_invocation = ToolInvocation::with_value_args(
                "read_text",
                serde_json::json!({
                    "attachment_index": 0,
                    "max_chars": 1200,
                }),
            )
            .with_reasoning("Need to read the current attachment text before answering.")
            .with_expected_follow_up("Use the text content to answer directly.");

            return Ok(SessionAgentDecision {
                intent: SessionIntent::GeneralReply,
                primary_object_type,
                primary_object_id,
                action_type: SessionActionType::ToolCall,
                next_action: SessionNextAction::ToolCall(tool_invocation.clone()),
                tool_invocation: Some(tool_invocation),
                skill_selection: None,
                run_creation: None,
                reply_text: "I will read the current attachment before answering.".to_string(),
                suggested_run_type: None,
                session_summary: Some("Preparing to read the current attachment before answering".to_string()),
                should_continue_planning: true,
                failure_hint: Some("reply_or_clarify".to_string()),
            });
        }

        if !input.intake.attachments.is_empty()
            && (normalized_message.contains("attachment")
                || normalized_message.contains("file")
                || normalized_message.contains("路径")
                || normalized_message.contains("附件"))
            && (normalized_message.contains("inside")
                || normalized_message.contains("content")
                || normalized_message.contains("contains")
                || normalized_message.contains("what is")
                || normalized_message.contains("内容")
                || normalized_message.contains("里面")
                || normalized_message.contains("是什么"))
        {
            let first_attachment = input.intake.attachments.first().expect("checked non-empty");
            let tool_invocation = ToolInvocation::with_args(
                "read_attachment_excerpt",
                &serde_json::json!({
                    "attachment_index": 0,
                    "attachment_id": first_attachment.attachment_id,
                    "max_chars": 400,
                })
                .to_string(),
            )
            .with_reasoning("The user is asking about the contents of the current attachment.")
            .with_expected_follow_up("Use the excerpt to answer what the attachment contains.");

            return Ok(SessionAgentDecision {
                intent: SessionIntent::GeneralReply,
                primary_object_type,
                primary_object_id,
                action_type: SessionActionType::ToolCall,
                next_action: SessionNextAction::ToolCall(tool_invocation.clone()),
                tool_invocation: Some(tool_invocation),
                skill_selection: None,
                run_creation: None,
                reply_text: "I will inspect the current attachment before answering.".to_string(),
                suggested_run_type: None,
                session_summary: Some(
                    "Preparing to inspect the current attachment before answering"
                        .to_string(),
                ),
                should_continue_planning: true,
                failure_hint: Some("reply_or_clarify".to_string()),
            });
        }

        if let Some(url) = extract_first_url(&input.intake.user_message) {
            let tool_invocation = ToolInvocation::with_value_args(
                "web_fetch",
                serde_json::json!({
                    "url": url,
                    "max_chars": 4000,
                }),
            )
            .with_reasoning("Need to inspect the referenced webpage before answering.")
            .with_expected_follow_up("Use the fetched page content to answer the user's question.");

            return Ok(SessionAgentDecision {
                intent: SessionIntent::GeneralReply,
                primary_object_type,
                primary_object_id,
                action_type: SessionActionType::ToolCall,
                next_action: SessionNextAction::ToolCall(tool_invocation.clone()),
                tool_invocation: Some(tool_invocation),
                skill_selection: None,
                run_creation: None,
                reply_text: "I will fetch the webpage before answering.".to_string(),
                suggested_run_type: None,
                session_summary: Some("Preparing a web fetch before answering".to_string()),
                should_continue_planning: true,
                failure_hint: Some("reply_or_clarify".to_string()),
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
                next_action: SessionNextAction::CreateRun(RunCreationRequest {
                    run_type: "deepening".to_string(),
                    reasoning_summary: Some("This request needs a deepening workflow.".to_string()),
                }),
                tool_invocation: None,
                skill_selection: None,
                run_creation: Some(RunCreationRequest {
                    run_type: "deepening".to_string(),
                    reasoning_summary: Some("This request needs a deepening workflow.".to_string()),
                }),
                reply_text: "I will start a deepening run to explore this topic further.".to_string(),
                suggested_run_type: Some("deepening".to_string()),
                session_summary: Some("Preparing to deepen understanding".to_string()),
                should_continue_planning: true,
                failure_hint: Some("clarify_or_stop".to_string()),
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
                next_action: SessionNextAction::CreateRun(RunCreationRequest {
                    run_type: "compose_and_verify".to_string(),
                    reasoning_summary: Some(
                        "This output request should enter the compose_and_verify workflow."
                            .to_string(),
                    ),
                }),
                tool_invocation: None,
                skill_selection: None,
                run_creation: Some(RunCreationRequest {
                    run_type: "compose_and_verify".to_string(),
                    reasoning_summary: Some(
                        "This output request should enter the compose_and_verify workflow."
                            .to_string(),
                    ),
                }),
                reply_text: "I will prepare a compose and verify run for this output request.".to_string(),
                suggested_run_type: Some("compose_and_verify".to_string()),
                session_summary: Some("Preparing to compose an output".to_string()),
                should_continue_planning: true,
                failure_hint: Some("clarify_or_stop".to_string()),
            });
        }

        Ok(SessionAgentDecision {
            intent: SessionIntent::GeneralReply,
            primary_object_type,
            primary_object_id,
            action_type: SessionActionType::DirectReply,
            next_action: SessionNextAction::DirectReply,
            tool_invocation: None,
            skill_selection: None,
            run_creation: None,
            reply_text: "Hello! I am ready to help with your Distilllab session.".to_string(),
            suggested_run_type: None,
            session_summary: Some("General session assistance".to_string()),
            should_continue_planning: false,
            failure_hint: None,
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
    tool_invocation: Option<ToolInvocation>,
    skill_selection: Option<SkillSelection>,
    reply_text: String,
    primary_object_type: Option<String>,
    primary_object_id: Option<String>,
    suggested_run_type: Option<String>,
    session_summary: Option<String>,
    should_continue_planning: Option<bool>,
    failure_hint: Option<String>,
}

impl LlmSessionAgent {
    pub fn new(config: LlmProviderConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    fn format_attachment_context(input: &SessionAgentInput) -> String {
        if input.intake.attachments.is_empty() {
            "current_intake_attachments: none".to_string()
        } else {
            let attachment_lines = input
                .intake
                .attachments
                .iter()
                .map(|attachment| {
                    format!(
                        "- attachment_id: {} | name: {} | kind: {} | mime_type: {} | size: {}",
                        attachment.attachment_id,
                        attachment.name,
                        attachment.kind,
                        attachment.mime_type,
                        attachment.size,
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            format!("current_intake_attachments:\n{}", attachment_lines)
        }
    }

    fn build_chat_messages(&self, input: &SessionAgentInput) -> Vec<OpenAiCompatibleChatMessage> {
        let attachment_context = Self::format_attachment_context(input);

        let system_context = format!(
            "{}\nCurrent session context:\nsession_id: {}\nsession_title: {}\ncurrent_intent: {}\ncurrent_object_type: {}\ncurrent_object_id: {}\nsession_summary: {}\n{}",
            session_agent_definition().system_prompt,
            input.session.id,
            input.session.title,
            input.session.current_intent,
            input.session.current_object_type,
            input.session.current_object_id,
            input.session.summary,
            attachment_context,
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

        let current_turn_already_present = input.recent_messages.iter().rev().any(|message| {
            message.role == SessionMessageRole::User && message.content == input.intake.user_message
        });

        if !current_turn_already_present {
            messages.push(OpenAiCompatibleChatMessage {
                role: "user".to_string(),
                content: input.intake.user_message.clone(),
            });
        }

        messages
    }

    fn build_direct_reply_stream_messages(
        &self,
        input: &SessionAgentInput,
        decision: &SessionAgentDecision,
    ) -> Vec<OpenAiCompatibleChatMessage> {
        let attachment_context = Self::format_attachment_context(input);
        let system_context = format!(
            "You are the assistant for Distilllab. You are writing the final user-facing reply for an already-selected direct_reply action. Respond with plain text only. Do not output JSON, metadata fields, or markdown fences.\nCurrent session context:\nsession_id: {}\nsession_title: {}\ncurrent_intent: {}\ncurrent_object_type: {}\ncurrent_object_id: {}\nsession_summary: {}\n{}\nPlanner decision:\nintent: {}\naction_type: {}\nreply_text_draft: {}",
            input.session.id,
            input.session.title,
            input.session.current_intent,
            input.session.current_object_type,
            input.session.current_object_id,
            input.session.summary,
            attachment_context,
            decision.intent.as_str(),
            decision.action_type.as_str(),
            decision.reply_text,
        );

        let mut messages = vec![OpenAiCompatibleChatMessage {
            role: "system".to_string(),
            content: system_context,
        }];

        for recent_message in &input.recent_messages {
            let role = match recent_message.role {
                SessionMessageRole::User => "user",
                SessionMessageRole::Assistant => "assistant",
                SessionMessageRole::System => continue,
            };

            messages.push(OpenAiCompatibleChatMessage {
                role: role.to_string(),
                content: recent_message.content.clone(),
            });
        }

        let current_turn_already_present = input.recent_messages.iter().rev().any(|message| {
            message.role == SessionMessageRole::User && message.content == input.intake.user_message
        });

        if !current_turn_already_present {
            messages.push(OpenAiCompatibleChatMessage {
                role: "user".to_string(),
                content: input.intake.user_message.clone(),
            });
        }

        messages
    }


    fn few_shot_examples(&self) -> Vec<OpenAiCompatibleChatMessage> {
        vec![
            OpenAiCompatibleChatMessage {
                role: "user".to_string(),
                content: "Please distill these work notes into Distilllab".to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "assistant".to_string(),
                content: r#"{"intent":"distill_material","action_type":"create_run","reply_text":"I will start a distill run for this work material.","primary_object_type":null,"primary_object_id":null,"suggested_run_type":"import_and_distill","session_summary":"Preparing to distill work material","tool_invocation":null,"should_continue_planning":true,"failure_hint":"clarify_or_stop"}"#.to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "user".to_string(),
                content: "Write a concise article from this project".to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "assistant".to_string(),
                content: r#"{"intent":"compose_output","action_type":"create_run","reply_text":"I will prepare a compose and verify run for this output request.","primary_object_type":"project","primary_object_id":"project-current","suggested_run_type":"compose_and_verify","session_summary":"Preparing to compose an output","tool_invocation":null,"skill_selection":null,"should_continue_planning":true,"failure_hint":"clarify_or_stop"}"#.to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "user".to_string(),
                content: "Please deepen this topic and ask follow-up questions".to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "assistant".to_string(),
                content: r#"{"intent":"deepen_understanding","action_type":"create_run","reply_text":"I will start a deepening run to explore this topic further.","primary_object_type":"asset","primary_object_id":"asset-current","suggested_run_type":"deepening","session_summary":"Preparing to deepen understanding","tool_invocation":null,"should_continue_planning":true,"failure_hint":"clarify_or_stop"}"#.to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "user".to_string(),
                content: "Search memory for related notes before answering".to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "assistant".to_string(),
                content: r#"{"intent":"general_reply","action_type":"tool_call","reply_text":"I will look up related notes before replying.","primary_object_type":null,"primary_object_id":null,"suggested_run_type":null,"session_summary":"Preparing a memory lookup before answering","tool_invocation":{"tool_name":"search_memory","arguments":{},"reasoning_summary":null,"expected_follow_up":null},"should_continue_planning":true,"failure_hint":"reply_or_clarify"}"#.to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "user".to_string(),
                content: "Show me which attachments are available in this turn".to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "assistant".to_string(),
                content: r#"{"intent":"general_reply","action_type":"tool_call","reply_text":"I will inspect the current attachments before replying.","primary_object_type":null,"primary_object_id":null,"suggested_run_type":null,"session_summary":"Preparing an attachment lookup before answering","tool_invocation":{"tool_name":"list_attachments","arguments":{},"reasoning_summary":null,"expected_follow_up":null},"should_continue_planning":true,"failure_hint":"reply_or_clarify"}"#.to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "user".to_string(),
                content: "Read the current attachment before answering".to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "assistant".to_string(),
                content: r#"{"intent":"general_reply","action_type":"tool_call","reply_text":"I will read the current attachment before replying.","primary_object_type":null,"primary_object_id":null,"suggested_run_type":null,"session_summary":"Preparing to read the current attachment before answering","tool_invocation":{"tool_name":"read_text","arguments":{"attachment_index":0,"max_chars":400},"reasoning_summary":null,"expected_follow_up":null},"should_continue_planning":true,"failure_hint":"reply_or_clarify"}"#.to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "user".to_string(),
                content: "Please check what this URL says before answering".to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "assistant".to_string(),
                content: r#"{"intent":"general_reply","action_type":"tool_call","reply_text":"I will fetch the webpage before replying.","primary_object_type":null,"primary_object_id":null,"suggested_run_type":null,"session_summary":"Preparing a web fetch before answering","tool_invocation":{"tool_name":"web_fetch","arguments":{"url":"https://example.com"},"reasoning_summary":null,"expected_follow_up":null},"should_continue_planning":true,"failure_hint":"reply_or_clarify"}"#.to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "user".to_string(),
                content: "What did we do so far?".to_string(),
            },
            OpenAiCompatibleChatMessage {
                role: "assistant".to_string(),
                content: r#"{"intent":"general_reply","action_type":"direct_reply","reply_text":"Here is a concise summary of the current session.","primary_object_type":null,"primary_object_id":null,"suggested_run_type":null,"session_summary":"Providing a direct session summary","tool_invocation":null,"should_continue_planning":false,"failure_hint":null}"#.to_string(),
            },
        ]
    }

    fn parse_action_type(action_type: &str) -> Option<SessionActionType> {
        match action_type {
            "direct_reply" => Some(SessionActionType::DirectReply),
            "request_clarification" => Some(SessionActionType::RequestClarification),
            "tool_call" => Some(SessionActionType::ToolCall),
            "skill_call" => Some(SessionActionType::SkillCall),
            "create_run" => Some(SessionActionType::CreateRun),
            "stop" => Some(SessionActionType::Stop),
            _ => None,
        }
    }

    fn parse_structured_decision(reply_text: &str) -> Option<SessionAgentDecision> {
        let parsed = serde_json::from_str::<StructuredSessionAgentDecision>(reply_text).ok()?;
        let action_type = Self::parse_action_type(parsed.action_type.as_str())?;
        let tool_invocation = parsed.tool_invocation.or_else(|| {
            parsed
                .tool_call_key
                .map(|tool_name| ToolInvocation::new(&tool_name))
        });
        let skill_selection = parsed.skill_selection;
        let run_creation = parsed.suggested_run_type.as_ref().map(|run_type| RunCreationRequest {
            run_type: run_type.clone(),
            reasoning_summary: None,
        });
        let next_action = match action_type {
            SessionActionType::DirectReply => SessionNextAction::DirectReply,
            SessionActionType::RequestClarification => SessionNextAction::RequestClarification,
            SessionActionType::ToolCall => {
                SessionNextAction::ToolCall(tool_invocation.clone()?)
            }
            SessionActionType::SkillCall => {
                SessionNextAction::SkillCall(skill_selection.clone()?)
            }
            SessionActionType::CreateRun => {
                SessionNextAction::CreateRun(run_creation.clone()?)
            }
            SessionActionType::Stop => SessionNextAction::Stop,
        };

        let mut decision = SessionAgentDecision {
            intent: SessionIntent::from_str(parsed.intent.as_str())?,
            primary_object_type: parsed.primary_object_type,
            primary_object_id: parsed.primary_object_id,
            action_type: action_type.clone(),
            next_action,
            tool_invocation,
            skill_selection,
            run_creation,
            reply_text: parsed.reply_text,
            suggested_run_type: parsed.suggested_run_type,
            session_summary: parsed.session_summary,
            should_continue_planning: parsed.should_continue_planning.unwrap_or(matches!(
                action_type,
                SessionActionType::ToolCall | SessionActionType::CreateRun | SessionActionType::RequestClarification
            )),
            failure_hint: parsed.failure_hint,
        };

        match decision.action_type {
            SessionActionType::ToolCall => {
                if decision.tool_invocation.is_none() {
                    return None;
                }
            }
            SessionActionType::SkillCall => {
                if decision.skill_selection.is_none() {
                    return None;
                }
                if decision.tool_invocation.is_some() || decision.run_creation.is_some() {
                    return None;
                }
            }
            SessionActionType::CreateRun => {
                if decision.run_creation.is_none() {
                    return None;
                }
                if decision.tool_invocation.is_some() || decision.skill_selection.is_some() {
                    return None;
                }
            }
            _ => {
                if decision.tool_invocation.is_some() {
                    return None;
                }
                if decision.skill_selection.is_some() || decision.run_creation.is_some() {
                    return None;
                }
            }
        }

        if decision.intent == SessionIntent::DistillMaterial
            && decision.action_type == SessionActionType::DirectReply
        {
            decision.action_type = SessionActionType::CreateRun;
            decision.next_action = SessionNextAction::CreateRun(RunCreationRequest {
                run_type: "import_and_distill".to_string(),
                reasoning_summary: Some("Distill material should enter the import_and_distill workflow.".to_string()),
            });
            decision.suggested_run_type = Some("import_and_distill".to_string());
            decision.tool_invocation = None;
            decision.skill_selection = None;
            decision.run_creation = Some(RunCreationRequest {
                run_type: "import_and_distill".to_string(),
                reasoning_summary: Some("Distill material should enter the import_and_distill workflow.".to_string()),
            });
            decision.should_continue_planning = true;
            decision.failure_hint = Some("clarify_or_stop".to_string());
        }

        if decision.action_type == SessionActionType::CreateRun {
            let valid_run_type = matches!(
                decision.suggested_run_type.as_deref(),
                Some("import_and_distill") | Some("deepening") | Some("compose_and_verify")
            );

            if !valid_run_type {
                return None;
            }
        }

        Some(decision)
    }

    fn fallback_direct_reply_decision(reply_text: &str) -> SessionAgentDecision {
        SessionAgentDecision {
            intent: SessionIntent::GeneralReply,
            primary_object_type: None,
            primary_object_id: None,
            action_type: SessionActionType::DirectReply,
            next_action: SessionNextAction::DirectReply,
            tool_invocation: None,
            skill_selection: None,
            run_creation: None,
            reply_text: reply_text.to_string(),
            suggested_run_type: None,
            session_summary: Some("LLM replied to the current session message".to_string()),
            should_continue_planning: false,
            failure_hint: None,
        }
    }

    fn fallback_clarification_decision() -> SessionAgentDecision {
        SessionAgentDecision {
            intent: SessionIntent::GeneralReply,
            primary_object_type: None,
            primary_object_id: None,
            action_type: SessionActionType::RequestClarification,
            next_action: SessionNextAction::RequestClarification,
            tool_invocation: None,
            skill_selection: None,
            run_creation: None,
            reply_text: "I need a bit more context before I can decide the next step for this session.".to_string(),
            suggested_run_type: None,
            session_summary: Some("Requesting clarification for the current session turn".to_string()),
            should_continue_planning: true,
            failure_hint: Some("clarify_or_stop".to_string()),
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
            stream: None,
        };

        let response = send_chat_completion_request(&self.client, &self.config, &request).await?;

        let raw_output = response
            .first_message_content()
            .ok_or_else(|| {
                AgentError::Response("llm response did not contain assistant content".to_string())
            })?
            .to_string();

        let decision = if raw_output.trim_start().starts_with('{') {
            Self::parse_structured_decision(&raw_output)
                .unwrap_or_else(Self::fallback_clarification_decision)
        } else {
            Self::fallback_direct_reply_decision(&raw_output)
        };

        Ok(LlmSessionAgentDebugResult {
            raw_output,
            decision,
        })
    }

    pub async fn decide_with_stream<F>(
        &self,
        input: SessionAgentInput,
        mut on_reply_chunk: F,
    ) -> Result<LlmSessionAgentDebugResult, AgentError>
    where
        F: FnMut(&str),
    {
        let mut debug_result = self.decide_with_debug(input.clone()).await?;

        if debug_result.decision.action_type != SessionActionType::DirectReply {
            return Ok(debug_result);
        }

        let stream_messages = self.build_direct_reply_stream_messages(&input, &debug_result.decision);
        let request = OpenAiCompatibleChatRequest {
            model: self.config.model.clone(),
            messages: stream_messages,
            stream: Some(true),
        };

        let mut streamed_reply_from_chunks = String::new();
        let mut emitted_any_chunk = false;
        let streamed_reply_result = stream_chat_completion_request(
            &self.client,
            &self.config,
            &request,
            |chunk| {
                if chunk.is_empty() {
                    return;
                }
                emitted_any_chunk = true;
                streamed_reply_from_chunks.push_str(chunk);
                on_reply_chunk(chunk);
            },
        )
        .await;

        let streamed_reply = match streamed_reply_result {
            Ok(full_reply) if !full_reply.trim().is_empty() => full_reply,
            Ok(_) => streamed_reply_from_chunks.clone(),
            Err(_) => {
                let fallback_request = OpenAiCompatibleChatRequest {
                    model: self.config.model.clone(),
                    messages: self.build_direct_reply_stream_messages(&input, &debug_result.decision),
                    stream: None,
                };

                let fallback_response =
                    send_chat_completion_request(&self.client, &self.config, &fallback_request).await;

                match fallback_response {
                    Ok(response) => response
                        .first_message_content()
                        .map(str::to_string)
                        .unwrap_or_else(|| streamed_reply_from_chunks.clone()),
                    Err(_) => streamed_reply_from_chunks.clone(),
                }
            }
        };

        let final_reply_text = if streamed_reply.trim().is_empty() {
            debug_result.decision.reply_text.clone()
        } else {
            streamed_reply
        };

        if !emitted_any_chunk && !final_reply_text.trim().is_empty() {
            on_reply_chunk(&final_reply_text);
        } else if emitted_any_chunk && !streamed_reply_from_chunks.is_empty() {
            if let Some(suffix) = final_reply_text.strip_prefix(&streamed_reply_from_chunks) {
                if !suffix.is_empty() {
                    on_reply_chunk(suffix);
                }
            }
        }

        debug_result.decision.reply_text = final_reply_text;

        Ok(debug_result)
    }
}

#[async_trait]
impl SessionAgent for LlmSessionAgent {
    async fn decide(&self, input: SessionAgentInput) -> Result<SessionAgentDecision, AgentError> {
        Ok(self.decide_with_debug(input).await?.decision)
    }
}

fn extract_first_url(text: &str) -> Option<String> {
    text.split_whitespace()
        .find(|token| token.starts_with("http://") || token.starts_with("https://"))
        .map(|token| {
            token
                .trim_end_matches(|ch: char| matches!(ch, '.' | ',' | ')' | ']' | '}' | '"' | '\''))
                .to_string()
        })
}

#[cfg(test)]
mod tests {
    use super::{
        BasicSessionAgent, LlmSessionAgent, RunCreationRequest, SessionActionType, SessionAgent,
        SessionAgentDecision, SessionAgentInput, SessionIntent, SessionNextAction,
        SkillSelection,
        session_agent_definition,
    };
    use crate::LlmProviderConfig;
    use schema::{
        AttachmentRef, Session, SessionIntake, SessionMessage, SessionMessageRole,
        SessionStatus,
    };
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
    fn session_agent_definition_describes_session_level_planning_role() {
        let definition = session_agent_definition();

        assert!(definition.responsibility_summary.contains("next action"));
        assert!(
            definition
                .responsibility_summary
                .contains("after actions succeed or fail")
        );
        assert!(definition.system_prompt.contains("post-action follow-up"));
        assert!(definition.system_prompt.contains("failure handling"));
    }

    #[test]
    fn session_agent_definition_exposes_allowed_skill_keys() {
        let definition = session_agent_definition();

        assert!(definition.allowed_skill_keys.contains(&"import_and_distill_skill".to_string()));
        assert!(definition.allowed_skill_keys.contains(&"deepen_asset_skill".to_string()));
        assert!(definition.allowed_skill_keys.contains(&"compose_and_verify_skill".to_string()));
    }

    #[test]
    fn session_agent_definition_declares_tools_used_by_examples_and_skills() {
        let definition = session_agent_definition();

        assert!(definition.allowed_tool_keys.contains(&"search_memory".to_string()));
        assert!(definition.allowed_tool_keys.contains(&"get_project".to_string()));
        assert!(definition.allowed_tool_keys.contains(&"get_asset".to_string()));
    }

    #[test]
    fn session_agent_prompt_explicitly_allows_follow_up_tool_calls_when_one_result_is_insufficient() {
        let definition = session_agent_definition();

        assert!(definition.system_prompt.contains("If one tool result is insufficient"));
    }

    #[test]
    fn session_agent_decision_uses_structured_action_type() {
        let decision = SessionAgentDecision {
            intent: SessionIntent::DistillMaterial,
            primary_object_type: Some("source".to_string()),
            primary_object_id: None,
            action_type: SessionActionType::CreateRun,
            next_action: SessionNextAction::CreateRun(RunCreationRequest {
                run_type: "import_and_distill".to_string(),
                reasoning_summary: None,
            }),
            tool_invocation: None,
            skill_selection: None,
            run_creation: Some(RunCreationRequest {
                run_type: "import_and_distill".to_string(),
                reasoning_summary: None,
            }),
            reply_text: "I will start a distill run for this work material.".to_string(),
            suggested_run_type: Some("import_and_distill".to_string()),
            session_summary: Some("Preparing to distill work material".to_string()),
            should_continue_planning: true,
            failure_hint: Some("clarify_or_stop".to_string()),
        };

        assert_eq!(decision.action_type, SessionActionType::CreateRun);
        assert!(matches!(decision.next_action, SessionNextAction::CreateRun(_)));
        assert!(decision.should_continue_planning);
        assert_eq!(decision.failure_hint.as_deref(), Some("clarify_or_stop"));
    }

    #[tokio::test]
    async fn skill_selection_contract_stays_distinct_from_run_and_tool_contracts() {
        let selection = SkillSelection::new("compose_and_verify_skill")
            .with_reasoning("Need a reusable output strategy")
            .with_expected_outcome("Produce a checked draft");

        assert_eq!(selection.skill_key, "compose_and_verify_skill");
        assert_eq!(
            selection.reasoning_summary.as_deref(),
            Some("Need a reusable output strategy")
        );
        assert_eq!(
            selection.expected_outcome.as_deref(),
            Some("Produce a checked draft")
        );
    }

    #[tokio::test]
    async fn basic_session_agent_keeps_distill_material_on_create_run() {
        let decision = BasicSessionAgent
            .decide(SessionAgentInput {
                session: Session {
                    id: "session-1".to_string(),
                    title: "Distill Session".to_string(),
                    status: SessionStatus::Active,
                    current_intent: "idle".to_string(),
                    current_object_type: "none".to_string(),
                    current_object_id: "none".to_string(),
                    summary: "Distill something".to_string(),
                    started_at: "2026-03-28T00:00:00Z".to_string(),
                    updated_at: "2026-03-28T00:00:00Z".to_string(),
                    last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                    last_run_at: "2026-03-28T00:00:00Z".to_string(),
                    last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                    metadata_json: "{}".to_string(),
                },
                recent_messages: vec![],
                intake: SessionIntake {
                    session_id: "session-1".to_string(),
                    user_message: "Please distill these work notes into Distilllab".to_string(),
                    attachments: vec![],
                    current_object_type: None,
                    current_object_id: None,
                },
            })
            .await
            .expect("basic session agent should decide");

        assert_eq!(decision.intent, SessionIntent::DistillMaterial);
        assert_eq!(decision.action_type, SessionActionType::CreateRun);
        assert!(matches!(decision.next_action, SessionNextAction::CreateRun(_)));
        assert_eq!(
            decision.run_creation.as_ref().map(|value| value.run_type.as_str()),
            Some("import_and_distill")
        );
        assert!(decision.skill_selection.is_none());
    }

    #[tokio::test]
    async fn basic_session_agent_can_use_tool_call_for_attachment_content_questions() {
        let decision = BasicSessionAgent
            .decide(SessionAgentInput {
                session: Session {
                    id: "session-1".to_string(),
                    title: "Attachment Session".to_string(),
                    status: SessionStatus::Active,
                    current_intent: "idle".to_string(),
                    current_object_type: "none".to_string(),
                    current_object_id: "none".to_string(),
                    summary: "Attachment inspection session".to_string(),
                    started_at: "2026-03-28T00:00:00Z".to_string(),
                    updated_at: "2026-03-28T00:00:00Z".to_string(),
                    last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                    last_run_at: "2026-03-28T00:00:00Z".to_string(),
                    last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                    metadata_json: "{}".to_string(),
                },
                recent_messages: vec![],
                intake: SessionIntake {
                    session_id: "session-1".to_string(),
                    user_message: "What is inside attachment paths?".to_string(),
                    attachments: vec![AttachmentRef {
                        attachment_id: "attachment-1".to_string(),
                        kind: "file_copy".to_string(),
                        name: "notes.md".to_string(),
                        mime_type: "text/markdown".to_string(),
                        path_or_locator: "/tmp/distilllab/attachments/session-1/notes.md".to_string(),
                        size: 128,
                        metadata_json: "{}".to_string(),
                    }],
                    current_object_type: None,
                    current_object_id: None,
                },
            })
            .await
            .expect("basic session agent should decide");

        assert_eq!(decision.action_type, SessionActionType::ToolCall);
        assert!(matches!(decision.next_action, SessionNextAction::ToolCall(_)));
        assert_eq!(
            decision.tool_invocation.as_ref().map(|value| value.tool_name.as_str()),
            Some("read_attachment_excerpt")
        );
        assert!(decision.should_continue_planning);
    }

    #[tokio::test]
    async fn basic_session_agent_can_list_attachments_when_user_asks_what_is_available() {
        let decision = BasicSessionAgent
            .decide(SessionAgentInput {
                session: Session {
                    id: "session-1".to_string(),
                    title: "Attachment Session".to_string(),
                    status: SessionStatus::Active,
                    current_intent: "idle".to_string(),
                    current_object_type: "none".to_string(),
                    current_object_id: "none".to_string(),
                    summary: "Attachment inspection session".to_string(),
                    started_at: "2026-03-28T00:00:00Z".to_string(),
                    updated_at: "2026-03-28T00:00:00Z".to_string(),
                    last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                    last_run_at: "2026-03-28T00:00:00Z".to_string(),
                    last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                    metadata_json: "{}".to_string(),
                },
                recent_messages: vec![],
                intake: SessionIntake {
                    session_id: "session-1".to_string(),
                    user_message: "What attachments are available right now?".to_string(),
                    attachments: vec![AttachmentRef {
                        attachment_id: "attachment-1".to_string(),
                        kind: "file_copy".to_string(),
                        name: "notes.md".to_string(),
                        mime_type: "text/markdown".to_string(),
                        path_or_locator: "/tmp/distilllab/attachments/session-1/notes.md".to_string(),
                        size: 128,
                        metadata_json: "{}".to_string(),
                    }],
                    current_object_type: None,
                    current_object_id: None,
                },
            })
            .await
            .expect("basic session agent should decide");

        assert_eq!(decision.action_type, SessionActionType::ToolCall);
        assert_eq!(
            decision.tool_invocation.as_ref().map(|value| value.tool_name.as_str()),
            Some("list_attachments")
        );
    }

    #[tokio::test]
    async fn basic_session_agent_can_use_read_text_for_local_file_questions() {
        let decision = BasicSessionAgent
            .decide(SessionAgentInput {
                session: Session {
                    id: "session-1".to_string(),
                    title: "Local File Session".to_string(),
                    status: SessionStatus::Active,
                    current_intent: "idle".to_string(),
                    current_object_type: "none".to_string(),
                    current_object_id: "none".to_string(),
                    summary: "Local file inspection".to_string(),
                    started_at: "2026-03-28T00:00:00Z".to_string(),
                    updated_at: "2026-03-28T00:00:00Z".to_string(),
                    last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                    last_run_at: "2026-03-28T00:00:00Z".to_string(),
                    last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                    metadata_json: "{}".to_string(),
                },
                recent_messages: vec![],
                intake: SessionIntake {
                    session_id: "session-1".to_string(),
                    user_message: "Please read the current attachment text before answering.".to_string(),
                    attachments: vec![AttachmentRef {
                        attachment_id: "attachment-1".to_string(),
                        kind: "file_copy".to_string(),
                        name: "notes.md".to_string(),
                        mime_type: "text/markdown".to_string(),
                        path_or_locator: "/tmp/distilllab/attachments/session-1/notes.md".to_string(),
                        size: 128,
                        metadata_json: "{}".to_string(),
                    }],
                    current_object_type: None,
                    current_object_id: None,
                },
            })
            .await
            .expect("basic session agent should decide");

        assert_eq!(decision.action_type, SessionActionType::ToolCall);
        assert_eq!(
            decision.tool_invocation.as_ref().map(|value| value.tool_name.as_str()),
            Some("read_text")
        );
    }

    #[tokio::test]
    async fn basic_session_agent_can_use_web_fetch_for_url_questions() {
        let decision = BasicSessionAgent
            .decide(SessionAgentInput {
                session: Session {
                    id: "session-1".to_string(),
                    title: "Web Session".to_string(),
                    status: SessionStatus::Active,
                    current_intent: "idle".to_string(),
                    current_object_type: "none".to_string(),
                    current_object_id: "none".to_string(),
                    summary: "Web inspection".to_string(),
                    started_at: "2026-03-28T00:00:00Z".to_string(),
                    updated_at: "2026-03-28T00:00:00Z".to_string(),
                    last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                    last_run_at: "2026-03-28T00:00:00Z".to_string(),
                    last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                    metadata_json: "{}".to_string(),
                },
                recent_messages: vec![],
                intake: SessionIntake {
                    session_id: "session-1".to_string(),
                    user_message: "Please check what https://example.com says before answering.".to_string(),
                    attachments: vec![],
                    current_object_type: None,
                    current_object_id: None,
                },
            })
            .await
            .expect("basic session agent should decide");

        assert_eq!(decision.action_type, SessionActionType::ToolCall);
        assert_eq!(
            decision.tool_invocation.as_ref().map(|value| value.tool_name.as_str()),
            Some("web_fetch")
        );
        assert_eq!(
            decision
                .tool_invocation
                .as_ref()
                .and_then(|value| value.arguments.get("url"))
                .and_then(|value| value.as_str()),
            Some("https://example.com")
        );
    }

    #[tokio::test]
    async fn basic_session_agent_keeps_distill_requests_ahead_of_web_fetch_heuristics() {
        let decision = BasicSessionAgent
            .decide(SessionAgentInput {
                session: Session {
                    id: "session-1".to_string(),
                    title: "Distill URL Session".to_string(),
                    status: SessionStatus::Active,
                    current_intent: "idle".to_string(),
                    current_object_type: "none".to_string(),
                    current_object_id: "none".to_string(),
                    summary: "Distill URL request".to_string(),
                    started_at: "2026-03-28T00:00:00Z".to_string(),
                    updated_at: "2026-03-28T00:00:00Z".to_string(),
                    last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                    last_run_at: "2026-03-28T00:00:00Z".to_string(),
                    last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                    metadata_json: "{}".to_string(),
                },
                recent_messages: vec![],
                intake: SessionIntake {
                    session_id: "session-1".to_string(),
                    user_message: "Please distill this URL https://example.com into Distilllab"
                        .to_string(),
                    attachments: vec![],
                    current_object_type: None,
                    current_object_id: None,
                },
            })
            .await
            .expect("basic session agent should decide");

        assert_eq!(decision.action_type, SessionActionType::CreateRun);
        assert_eq!(decision.suggested_run_type.as_deref(), Some("import_and_distill"));
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
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "Import these notes".to_string(),
                attachments: vec![],
                current_object_type: None,
                current_object_id: None,
            },
        };

        assert_eq!(input.session.id, "session-1");
        assert_eq!(input.recent_messages.len(), 1);
        assert_eq!(input.intake.user_message, "Import these notes");
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
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "Hello Distilllab".to_string(),
                attachments: vec![],
                current_object_type: None,
                current_object_id: None,
            },
        };

        let decision = agent
            .decide(input)
            .await
            .expect("basic session agent should decide");

        assert_eq!(decision.action_type, SessionActionType::DirectReply);
        assert_eq!(decision.intent, SessionIntent::GeneralReply);
        assert!(decision.tool_invocation.is_none());
        assert!(!decision.should_continue_planning);
        assert_eq!(decision.failure_hint, None);
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
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "Please distill these work notes into Distilllab".to_string(),
                attachments: vec![],
                current_object_type: None,
                current_object_id: None,
            },
        };

        let decision = agent
            .decide(input)
            .await
            .expect("basic session agent should decide");

        assert_eq!(decision.action_type, SessionActionType::CreateRun);
        assert_eq!(decision.intent, SessionIntent::DistillMaterial);
        assert!(decision.tool_invocation.is_none());
        assert_eq!(
            decision.suggested_run_type,
            Some("import_and_distill".to_string())
        );
    }

    #[tokio::test]
    async fn basic_session_agent_treats_here_are_my_notes_as_distill_material_intake() {
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
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "Here are my notes, distill them".to_string(),
                attachments: vec![],
                current_object_type: None,
                current_object_id: None,
            },
        };

        let decision = agent
            .decide(input)
            .await
            .expect("basic session agent should decide");

        assert_eq!(decision.intent, SessionIntent::DistillMaterial);
        assert_eq!(decision.action_type, SessionActionType::CreateRun);
        assert!(decision.tool_invocation.is_none());
        assert_eq!(decision.suggested_run_type.as_deref(), Some("import_and_distill"));
    }

    #[tokio::test]
    async fn basic_session_agent_treats_file_based_request_as_distill_material_intake() {
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
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "Use these files to create a distillation run".to_string(),
                attachments: vec![AttachmentRef {
                    attachment_id: "attachment-1".to_string(),
                    kind: "file_path".to_string(),
                    name: "requirements.md".to_string(),
                    mime_type: "text/markdown".to_string(),
                    path_or_locator: "/tmp/requirements.md".to_string(),
                    size: 256,
                    metadata_json: "{}".to_string(),
                }],
                current_object_type: None,
                current_object_id: None,
            },
        };

        let decision = agent
            .decide(input)
            .await
            .expect("basic session agent should decide");

        assert_eq!(decision.intent, SessionIntent::DistillMaterial);
        assert_eq!(decision.action_type, SessionActionType::CreateRun);
        assert!(decision.tool_invocation.is_none());
        assert_eq!(decision.suggested_run_type.as_deref(), Some("import_and_distill"));
    }

    #[tokio::test]
    async fn basic_session_agent_treats_attachment_only_input_as_distill_material_intake() {
        let agent = BasicSessionAgent;

        let input = SessionAgentInput {
            session: Session {
                id: "session-1".to_string(),
                title: "Attachment Intake Session".to_string(),
                status: SessionStatus::Active,
                current_intent: "idle".to_string(),
                current_object_type: "none".to_string(),
                current_object_id: "none".to_string(),
                summary: "Testing attachment-driven intake routing".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![],
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "请帮我提炼一下".to_string(),
                attachments: vec![AttachmentRef {
                    attachment_id: "attachment-1".to_string(),
                    kind: "file_path".to_string(),
                    name: "requirements.md".to_string(),
                    mime_type: "text/markdown".to_string(),
                    path_or_locator: "/tmp/requirements.md".to_string(),
                    size: 256,
                    metadata_json: "{}".to_string(),
                }],
                current_object_type: None,
                current_object_id: None,
            },
        };

        let decision = agent
            .decide(input)
            .await
            .expect("basic session agent should decide");

        assert_eq!(decision.intent, SessionIntent::DistillMaterial);
        assert_eq!(decision.action_type, SessionActionType::CreateRun);
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
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "Please deepen this topic and ask follow-up questions".to_string(),
                attachments: vec![],
                current_object_type: Some("asset".to_string()),
                current_object_id: Some("asset-1".to_string()),
            },
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
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "Write a summary article from these materials".to_string(),
                attachments: vec![],
                current_object_type: Some("project".to_string()),
                current_object_id: Some("project-1".to_string()),
            },
        };

        let decision = agent
            .decide(input)
            .await
            .expect("basic session agent should decide");

        assert_eq!(decision.action_type, SessionActionType::CreateRun);
        assert_eq!(decision.intent, SessionIntent::ComposeOutput);
        assert_eq!(decision.primary_object_type.as_deref(), Some("project"));
        assert_eq!(decision.primary_object_id.as_deref(), Some("project-1"));
        assert_eq!(
            decision.run_creation.as_ref().map(|value| value.run_type.as_str()),
            Some("compose_and_verify")
        );
        assert!(decision.skill_selection.is_none());
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
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "Hello from the user".to_string(),
                attachments: vec![],
                current_object_type: None,
                current_object_id: None,
            },
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
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "Current user follow-up".to_string(),
                attachments: vec![],
                current_object_type: None,
                current_object_id: None,
            },
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
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "Please distill these work notes".to_string(),
                attachments: vec![],
                current_object_type: None,
                current_object_id: None,
            },
        };

        let messages = agent.build_chat_messages(&input);

        assert!(messages.len() >= 4);
        assert_eq!(messages[1].role, "user");
        assert!(messages[1].content.contains("distill these work notes"));
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
        assert!(joined.contains("\"tool_invocation\":"));
        assert!(joined.contains("list_attachments"));
        assert!(joined.contains("read_text"));
        assert!(joined.contains("web_fetch"));
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
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "Summarize the current source".to_string(),
                attachments: vec![],
                current_object_type: Some("source".to_string()),
                current_object_id: Some("source-1".to_string()),
            },
        };

        let messages = agent.build_chat_messages(&input);

        assert!(messages[0].content.contains("current_object_type: source"));
        assert!(messages[0].content.contains("current_object_id: source-1"));
        assert!(messages[0].content.contains("session_summary: The session is focused on one imported source"));
    }

    #[test]
    fn llm_session_agent_includes_current_intake_attachments_in_system_context() {
        let agent = LlmSessionAgent::new(LlmProviderConfig {
            provider_kind: "openai_compatible".to_string(),
            base_url: "http://localhost:11434/v1".to_string(),
            model: "qwen-test".to_string(),
            api_key: None,
        });

        let input = SessionAgentInput {
            session: Session {
                id: "session-1".to_string(),
                title: "Attachment Session".to_string(),
                status: SessionStatus::Active,
                current_intent: "idle".to_string(),
                current_object_type: "none".to_string(),
                current_object_id: "none".to_string(),
                summary: "Testing attachment context".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![],
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "What is inside Attachment Paths?".to_string(),
                attachments: vec![AttachmentRef {
                    attachment_id: "attachment-1".to_string(),
                    kind: "file_copy".to_string(),
                    name: "notes.md".to_string(),
                    mime_type: "text/markdown".to_string(),
                    path_or_locator: "/tmp/distilllab/attachments/session-1/notes.md".to_string(),
                    size: 128,
                    metadata_json: "{}".to_string(),
                }],
                current_object_type: None,
                current_object_id: None,
            },
        };

        let messages = agent.build_chat_messages(&input);

        assert!(messages[0].content.contains("current_intake_attachments:"));
        assert!(messages[0].content.contains("attachment_id: attachment-1"));
        assert!(messages[0].content.contains("name: notes.md"));
        assert!(messages[0].content.contains("mime_type: text/markdown"));
        assert!(!messages[0].content.contains("locator:"));
    }

    #[test]
    fn llm_session_agent_includes_attachment_excerpt_in_system_context_when_file_is_readable() {
        let agent = LlmSessionAgent::new(LlmProviderConfig {
            provider_kind: "openai_compatible".to_string(),
            base_url: "http://localhost:11434/v1".to_string(),
            model: "qwen-test".to_string(),
            api_key: None,
        });
        let temp_dir = std::env::temp_dir().join("distilllab-attachment-context-fixed");
        std::fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let attachment_path = temp_dir.join("notes.md");
        std::fs::write(
            &attachment_path,
            "Attachment heading\nThis attachment contains project notes.",
        )
        .expect("attachment should be written");

        let input = SessionAgentInput {
            session: Session {
                id: "session-1".to_string(),
                title: "Attachment Session".to_string(),
                status: SessionStatus::Active,
                current_intent: "idle".to_string(),
                current_object_type: "none".to_string(),
                current_object_id: "none".to_string(),
                summary: "Testing attachment excerpt context".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![],
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "What is inside Attachment Paths?".to_string(),
                attachments: vec![AttachmentRef {
                    attachment_id: "attachment-1".to_string(),
                    kind: "file_copy".to_string(),
                    name: "notes.md".to_string(),
                    mime_type: "text/markdown".to_string(),
                    path_or_locator: attachment_path.to_string_lossy().to_string(),
                    size: 128,
                    metadata_json: "{}".to_string(),
                }],
                current_object_type: None,
                current_object_id: None,
            },
        };

        let messages = agent.build_chat_messages(&input);

        assert!(!messages[0].content.contains("excerpt:"));
        assert!(!messages[0].content.contains("Attachment heading"));

        let _ = std::fs::remove_file(&attachment_path);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn llm_session_agent_does_not_duplicate_current_turn_if_it_is_already_in_recent_messages() {
        let agent = LlmSessionAgent::new(LlmProviderConfig {
            provider_kind: "openai_compatible".to_string(),
            base_url: "http://localhost:11434/v1".to_string(),
            model: "qwen-test".to_string(),
            api_key: None,
        });

        let input = SessionAgentInput {
            session: Session {
                id: "session-1".to_string(),
                title: "Duplicate Turn Session".to_string(),
                status: SessionStatus::Active,
                current_intent: "idle".to_string(),
                current_object_type: "none".to_string(),
                current_object_id: "none".to_string(),
                summary: "Testing duplicate current turn handling".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![SessionMessage {
                id: "message-current".to_string(),
                session_id: "session-1".to_string(),
                run_id: None,
                message_type: "user_message".to_string(),
                role: SessionMessageRole::User,
                content: "Current message".to_string(),
                data_json: "{}".to_string(),
                created_at: "2026-03-28T00:00:00Z".to_string(),
            }],
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "Current message".to_string(),
                attachments: vec![],
                current_object_type: None,
                current_object_id: None,
            },
        };

        let messages = agent.build_chat_messages(&input);
        let current_turn_count = messages
            .iter()
            .filter(|message| message.content == "Current message")
            .count();

        assert_eq!(current_turn_count, 1);
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
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "Say hello".to_string(),
                attachments: vec![],
                current_object_type: None,
                current_object_id: None,
            },
        };

        let decision = agent.decide(input).await.expect("llm session agent should decide");

        assert_eq!(decision.action_type, SessionActionType::DirectReply);
        assert_eq!(decision.intent, SessionIntent::GeneralReply);
        assert_eq!(decision.reply_text, "Hello from fake llm");
    }

    #[tokio::test]
    async fn llm_session_agent_returns_safe_clarification_when_create_run_json_is_invalid() {
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
                            "content": "{\"intent\":\"compose_output\",\"action_type\":\"create_run\",\"reply_text\":\"I will start a run.\",\"suggested_run_type\":null,\"session_summary\":\"Preparing to compose output\",\"tool_call_key\":null,\"should_continue_planning\":true,\"failure_hint\":\"clarify_or_stop\"}"
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
            model: "qwen-test".to_string(),
            api_key: None,
        });

        let input = SessionAgentInput {
            session: Session {
                id: "session-1".to_string(),
                title: "Safe fallback session".to_string(),
                status: SessionStatus::Active,
                current_intent: "idle".to_string(),
                current_object_type: "none".to_string(),
                current_object_id: "none".to_string(),
                summary: "Testing invalid create_run fallback".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![],
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "Write an output for this project".to_string(),
                attachments: vec![],
                current_object_type: None,
                current_object_id: None,
            },
        };

        let result = agent
            .decide_with_debug(input)
            .await
            .expect("llm planner should return a safe fallback decision");

        assert_eq!(result.decision.action_type, SessionActionType::RequestClarification);
        assert!(!result.decision.reply_text.contains("\"intent\""));
    }

    #[test]
    fn llm_session_agent_rejects_create_run_without_valid_run_type() {
        let raw_json = r#"{"intent":"compose_output","action_type":"create_run","reply_text":"I will start a run.","primary_object_type":"project","primary_object_id":"project-1","suggested_run_type":null,"session_summary":"Preparing to compose output","tool_invocation":null,"should_continue_planning":true,"failure_hint":"clarify_or_stop"}"#;

        let decision = LlmSessionAgent::parse_structured_decision(raw_json);

        assert!(decision.is_none());
    }

    #[test]
    fn llm_session_agent_rejects_tool_call_without_tool_invocation() {
        let raw_json = r#"{"intent":"general_reply","action_type":"tool_call","reply_text":"I will use a tool.","primary_object_type":null,"primary_object_id":null,"suggested_run_type":null,"session_summary":"Preparing tool use","tool_invocation":null,"should_continue_planning":true,"failure_hint":"reply_or_clarify"}"#;

        let decision = LlmSessionAgent::parse_structured_decision(raw_json);

        assert!(decision.is_none());
    }

    #[test]
    fn llm_session_agent_rejects_non_tool_action_with_tool_invocation() {
        let raw_json = r#"{"intent":"general_reply","action_type":"direct_reply","reply_text":"Here is the answer.","primary_object_type":null,"primary_object_id":null,"suggested_run_type":null,"session_summary":"Providing answer","tool_invocation":{"tool_name":"search_memory","arguments":{},"reasoning_summary":null,"expected_follow_up":null},"should_continue_planning":false,"failure_hint":null}"#;

        let decision = LlmSessionAgent::parse_structured_decision(raw_json);

        assert!(decision.is_none());
    }

    #[test]
    fn llm_session_agent_keeps_legacy_arguments_json_as_compatibility_path() {
        let raw_json = r#"{"intent":"general_reply","action_type":"tool_call","reply_text":"I will use search before replying.","primary_object_type":null,"primary_object_id":null,"suggested_run_type":null,"session_summary":"Preparing memory lookup","tool_invocation":{"tool_name":"search_memory","arguments_json":"{\"query\":\"legacy\"}","reasoning_summary":null,"expected_follow_up":null},"should_continue_planning":true,"failure_hint":"reply_or_clarify"}"#;

        let decision = LlmSessionAgent::parse_structured_decision(raw_json)
            .expect("legacy arguments_json should still parse");

        assert_eq!(decision.action_type, SessionActionType::ToolCall);
        assert_eq!(
            decision
                .tool_invocation
                .as_ref()
                .and_then(|value| value.arguments.get("query"))
                .and_then(|value| value.as_str()),
            Some("legacy")
        );
    }

    #[test]
    fn llm_session_agent_defaults_follow_up_planning_for_legacy_tool_call_json() {
        let raw_json = r#"{"intent":"general_reply","action_type":"tool_call","reply_text":"I will look up related notes before replying.","primary_object_type":null,"primary_object_id":null,"suggested_run_type":null,"session_summary":"Preparing a memory lookup before answering","tool_call_key":"search_memory"}"#;

        let decision = LlmSessionAgent::parse_structured_decision(raw_json)
            .expect("legacy tool_call json should still parse");

        assert_eq!(decision.action_type, SessionActionType::ToolCall);
        assert!(decision.should_continue_planning);
        assert_eq!(
            decision.tool_invocation.as_ref().map(|value| value.tool_name.as_str()),
            Some("search_memory")
        );
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
                            "content": "{\"intent\":\"distill_material\",\"action_type\":\"create_run\",\"reply_text\":\"I will start a distill run for this work material.\",\"suggested_run_type\":\"import_and_distill\",\"session_summary\":\"Preparing to distill work material\",\"tool_call_key\":null}"
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
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "Distill these work notes".to_string(),
                attachments: vec![],
                current_object_type: None,
                current_object_id: None,
            },
        };

        let decision = agent.decide(input).await.expect("llm session agent should decide");

        assert_eq!(decision.action_type, SessionActionType::CreateRun);
        assert_eq!(decision.intent, SessionIntent::DistillMaterial);
        assert_eq!(decision.suggested_run_type.as_deref(), Some("import_and_distill"));
        assert_eq!(decision.reply_text, "I will start a distill run for this work material.");
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
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "Hello".to_string(),
                attachments: vec![],
                current_object_type: None,
                current_object_id: None,
            },
        };

        let result = agent
            .decide_with_debug(input)
            .await
            .expect("llm session agent debug should decide");

        assert!(result.raw_output.contains("\"intent\":\"general_reply\""));
        assert_eq!(result.decision.intent, SessionIntent::GeneralReply);
        assert_eq!(result.decision.reply_text, "Here is the answer.");
    }

    #[tokio::test]
    async fn llm_session_agent_normalizes_distill_material_direct_reply_into_create_run() {
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
                            "content": "{\"intent\":\"distill_material\",\"action_type\":\"direct_reply\",\"reply_text\":\"I extracted requirement points for you.\",\"primary_object_type\":\"material\",\"primary_object_id\":null,\"suggested_run_type\":null,\"session_summary\":\"User provided work content\",\"tool_call_key\":null}"
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
                title: "Normalization Session".to_string(),
                status: SessionStatus::Active,
                current_intent: "idle".to_string(),
                current_object_type: "none".to_string(),
                current_object_id: "none".to_string(),
                summary: "Testing distill intent normalization".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![],
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "Please distill these work items".to_string(),
                attachments: vec![],
                current_object_type: None,
                current_object_id: None,
            },
        };

        let decision = agent.decide(input).await.expect("llm session agent should decide");

        assert_eq!(decision.intent, SessionIntent::DistillMaterial);
        assert_eq!(decision.action_type, SessionActionType::CreateRun);
        assert_eq!(decision.suggested_run_type.as_deref(), Some("import_and_distill"));
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
        assert!(definition.system_prompt.contains("distill_material"));
        assert!(definition.system_prompt.contains("deepen_understanding"));
        assert!(definition.system_prompt.contains("compose_output"));
        assert!(definition.system_prompt.contains("action_type"));
        assert!(definition.system_prompt.contains("reply_text"));
        assert!(definition.system_prompt.contains("suggested_run_type"));
        assert!(definition.system_prompt.contains("direct_reply"));
        assert!(definition.system_prompt.contains("request_clarification"));
        assert!(definition.system_prompt.contains("create_run"));
    }

    #[tokio::test]
    async fn llm_session_agent_streaming_direct_reply_uses_second_plain_text_call() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let address = listener
            .local_addr()
            .expect("listener should have local addr");

        tokio::spawn(async move {
            let (mut stream_one, _) = listener
                .accept()
                .await
                .expect("server should accept first connection");
            let mut buffer_one = [0_u8; 8192];
            let bytes_one = stream_one
                .read(&mut buffer_one)
                .await
                .expect("server should read first request");
            let request_one = String::from_utf8_lossy(&buffer_one[..bytes_one]);
            assert!(request_one.contains("\"stream\":false") || !request_one.contains("\"stream\":"));

            let planner_json = "{\"intent\":\"general_reply\",\"action_type\":\"direct_reply\",\"reply_text\":\"Draft planner reply\",\"primary_object_type\":null,\"primary_object_id\":null,\"suggested_run_type\":null,\"session_summary\":\"Providing direct reply\",\"tool_invocation\":null,\"skill_selection\":null,\"should_continue_planning\":false,\"failure_hint\":null}";
            let encoded_planner_json = serde_json::to_string(planner_json)
                .expect("planner json should encode");
            let response_body_one = format!(
                "{{\"choices\":[{{\"message\":{{\"role\":\"assistant\",\"content\":{encoded_planner_json}}}}}]}}"
            );
            let response_one = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body_one.len(),
                response_body_one
            );
            stream_one
                .write_all(response_one.as_bytes())
                .await
                .expect("server should write first response");

            let (mut stream_two, _) = listener
                .accept()
                .await
                .expect("server should accept second connection");
            let mut buffer_two = [0_u8; 8192];
            let bytes_two = stream_two
                .read(&mut buffer_two)
                .await
                .expect("server should read second request");
            let request_two = String::from_utf8_lossy(&buffer_two[..bytes_two]);
            assert!(request_two.contains("\"stream\":true"));

            let response_body_two = concat!(
                "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n",
                "data: {\"choices\":[{\"delta\":{\"content\":\" world\"}}]}\n",
                "data: [DONE]\n",
                "\n"
            );
            let response_two = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body_two.len(),
                response_body_two
            );
            stream_two
                .write_all(response_two.as_bytes())
                .await
                .expect("server should write second response");
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
                title: "Streaming Session".to_string(),
                status: SessionStatus::Active,
                current_intent: "idle".to_string(),
                current_object_type: "none".to_string(),
                current_object_id: "none".to_string(),
                summary: "Testing two-phase streaming direct reply".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![],
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "Hello".to_string(),
                attachments: vec![],
                current_object_type: None,
                current_object_id: None,
            },
        };

        let mut observed_chunks = Vec::new();
        let result = agent
            .decide_with_stream(input, |chunk| observed_chunks.push(chunk.to_string()))
            .await
            .expect("two-phase stream should succeed");

        assert_eq!(observed_chunks, vec!["Hello".to_string(), " world".to_string()]);
        assert_eq!(result.decision.action_type, SessionActionType::DirectReply);
        assert_eq!(result.decision.reply_text, "Hello world");
    }

    #[tokio::test]
    async fn llm_session_agent_streaming_non_direct_reply_skips_second_stream_call() {
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
            let mut buffer = [0_u8; 8192];
            let bytes = stream
                .read(&mut buffer)
                .await
                .expect("server should read request");
            let request = String::from_utf8_lossy(&buffer[..bytes]);
            assert!(request.contains("\"stream\":false") || !request.contains("\"stream\":"));

            let planner_json = "{\"intent\":\"general_reply\",\"action_type\":\"tool_call\",\"reply_text\":\"I will check first\",\"primary_object_type\":null,\"primary_object_id\":null,\"suggested_run_type\":null,\"session_summary\":\"Need tool first\",\"tool_invocation\":{\"tool_name\":\"search_memory\",\"arguments\":{},\"reasoning_summary\":null,\"expected_follow_up\":null},\"skill_selection\":null,\"should_continue_planning\":true,\"failure_hint\":\"reply_or_clarify\"}";
            let encoded_planner_json = serde_json::to_string(planner_json)
                .expect("planner json should encode");
            let response_body = format!(
                "{{\"choices\":[{{\"message\":{{\"role\":\"assistant\",\"content\":{encoded_planner_json}}}}}]}}"
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
                title: "Streaming Session".to_string(),
                status: SessionStatus::Active,
                current_intent: "idle".to_string(),
                current_object_type: "none".to_string(),
                current_object_id: "none".to_string(),
                summary: "Testing non-direct-reply path".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![],
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "Find related notes".to_string(),
                attachments: vec![],
                current_object_type: None,
                current_object_id: None,
            },
        };

        let mut observed_chunks = Vec::new();
        let result = agent
            .decide_with_stream(input, |chunk| observed_chunks.push(chunk.to_string()))
            .await
            .expect("streaming decision should succeed");

        assert!(observed_chunks.is_empty());
        assert_eq!(result.decision.action_type, SessionActionType::ToolCall);
        assert_eq!(
            result
                .decision
                .tool_invocation
                .as_ref()
                .map(|value| value.tool_name.as_str()),
            Some("search_memory")
        );
    }

    #[tokio::test]
    async fn llm_session_agent_streaming_direct_reply_fallback_emits_single_chunk() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let address = listener
            .local_addr()
            .expect("listener should have local addr");

        tokio::spawn(async move {
            // First connection: planner non-stream JSON decision
            let (mut stream_one, _) = listener
                .accept()
                .await
                .expect("server should accept first connection");
            let mut buffer_one = [0_u8; 8192];
            let _ = stream_one
                .read(&mut buffer_one)
                .await
                .expect("server should read first request");

            let planner_json = "{\"intent\":\"general_reply\",\"action_type\":\"direct_reply\",\"reply_text\":\"Draft planner reply\",\"primary_object_type\":null,\"primary_object_id\":null,\"suggested_run_type\":null,\"session_summary\":\"Providing direct reply\",\"tool_invocation\":null,\"skill_selection\":null,\"should_continue_planning\":false,\"failure_hint\":null}";
            let encoded_planner_json = serde_json::to_string(planner_json)
                .expect("planner json should encode");
            let response_body_one = format!(
                "{{\"choices\":[{{\"message\":{{\"role\":\"assistant\",\"content\":{encoded_planner_json}}}}}]}}"
            );
            let response_one = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body_one.len(),
                response_body_one
            );
            stream_one
                .write_all(response_one.as_bytes())
                .await
                .expect("server should write first response");

            // Second connection: broken SSE to force stream parsing error
            let (mut stream_two, _) = listener
                .accept()
                .await
                .expect("server should accept second connection");
            let mut buffer_two = [0_u8; 8192];
            let _ = stream_two
                .read(&mut buffer_two)
                .await
                .expect("server should read second request");

            let response_body_two = concat!(
                "data: {not-valid-json}\n",
                "data: [DONE]\n",
                "\n"
            );
            let response_two = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body_two.len(),
                response_body_two
            );
            stream_two
                .write_all(response_two.as_bytes())
                .await
                .expect("server should write second response");

            // Third connection: non-stream fallback reply
            let (mut stream_three, _) = listener
                .accept()
                .await
                .expect("server should accept third connection");
            let mut buffer_three = [0_u8; 8192];
            let _ = stream_three
                .read(&mut buffer_three)
                .await
                .expect("server should read third request");

            let response_body_three = r#"{
                "choices": [
                    {
                        "message": {
                            "role": "assistant",
                            "content": "Fallback final reply"
                        }
                    }
                ]
            }"#;
            let response_three = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body_three.len(),
                response_body_three
            );
            stream_three
                .write_all(response_three.as_bytes())
                .await
                .expect("server should write third response");
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
                title: "Streaming Session".to_string(),
                status: SessionStatus::Active,
                current_intent: "idle".to_string(),
                current_object_type: "none".to_string(),
                current_object_id: "none".to_string(),
                summary: "Testing fallback streaming path".to_string(),
                started_at: "2026-03-28T00:00:00Z".to_string(),
                updated_at: "2026-03-28T00:00:00Z".to_string(),
                last_user_message_at: "2026-03-28T00:00:00Z".to_string(),
                last_run_at: "2026-03-28T00:00:00Z".to_string(),
                last_compacted_at: "2026-03-28T00:00:00Z".to_string(),
                metadata_json: "{}".to_string(),
            },
            recent_messages: vec![],
            intake: SessionIntake {
                session_id: "session-1".to_string(),
                user_message: "Hello".to_string(),
                attachments: vec![],
                current_object_type: None,
                current_object_id: None,
            },
        };

        let mut observed_chunks = Vec::new();
        let result = agent
            .decide_with_stream(input, |chunk| observed_chunks.push(chunk.to_string()))
            .await
            .expect("streaming decision should succeed with fallback");

        assert_eq!(observed_chunks, vec!["Fallback final reply".to_string()]);
        assert_eq!(result.decision.action_type, SessionActionType::DirectReply);
        assert_eq!(result.decision.reply_text, "Fallback final reply");
    }

}
