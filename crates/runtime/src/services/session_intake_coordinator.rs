use crate::app::AppRuntime;
use crate::contracts::RunInput;
use agent::{
    BasicSessionAgent, LlmProviderConfig, LlmSessionAgent, SessionAgent, SessionAgentDecision,
    SessionAgentInput,
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

    let decision = if let Some(config) = provider_config {
        let session_agent = LlmSessionAgent::new(config);
        session_agent.decide(input).await?
    } else {
        let session_agent = BasicSessionAgent;
        session_agent.decide(input).await?
    };

    let assistant_message_type = match decision.action_type {
        agent::SessionActionType::DirectReply => "assistant_message",
        agent::SessionActionType::RequestClarification => "clarification_message",
        agent::SessionActionType::ToolCall => "system_message",
        agent::SessionActionType::CreateRun => "system_message",
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
    session.summary = decision
        .session_summary
        .clone()
        .unwrap_or_else(|| session.summary.clone());
    session.updated_at = now.clone();
    session.last_user_message_at = now;
    update_session(&conn, &session)?;

    let run_input = if decision.action_type == agent::SessionActionType::CreateRun {
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
    })
}
