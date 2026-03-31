use crate::app::AppRuntime;
use crate::contracts::RunInput;
use crate::services::ToolExecutor;
use agent::{
    BasicSessionAgent, LlmProviderConfig, LlmSessionAgent, SessionAgent, SessionAgentDecision,
    SessionAgentInput, SessionActionType, ToolExecutionResult,
};
use chrono::Utc;
use memory::db::open_database;
use memory::migrations::run_migrations;
use memory::session_message_store::{insert_session_message, list_session_messages_for_session};
use memory::session_store::{get_session_by_id, update_session};
use schema::{SessionIntake, SessionMessage, SessionMessageRole};
use uuid::Uuid;

type RuntimeError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone)]
pub struct IntakeDecisionOutcome {
    pub decision: SessionAgentDecision,
    pub run_input: Option<RunInput>,
    pub created_run: Option<String>,
    pub assistant_message_id: String,
    pub tool_result: Option<ToolExecutionResult>,
}

async fn decide_with_provider(
    provider_config: Option<LlmProviderConfig>,
    input: SessionAgentInput,
) -> Result<SessionAgentDecision, RuntimeError> {
    if let Some(config) = provider_config {
        let session_agent = LlmSessionAgent::new(config);
        Ok(session_agent.decide(input).await?)
    } else {
        let session_agent = BasicSessionAgent;
        Ok(session_agent.decide(input).await?)
    }
}

fn build_session_message(
    session_id: &str,
    run_id: Option<String>,
    message_type: &str,
    role: SessionMessageRole,
    content: String,
) -> SessionMessage {
    SessionMessage {
        id: format!("message-{}", Uuid::new_v4()),
        session_id: session_id.to_string(),
        run_id,
        message_type: message_type.to_string(),
        role,
        content,
        data_json: "{}".to_string(),
        created_at: Utc::now().to_string(),
    }
}

pub async fn decide_and_record_intake(
    runtime: &AppRuntime,
    intake: SessionIntake,
    provider_config: Option<LlmProviderConfig>,
) -> Result<IntakeDecisionOutcome, RuntimeError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let mut session = get_session_by_id(&conn, &intake.session_id)?.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("session not found: {}", intake.session_id),
        )
    })?;

    let user_session_message = build_session_message(
        &session.id,
        None,
        "user_message",
        SessionMessageRole::User,
        intake.user_message.clone(),
    );
    insert_session_message(&conn, &user_session_message)?;

    let recent_messages = list_session_messages_for_session(&conn, &session.id)?;
    let input = SessionAgentInput {
        session: session.clone(),
        recent_messages,
        intake: intake.clone(),
    };

    let mut decision = decide_with_provider(provider_config.clone(), input).await?;
    let mut tool_result = None;

    if decision.action_type == SessionActionType::ToolCall {
        let invocation = decision.tool_invocation.clone().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "tool_call decision missing tool_invocation",
            )
        })?;

        let executor = ToolExecutor::new();
        let executed_tool_result = executor.execute(runtime, &invocation).await;
        tool_result = Some(executed_tool_result.clone());

        let tool_result_message = build_session_message(
            &session.id,
            None,
            "tool_result_message",
            SessionMessageRole::System,
            executed_tool_result
                .rendered_summary
                .clone()
                .or_else(|| executed_tool_result.error_message.clone())
                .unwrap_or_else(|| format!("tool executed: {}", executed_tool_result.tool_name)),
        );
        insert_session_message(&conn, &tool_result_message)?;

        let follow_up_input = SessionAgentInput {
            session: session.clone(),
            recent_messages: list_session_messages_for_session(&conn, &session.id)?,
            intake: intake.clone(),
        };
        decision = decide_with_provider(provider_config.clone(), follow_up_input).await?;
    }

    let assistant_message_type = match decision.action_type {
        SessionActionType::DirectReply => "assistant_message",
        SessionActionType::RequestClarification => "clarification_message",
        SessionActionType::ToolCall => "system_message",
        SessionActionType::CreateRun => "system_message",
    };

    let assistant_session_message = build_session_message(
        &session.id,
        None,
        assistant_message_type,
        SessionMessageRole::Assistant,
        decision.reply_text.clone(),
    );
    insert_session_message(&conn, &assistant_session_message)?;

    let now = Utc::now().to_string();
    session.current_intent = decision.intent.as_str().to_string();
    session.current_object_type = decision
        .primary_object_type
        .clone()
        .unwrap_or_else(|| "none".to_string());
    session.current_object_id = decision
        .primary_object_id
        .clone()
        .unwrap_or_else(|| "none".to_string());
    session.summary = decision
        .session_summary
        .clone()
        .unwrap_or_else(|| session.summary.clone());
    session.updated_at = now.clone();
    session.last_user_message_at = now;
    update_session(&conn, &session)?;

    let run_input = if decision.action_type == SessionActionType::CreateRun {
        Some(RunInput {
            session_id: session.id.clone(),
            trigger_message: intake.user_message.clone(),
            attachment_refs: intake.attachments.clone(),
            current_object_type: intake.current_object_type.clone(),
            current_object_id: intake.current_object_id.clone(),
            decision_summary: decision.reply_text.clone(),
        })
    } else {
        None
    };

    Ok(IntakeDecisionOutcome {
        decision,
        run_input,
        created_run: None,
        assistant_message_id: assistant_session_message.id,
        tool_result,
    })
}
