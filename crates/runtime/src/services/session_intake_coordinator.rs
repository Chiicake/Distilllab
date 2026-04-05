use crate::app::AppRuntime;
use crate::contracts::RunInput;
use crate::services::session_service::{
    derive_initial_session_title, derive_refreshed_session_title, effective_session_title,
    should_assign_initial_session_title, should_attempt_session_title_refresh,
    should_replace_session_title,
};
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

async fn decide_with_provider_streaming<F>(
    provider_config: Option<LlmProviderConfig>,
    input: SessionAgentInput,
    on_reply_chunk: &mut F,
) -> Result<SessionAgentDecision, RuntimeError>
where
    F: FnMut(&str),
{
    if let Some(config) = provider_config {
        let session_agent = LlmSessionAgent::new(config);
        Ok(session_agent
            .decide_with_stream(input, |chunk| on_reply_chunk(chunk))
            .await?
            .decision)
    } else {
        let session_agent = BasicSessionAgent;
        Ok(session_agent.decide(input).await?)
    }
}

fn tool_loop_guard_decision(message: &str) -> SessionAgentDecision {
    SessionAgentDecision {
        intent: agent::SessionIntent::GeneralReply,
        primary_object_type: None,
        primary_object_id: None,
        action_type: SessionActionType::RequestClarification,
        next_action: agent::SessionNextAction::RequestClarification,
        tool_invocation: None,
        skill_selection: None,
        run_creation: None,
        reply_text: message.to_string(),
        suggested_run_type: None,
        session_summary: Some("Requesting clarification after repeated tool loop without progress".to_string()),
        should_continue_planning: false,
        failure_hint: Some("clarify_or_stop".to_string()),
    }
}

fn build_session_message(
    session_id: &str,
    run_id: Option<String>,
    message_type: &str,
    role: SessionMessageRole,
    content: String,
) -> SessionMessage {
    build_session_message_with_data(session_id, run_id, message_type, role, content, "{}".to_string())
}

fn build_session_message_with_data(
    session_id: &str,
    run_id: Option<String>,
    message_type: &str,
    role: SessionMessageRole,
    content: String,
    data_json: String,
) -> SessionMessage {
    SessionMessage {
        id: format!("message-{}", Uuid::new_v4()),
        session_id: session_id.to_string(),
        run_id,
        message_type: message_type.to_string(),
        role,
        content,
        data_json,
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

    let user_session_message = build_session_message_with_data(
        &session.id,
        None,
        "user_message",
        SessionMessageRole::User,
        intake.user_message.clone(),
        serde_json::json!({
            "attachments": intake.attachments,
        })
        .to_string(),
    );
    insert_session_message(&conn, &user_session_message)?;

    let recent_messages = list_session_messages_for_session(&conn, &session.id)?;
    let user_message_count = recent_messages
        .iter()
        .filter(|message| message.role == SessionMessageRole::User)
        .count();
    let input = SessionAgentInput {
        session: session.clone(),
        recent_messages,
        intake: intake.clone(),
    };

    let mut decision = decide_with_provider(provider_config.clone(), input).await?;
    let mut tool_result = None;
    let mut last_tool_signature: Option<String> = None;
    let mut last_tool_result: Option<ToolExecutionResult> = None;
    let mut consecutive_same_failure_retries = 0usize;

    while decision.action_type == SessionActionType::ToolCall {
        let invocation = decision.tool_invocation.clone().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "tool_call decision missing tool_invocation",
            )
        })?;
        let tool_signature = serde_json::json!({
            "tool_name": invocation.tool_name,
            "arguments": invocation.arguments,
        })
        .to_string();

        if last_tool_signature.as_deref() == Some(tool_signature.as_str()) {
            if last_tool_result.as_ref().is_some_and(|result| result.ok) {
                decision = tool_loop_guard_decision(
                    "I already called the same tool with the same arguments and did not make progress. Please clarify what to inspect next.",
                );
                break;
            }

            if consecutive_same_failure_retries >= 1 {
                decision = tool_loop_guard_decision(
                    "The same tool call is failing repeatedly. Please clarify or change the requested input.",
                );
                break;
            }

            consecutive_same_failure_retries += 1;
        } else {
            consecutive_same_failure_retries = 0;
        }

        let executor = ToolExecutor::new();
        let executed_tool_result = executor
            .execute_with_attachments(runtime, &invocation, &intake.attachments)
            .await;
        tool_result = Some(executed_tool_result.clone());
        last_tool_signature = Some(tool_signature);
        last_tool_result = Some(executed_tool_result.clone());

        let tool_result_message = build_session_message_with_data(
            &session.id,
            None,
            "tool_result_message",
            SessionMessageRole::System,
            executed_tool_result
                .rendered_summary
                .clone()
                .or_else(|| executed_tool_result.error_message.clone())
                .unwrap_or_else(|| format!("tool executed: {}", executed_tool_result.tool_name)),
            serde_json::json!({
                "tool_name": invocation.tool_name,
                "arguments": invocation.arguments,
            })
            .to_string(),
        );
        insert_session_message(&conn, &tool_result_message)?;

        if !decision.should_continue_planning || !executed_tool_result.should_continue_planning {
            decision = tool_loop_guard_decision(
                "Tool execution ended without a terminal planner action. Please clarify the next step.",
            );
            break;
        }

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
        SessionActionType::SkillCall => "system_message",
        SessionActionType::CreateRun => "system_message",
        SessionActionType::Stop => "assistant_message",
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
    let current_effective_title = effective_session_title(&session);
    let candidate_title = if should_assign_initial_session_title(&session.title, user_message_count) {
        derive_initial_session_title(&intake.user_message)
    } else {
        let recent_user_messages = list_session_messages_for_session(&conn, &session.id)?
            .into_iter()
            .filter(|message| message.role == SessionMessageRole::User)
            .map(|message| message.content)
            .collect::<Vec<_>>();
        derive_refreshed_session_title(&recent_user_messages)
    };
    if session.manual_title.is_none()
        && (should_assign_initial_session_title(&session.title, user_message_count)
            || (should_attempt_session_title_refresh(user_message_count)
                && should_replace_session_title(&current_effective_title, &candidate_title)))
    {
        session.title = candidate_title;
    }
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

pub async fn decide_and_record_intake_streaming<F>(
    runtime: &AppRuntime,
    intake: SessionIntake,
    provider_config: Option<LlmProviderConfig>,
    mut on_reply_chunk: F,
) -> Result<IntakeDecisionOutcome, RuntimeError>
where
    F: FnMut(&str),
{
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let mut session = get_session_by_id(&conn, &intake.session_id)?.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("session not found: {}", intake.session_id),
        )
    })?;

    let user_session_message = build_session_message_with_data(
        &session.id,
        None,
        "user_message",
        SessionMessageRole::User,
        intake.user_message.clone(),
        serde_json::json!({
            "attachments": intake.attachments,
        })
        .to_string(),
    );
    insert_session_message(&conn, &user_session_message)?;

    let recent_messages = list_session_messages_for_session(&conn, &session.id)?;
    let user_message_count = recent_messages
        .iter()
        .filter(|message| message.role == SessionMessageRole::User)
        .count();
    let input = SessionAgentInput {
        session: session.clone(),
        recent_messages,
        intake: intake.clone(),
    };

    let mut decision = decide_with_provider_streaming(provider_config.clone(), input, &mut on_reply_chunk).await?;
    let mut tool_result = None;
    let mut last_tool_signature: Option<String> = None;
    let mut last_tool_result: Option<ToolExecutionResult> = None;
    let mut consecutive_same_failure_retries = 0usize;

    while decision.action_type == SessionActionType::ToolCall {
        let invocation = decision.tool_invocation.clone().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "tool_call decision missing tool_invocation",
            )
        })?;
        let tool_signature = serde_json::json!({
            "tool_name": invocation.tool_name,
            "arguments": invocation.arguments,
        })
        .to_string();

        if last_tool_signature.as_deref() == Some(tool_signature.as_str()) {
            if last_tool_result.as_ref().is_some_and(|result| result.ok) {
                decision = tool_loop_guard_decision(
                    "I already called the same tool with the same arguments and did not make progress. Please clarify what to inspect next.",
                );
                break;
            }

            if consecutive_same_failure_retries >= 1 {
                decision = tool_loop_guard_decision(
                    "The same tool call is failing repeatedly. Please clarify or change the requested input.",
                );
                break;
            }

            consecutive_same_failure_retries += 1;
        } else {
            consecutive_same_failure_retries = 0;
        }

        let executor = ToolExecutor::new();
        let executed_tool_result = executor
            .execute_with_attachments(runtime, &invocation, &intake.attachments)
            .await;
        tool_result = Some(executed_tool_result.clone());
        last_tool_signature = Some(tool_signature);
        last_tool_result = Some(executed_tool_result.clone());

        let tool_result_message = build_session_message_with_data(
            &session.id,
            None,
            "tool_result_message",
            SessionMessageRole::System,
            executed_tool_result
                .rendered_summary
                .clone()
                .or_else(|| executed_tool_result.error_message.clone())
                .unwrap_or_else(|| format!("tool executed: {}", executed_tool_result.tool_name)),
            serde_json::json!({
                "tool_name": invocation.tool_name,
                "arguments": invocation.arguments,
            })
            .to_string(),
        );
        insert_session_message(&conn, &tool_result_message)?;

        if !decision.should_continue_planning || !executed_tool_result.should_continue_planning {
            decision = tool_loop_guard_decision(
                "Tool execution ended without a terminal planner action. Please clarify the next step.",
            );
            break;
        }

        let follow_up_input = SessionAgentInput {
            session: session.clone(),
            recent_messages: list_session_messages_for_session(&conn, &session.id)?,
            intake: intake.clone(),
        };
        decision = decide_with_provider_streaming(
            provider_config.clone(),
            follow_up_input,
            &mut on_reply_chunk,
        )
        .await?;
    }

    let assistant_message_type = match decision.action_type {
        SessionActionType::DirectReply => "assistant_message",
        SessionActionType::RequestClarification => "clarification_message",
        SessionActionType::ToolCall => "system_message",
        SessionActionType::SkillCall => "system_message",
        SessionActionType::CreateRun => "system_message",
        SessionActionType::Stop => "assistant_message",
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
    let current_effective_title = effective_session_title(&session);
    let candidate_title = if should_assign_initial_session_title(&session.title, user_message_count) {
        derive_initial_session_title(&intake.user_message)
    } else {
        let recent_user_messages = list_session_messages_for_session(&conn, &session.id)?
            .into_iter()
            .filter(|message| message.role == SessionMessageRole::User)
            .map(|message| message.content)
            .collect::<Vec<_>>();
        derive_refreshed_session_title(&recent_user_messages)
    };
    if session.manual_title.is_none()
        && (should_assign_initial_session_title(&session.title, user_message_count)
            || (should_attempt_session_title_refresh(user_message_count)
                && should_replace_session_title(&current_effective_title, &candidate_title)))
    {
        session.title = candidate_title;
    }
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
