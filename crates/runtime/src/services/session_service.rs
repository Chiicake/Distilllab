use crate::app::AppRuntime;
use crate::contracts::{
    LlmSessionDebugRequest, RunProgressUpdate, RunResultContext, SessionIntakePreview,
    SessionMessageExecutionResult, SessionMessageRequest,
};
use crate::flows::build_import_and_distill_handoff_preview;
use crate::services::distill_run_executor::{
    create_and_execute_from_decision, create_and_execute_from_decision_with_progress,
};
use crate::services::session_intake_coordinator::{
    decide_and_record_intake, decide_and_record_intake_streaming,
};
use agent::{
    run_run_completion_summarizer, BasicSessionAgent, LlmProviderConfig, LlmSessionAgent,
    RunCompletionResultContext, RunCompletionSummaryInput, SessionAgent, SessionAgentDecision,
    SessionAgentInput, SessionIntent,
};
use chrono::Utc;
use memory::db::open_database;
use memory::migrations::run_migrations;
use memory::session_message_store::{
    delete_session_messages_for_session, insert_session_message,
    list_session_messages_for_session, update_session_message_run_and_content,
};
use memory::session_store::{
    delete_session, get_session_by_id, insert_session, list_sessions as memory_list_sessions,
    update_session,
};
use schema::{Session, SessionIntake, SessionMessage, SessionMessageRole, SessionStatus};
use uuid::Uuid;

type RuntimeError = Box<dyn std::error::Error + Send + Sync>;

pub(crate) fn derive_initial_session_title(first_user_message: &str) -> String {
    let cleaned = first_user_message
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .trim_matches(|ch: char| matches!(ch, '"' | '\''))
        .to_string();

    if cleaned.is_empty() {
        return "Untitled Session".to_string();
    }

    cleaned.chars().take(80).collect()
}

pub(crate) fn should_refresh_session_title(new_messages_since_last_refresh: usize) -> bool {
    new_messages_since_last_refresh > 0 && new_messages_since_last_refresh % 6 == 0
}

pub(crate) fn should_assign_initial_session_title(current_title: &str, user_message_count: usize) -> bool {
    user_message_count == 1 && is_generic_session_title(current_title)
}

pub(crate) fn should_attempt_session_title_refresh(user_message_count: usize) -> bool {
    user_message_count > 1 && should_refresh_session_title(user_message_count - 1)
}

pub(crate) fn should_replace_session_title(current_title: &str, candidate_title: &str) -> bool {
    if is_generic_session_title(current_title) {
        return true;
    }

    let current = current_title.trim();
    let candidate = candidate_title.trim();
    !candidate.is_empty() && candidate != current && candidate.len() > current.len()
}

pub(crate) fn derive_refreshed_session_title(recent_user_messages: &[String]) -> String {
    let recent_window = recent_user_messages
        .iter()
        .rev()
        .take(6)
        .cloned()
        .collect::<Vec<_>>();

    let meaningful = recent_window
        .iter()
        .filter(|message| !message.trim().is_empty())
        .filter(|message| !is_placeholder_title_candidate(message))
        .max_by_key(|message| message.len())
        .cloned();

    meaningful
        .map(|message| derive_initial_session_title(&message))
        .unwrap_or_else(|| "Untitled Session".to_string())
}

fn is_generic_session_title(title: &str) -> bool {
    let normalized = title.trim().to_ascii_lowercase();
    normalized.is_empty() || normalized == "demo session" || normalized == "untitled session"
}

fn is_placeholder_title_candidate(message: &str) -> bool {
    let normalized = message.trim().to_ascii_lowercase();
    normalized.is_empty()
        || normalized == "short placeholder"
        || normalized == "placeholder"
        || normalized == "test"
}

pub(crate) fn effective_session_title(session: &Session) -> String {
    session
        .manual_title
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| session.title.clone())
}

pub fn rename_session(
    runtime: &AppRuntime,
    session_id: &str,
    manual_title: Option<String>,
) -> Result<Session, RuntimeError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let mut session = get_session_by_id(&conn, session_id)?.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("session not found: {session_id}"),
        )
    })?;

    session.manual_title = manual_title
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    session.updated_at = Utc::now().to_string();
    update_session(&conn, &session)?;

    Ok(session)
}

pub fn pin_session(
    runtime: &AppRuntime,
    session_id: &str,
    pinned: bool,
) -> Result<Session, RuntimeError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let mut session = get_session_by_id(&conn, session_id)?.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("session not found: {session_id}"),
        )
    })?;

    session.pinned = pinned;
    session.updated_at = Utc::now().to_string();
    update_session(&conn, &session)?;

    Ok(session)
}

pub fn delete_session_and_related(runtime: &AppRuntime, session_id: &str) -> Result<(), RuntimeError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    delete_session_messages_for_session(&conn, session_id)?;
    delete_session(&conn, session_id)?;

    Ok(())
}

fn build_demo_agent_session(session_id: &str, title: &str, summary: &str) -> Session {
    let now = Utc::now().to_string();

    Session {
        id: session_id.to_string(),
        title: title.to_string(),
        manual_title: None,
        pinned: false,
        status: SessionStatus::Active,
        current_intent: "idle".to_string(),
        current_object_type: "none".to_string(),
        current_object_id: "none".to_string(),
        summary: summary.to_string(),
        started_at: now.clone(),
        updated_at: now.clone(),
        last_user_message_at: now.clone(),
        last_run_at: now.clone(),
        last_compacted_at: now,
        metadata_json: "{}".to_string(),
    }
}

fn normalize_optional_api_key(api_key: Option<String>) -> Option<String> {
    api_key.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn indent_block(text: &str) -> String {
    text.lines()
        .map(|line| format!("  {}", line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn run_progress_status_text(update: &RunProgressUpdate) -> String {
    let percent = update
        .progress_percent
        .map(|value| format!("{}%", value))
        .unwrap_or_else(|| "n/a".to_string());
    let step_key = update.step_key.as_deref().unwrap_or("run");
    let detail = update.detail_text.as_deref().unwrap_or("");

    match update.phase {
        crate::contracts::RunProgressPhase::Created => {
            format!("run created: {} ({})", update.run_id, update.run_type)
        }
        crate::contracts::RunProgressPhase::StateChanged => format!(
            "run state: {} {} ({}){}",
            update.run_id,
            update.run_state,
            percent,
            if detail.is_empty() {
                "".to_string()
            } else {
                format!(" - {}", detail)
            }
        ),
        crate::contracts::RunProgressPhase::StepStarted => format!(
            "run step started: {} [{}] ({}){}",
            update.run_id,
            step_key,
            percent,
            if detail.is_empty() {
                "".to_string()
            } else {
                format!(" - {}", detail)
            }
        ),
        crate::contracts::RunProgressPhase::StepFinished => format!(
            "run step finished: {} [{}] ({}){}",
            update.run_id,
            step_key,
            percent,
            if detail.is_empty() {
                "".to_string()
            } else {
                format!(" - {}", detail)
            }
        ),
    }
}

fn record_run_progress_message(
    conn: &rusqlite::Connection,
    session_id: &str,
    update: &RunProgressUpdate,
) -> Result<(), RuntimeError> {
    let status_text = run_progress_status_text(update);
    let mut history = Vec::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT data_json FROM session_messages WHERE session_id = ?1 AND run_id = ?2 AND message_type = 'run_progress_message' ORDER BY created_at ASC, id ASC",
            )
            .map_err(|error| Box::new(error) as RuntimeError)?;

        let rows = stmt
            .query_map([session_id, update.run_id.as_str()], |row| row.get::<_, String>(0))
            .map_err(|error| Box::new(error) as RuntimeError)?;

        for row in rows {
            let raw = row.map_err(|error| Box::new(error) as RuntimeError)?;
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw)
                && let Some(existing_update) = value.get("runProgress")
            {
                history.push(existing_update.clone());
            }
        }
    }

    history.push(
        serde_json::to_value(update)
            .map_err(|error| Box::new(error) as RuntimeError)?,
    );

    let data_json = serde_json::json!({
        "statusText": status_text,
        "runProgress": update,
        "runProgressHistory": history,
    })
    .to_string();

    insert_session_message(
        conn,
        &SessionMessage {
            id: format!("message-{}", Uuid::new_v4()),
            session_id: session_id.to_string(),
            run_id: Some(update.run_id.clone()),
            message_type: "run_progress_message".to_string(),
            role: SessionMessageRole::System,
            content: status_text,
            data_json,
            created_at: Utc::now().to_string(),
        },
    )
    .map_err(|error| Box::new(error) as RuntimeError)
}

fn record_run_progress_message_for_runtime(
    runtime: &AppRuntime,
    session_id: &str,
    update: &RunProgressUpdate,
) -> Result<(), RuntimeError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;
    record_run_progress_message(&conn, session_id, update)
}

fn build_run_result_context(
    run: &schema::Run,
    output: &crate::contracts::RunExecutionOutput,
    assets: &[schema::Asset],
) -> RunResultContext {
    RunResultContext {
        run_id: run.id.clone(),
        run_type: run.run_type.as_str().to_string(),
        status: run.status.as_str().to_string(),
        asset_count: output.asset_ids.len(),
        work_item_count: output.work_item_ids.len(),
        primary_asset_title: output.primary_asset_id.as_ref().and_then(|primary_id| {
            assets
                .iter()
                .find(|asset| &asset.id == primary_id)
                .map(|asset| asset.title.clone())
        }),
        asset_summaries: assets.iter().map(|asset| asset.summary.clone()).collect(),
        execution_summary: output.execution_summary.clone(),
    }
}

async fn summarize_run_completion(
    client: &reqwest::Client,
    provider_config: &LlmProviderConfig,
    session_id: &str,
    user_message: &str,
    run_result: RunResultContext,
) -> Result<agent::RunCompletionSummaryOutput, RuntimeError> {
    run_run_completion_summarizer(
        client,
        provider_config,
        &RunCompletionSummaryInput {
            session_id: session_id.to_string(),
            user_message: user_message.to_string(),
            run_result: RunCompletionResultContext {
                run_id: run_result.run_id,
                run_type: run_result.run_type,
                status: run_result.status,
                asset_count: run_result.asset_count,
                work_item_count: run_result.work_item_count,
                primary_asset_title: run_result.primary_asset_title,
                asset_summaries: run_result.asset_summaries,
                execution_summary: run_result.execution_summary,
            },
        },
    )
    .await
    .map_err(RuntimeError::from)
}

async fn append_run_completion_summary_message(
    runtime: &AppRuntime,
    provider_config: &LlmProviderConfig,
    session_id: &str,
    user_message: &str,
    execution_outcome: &crate::services::distill_run_executor::DistillRunExecutionOutcome,
) -> Result<Option<String>, RuntimeError> {
    let Some(output) = &execution_outcome.output else {
        return Ok(None);
    };

    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let assets = memory::asset_store::list_assets(&conn)?
        .into_iter()
        .filter(|asset| output.asset_ids.contains(&asset.id))
        .collect::<Vec<_>>();

    let run_result_context = build_run_result_context(&execution_outcome.run, output, &assets);
    let llm_client = reqwest::Client::new();
    let completion_summary = summarize_run_completion(
        &llm_client,
        provider_config,
        session_id,
        user_message,
        run_result_context,
    )
    .await?;

    let content = completion_summary.reply_text;
    insert_session_message(
        &conn,
        &SessionMessage {
            id: format!("message-{}", Uuid::new_v4()),
            session_id: session_id.to_string(),
            run_id: Some(execution_outcome.run.id.clone()),
            message_type: "assistant_message".to_string(),
            role: SessionMessageRole::Assistant,
            content: content.clone(),
            data_json: serde_json::json!({
                "sessionSummary": completion_summary.session_summary,
                "runId": execution_outcome.run.id,
                "runOutput": output,
            })
            .to_string(),
            created_at: Utc::now().to_string(),
        },
    )?;

    Ok(Some(content))
}

pub(crate) fn format_session_messages_for_debug(messages: &[SessionMessage]) -> String {
    if messages.is_empty() {
        return "no session messages found".to_string();
    }

    messages
        .iter()
        .map(|message| {
            if message.message_type == "tool_result_message" {
                let tool_header = serde_json::from_str::<serde_json::Value>(&message.data_json)
                    .ok()
                    .and_then(|value| {
                        let tool_name = value.get("tool_name").and_then(|v| v.as_str())?;
                        let arguments = value.get("arguments")?;
                        Some(format!("[Tool] {}({})", tool_name, arguments))
                    })
                    .unwrap_or_else(|| "[Tool] unknown()".to_string());

                format!("{}\n{}", tool_header, indent_block(&message.content))
            } else if message.message_type == "run_progress_message" {
                let run_header = serde_json::from_str::<serde_json::Value>(&message.data_json)
                    .ok()
                    .and_then(|value| {
                        let run_progress = value.get("runProgress")?;
                        let run_id = run_progress.get("runId").and_then(|v| v.as_str())?;
                        let step_key = run_progress.get("stepKey").and_then(|v| v.as_str());
                        let run_phase = run_progress
                            .get("phase")
                            .and_then(|v| v.as_str())
                            .unwrap_or("state_changed");

                        Some(match step_key {
                            Some(key) => {
                                format!("[Run] {} [{}] ({})", run_id, key, run_phase)
                            }
                            None => format!("[Run] {} ({})", run_id, run_phase),
                        })
                    })
                    .unwrap_or_else(|| "[Run] unknown".to_string());

                let run_body = format!("{}\n{}", message.content, message.data_json);
                format!("{}\n{}", run_header, indent_block(&run_body))
            } else {
                let role_header = match message.role {
                    schema::SessionMessageRole::User => "[User]",
                    schema::SessionMessageRole::Assistant => "[Assistant]",
                    schema::SessionMessageRole::System => "[System]",
                };

                format!("{}\n{}", role_header, indent_block(&message.content))
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

async fn decide_llm_session_message_with_provider_config(
    config: LlmProviderConfig,
    user_message: &str,
) -> Result<SessionAgentDecision, RuntimeError> {
    let session = build_demo_agent_session(
        "session-llm-demo",
        "LLM Demo Session",
        "Demo session for llm-backed session-agent decision",
    );

    let input = SessionAgentInput {
        session,
        recent_messages: vec![],
        intake: SessionIntake {
            session_id: "session-llm-demo".to_string(),
            user_message: user_message.to_string(),
            attachments: vec![],
            current_object_type: None,
            current_object_id: None,
        },
    };

    let session_agent = LlmSessionAgent::new(config);
    let decision = session_agent.decide(input).await?;

    Ok(decision)
}

async fn send_session_message_with_optional_provider_config(
    runtime: &AppRuntime,
    session_id: &str,
    user_message: &str,
    attachments: Vec<schema::AttachmentRef>,
    provider_config: Option<LlmProviderConfig>,
) -> Result<SessionAgentDecision, RuntimeError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let session = get_session_by_id(&conn, session_id)?.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("session not found: {session_id}"),
        )
    })?;
    let outcome = decide_and_record_intake(
        runtime,
        SessionIntake {
            session_id: session.id.clone(),
            user_message: user_message.to_string(),
            attachments: attachments.clone(),
            current_object_type: match session.current_object_type.as_str() {
                "none" => None,
                other => Some(other.to_string()),
            },
            current_object_id: match session.current_object_id.as_str() {
                "none" => None,
                other => Some(other.to_string()),
            },
        },
        provider_config.clone(),
    )
    .await?;

    let decision = outcome.decision;
    let assistant_message_id = outcome.assistant_message_id;
    let coordinator_run_input = outcome.run_input;

    let execution_outcome = if decision.action_type == agent::SessionActionType::CreateRun {
        let run_input = coordinator_run_input.clone().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "missing run_input for create_run decision",
            )
        })?;

        Some(create_and_execute_from_decision(
            runtime,
            provider_config.as_ref(),
            &decision,
            run_input,
        )
        .await?)
    } else {
        None
    };
    let created_run_id = execution_outcome.as_ref().map(|outcome| outcome.run.id.clone());
    let materialize_result = execution_outcome
        .as_ref()
        .and_then(|outcome| outcome.materialize_result.clone());
    let completion_summary_text = match (&provider_config, &execution_outcome) {
        (Some(provider_config), Some(execution_outcome)) => {
            append_run_completion_summary_message(
                runtime,
                provider_config,
                &session.id,
                user_message,
                execution_outcome,
            )
            .await?
        }
        _ => None,
    };

    let final_assistant_content = match completion_summary_text {
        Some(summary) => summary,
        None => match &materialize_result {
        Some(result) => format!("{}\n\n{}", decision.reply_text, result.summary),
        None => decision.reply_text.clone(),
        },
    };
    update_session_message_run_and_content(
        &conn,
        &assistant_message_id,
        created_run_id.as_deref(),
        &final_assistant_content,
    )?;

    Ok(decision)
}

async fn send_session_message_with_optional_provider_config_and_result(
    runtime: &AppRuntime,
    session_id: &str,
    user_message: &str,
    attachments: Vec<schema::AttachmentRef>,
    provider_config: Option<LlmProviderConfig>,
) -> Result<SessionMessageExecutionResult, RuntimeError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let session = get_session_by_id(&conn, session_id)?.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("session not found: {session_id}"),
        )
    })?;
    let outcome = decide_and_record_intake(
        runtime,
        SessionIntake {
            session_id: session.id.clone(),
            user_message: user_message.to_string(),
            attachments: attachments.clone(),
            current_object_type: match session.current_object_type.as_str() {
                "none" => None,
                other => Some(other.to_string()),
            },
            current_object_id: match session.current_object_id.as_str() {
                "none" => None,
                other => Some(other.to_string()),
            },
        },
        provider_config.clone(),
    )
    .await?;

    let decision = outcome.decision;
    let assistant_message_id = outcome.assistant_message_id;
    let coordinator_run_input = outcome.run_input;

    let execution_outcome = if decision.action_type == agent::SessionActionType::CreateRun {
        let run_input = coordinator_run_input.clone().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "missing run_input for create_run decision",
            )
        })?;

        Some(create_and_execute_from_decision(
            runtime,
            provider_config.as_ref(),
            &decision,
            run_input,
        )
        .await?)
    } else {
        None
    };
    let created_run_id = execution_outcome.as_ref().map(|outcome| outcome.run.id.clone());
    let materialize_result = execution_outcome
        .as_ref()
        .and_then(|outcome| outcome.materialize_result.clone());
    let completion_summary_text = match (&provider_config, &execution_outcome) {
        (Some(provider_config), Some(execution_outcome)) => {
            append_run_completion_summary_message(
                runtime,
                provider_config,
                &session.id,
                user_message,
                execution_outcome,
            )
            .await?
        }
        _ => None,
    };

    let final_assistant_content = match completion_summary_text.clone() {
        Some(summary) => summary,
        None => match &materialize_result {
        Some(result) => format!("{}\n\n{}", decision.reply_text, result.summary),
        None => decision.reply_text.clone(),
        },
    };
    update_session_message_run_and_content(
        &conn,
        &assistant_message_id,
        created_run_id.as_deref(),
        &final_assistant_content,
    )?;

    let messages = list_session_messages_for_session(&conn, &session.id)?;

    Ok(SessionMessageExecutionResult {
        session_id: session.id,
        action_type: decision.action_type.as_str().to_string(),
        intent: decision.intent.as_str().to_string(),
        tool_name: outcome.tool_result.as_ref().map(|result| result.tool_name.clone()),
        tool_ok: outcome.tool_result.as_ref().map(|result| result.ok),
        tool_summary: outcome
            .tool_result
            .as_ref()
            .and_then(|result| result.rendered_summary.clone().or(result.error_message.clone())),
        assistant_text: final_assistant_content,
        timeline_text: format_session_messages_for_debug(&messages),
        created_run_id,
        run_status: execution_outcome
            .as_ref()
            .map(|outcome| outcome.run.status.as_str().to_string()),
    })
}

pub async fn send_session_message_with_config_and_result_streaming<F>(
    runtime: &AppRuntime,
    request: SessionMessageRequest,
    mut on_reply_chunk: F,
) -> Result<SessionMessageExecutionResult, RuntimeError>
where
    F: FnMut(&str),
{
    send_session_message_with_config_and_result_streaming_with_progress(
        runtime,
        request,
        |chunk| on_reply_chunk(chunk),
        |_| {},
    )
    .await
}

pub async fn send_session_message_with_config_and_result_streaming_with_progress<F, G>(
    runtime: &AppRuntime,
    request: SessionMessageRequest,
    mut on_reply_chunk: F,
    mut on_run_progress: G,
) -> Result<SessionMessageExecutionResult, RuntimeError>
where
    F: FnMut(&str),
    G: FnMut(RunProgressUpdate),
{
    let provider_config = LlmProviderConfig {
        provider_kind: request.provider_kind.clone(),
        base_url: request.base_url.clone(),
        model: request.model.clone(),
        api_key: normalize_optional_api_key(request.api_key.clone()),
    };

    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let session = get_session_by_id(&conn, &request.session_id)?.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("session not found: {}", request.session_id),
        )
    })?;
    let session_id_for_progress = session.id.clone();

    let outcome = decide_and_record_intake_streaming(
        runtime,
        SessionIntake {
            session_id: session.id.clone(),
            user_message: request.user_message,
            attachments: request.attachments,
            current_object_type: match session.current_object_type.as_str() {
                "none" => None,
                other => Some(other.to_string()),
            },
            current_object_id: match session.current_object_id.as_str() {
                "none" => None,
                other => Some(other.to_string()),
            },
        },
        Some(provider_config.clone()),
        |chunk| on_reply_chunk(chunk),
    )
    .await?;

    let decision = outcome.decision;
    let assistant_message_id = outcome.assistant_message_id;
    let coordinator_run_input = outcome.run_input;

    let execution_outcome = if decision.action_type == agent::SessionActionType::CreateRun {
        let run_input = coordinator_run_input.clone().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "missing run_input for create_run decision",
            )
        })?;

        Some(create_and_execute_from_decision_with_progress(
            runtime,
            Some(&provider_config),
            &decision,
            run_input,
            |update| {
                let _ = record_run_progress_message_for_runtime(runtime, &session_id_for_progress, &update);
                on_run_progress(update);
            },
        )
        .await?)
    } else {
        None
    };
    let created_run_id = execution_outcome.as_ref().map(|outcome| outcome.run.id.clone());
    let materialize_result = execution_outcome
        .as_ref()
        .and_then(|outcome| outcome.materialize_result.clone());

    let final_assistant_content = match &materialize_result {
        Some(result) => format!("{}\n\n{}", decision.reply_text, result.summary),
        None => decision.reply_text.clone(),
    };
    update_session_message_run_and_content(
        &conn,
        &assistant_message_id,
        created_run_id.as_deref(),
        &final_assistant_content,
    )?;

    let messages = list_session_messages_for_session(&conn, &session.id)?;

    Ok(SessionMessageExecutionResult {
        session_id: session.id,
        action_type: decision.action_type.as_str().to_string(),
        intent: decision.intent.as_str().to_string(),
        tool_name: outcome.tool_result.as_ref().map(|result| result.tool_name.clone()),
        tool_ok: outcome.tool_result.as_ref().map(|result| result.ok),
        tool_summary: outcome
            .tool_result
            .as_ref()
            .and_then(|result| result.rendered_summary.clone().or(result.error_message.clone())),
        assistant_text: final_assistant_content,
        timeline_text: format_session_messages_for_debug(&messages),
        created_run_id,
        run_status: execution_outcome
            .as_ref()
            .map(|outcome| outcome.run.status.as_str().to_string()),
    })
}

fn llm_provider_config_from_env() -> Result<Option<LlmProviderConfig>, RuntimeError> {
    let base_url = match std::env::var("DISTILLLAB_LLM_BASE_URL") {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(error) => return Err(Box::new(error)),
    };

    let model = match std::env::var("DISTILLLAB_LLM_MODEL") {
        Ok(value) => value,
        Err(error) => return Err(Box::new(error)),
    };

    let api_key = normalize_optional_api_key(std::env::var("DISTILLLAB_LLM_API_KEY").ok());

    Ok(Some(LlmProviderConfig {
        provider_kind: "openai_compatible".to_string(),
        base_url,
        model,
        api_key,
    }))
}

pub async fn decide_demo_session_message(
    _runtime: &AppRuntime,
    user_message: &str,
) -> Result<SessionAgentDecision, RuntimeError> {
    let session = build_demo_agent_session(
        "session-demo",
        "Demo Session",
        "Demo session for session-agent decision",
    );

    let input = SessionAgentInput {
        session,
        recent_messages: vec![],
        intake: SessionIntake {
            session_id: "session-demo".to_string(),
            user_message: user_message.to_string(),
            attachments: vec![],
            current_object_type: None,
            current_object_id: None,
        },
    };

    let session_agent = BasicSessionAgent;
    let decision = session_agent.decide(input).await?;

    Ok(decision)
}

pub async fn decide_llm_session_message(
    _runtime: &AppRuntime,
    user_message: &str,
) -> Result<SessionAgentDecision, RuntimeError> {
    let config = llm_provider_config_from_env()?.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "DISTILLLAB_LLM_BASE_URL is not configured",
        )
    })?;

    decide_llm_session_message_with_provider_config(config, user_message).await
}

pub async fn decide_llm_session_message_with_config(
    _runtime: &AppRuntime,
    request: LlmSessionDebugRequest,
) -> Result<SessionAgentDecision, RuntimeError> {
    let config = LlmProviderConfig {
        provider_kind: request.provider_kind,
        base_url: request.base_url,
        model: request.model,
        api_key: normalize_optional_api_key(request.api_key),
    };

    decide_llm_session_message_with_provider_config(config, &request.user_message).await
}

pub async fn send_session_message(
    runtime: &AppRuntime,
    session_id: &str,
    user_message: &str,
) -> Result<SessionAgentDecision, RuntimeError> {
    send_session_message_with_optional_provider_config(
        runtime,
        session_id,
        user_message,
        vec![],
        llm_provider_config_from_env()?,
    )
    .await
}

pub async fn send_session_message_with_config(
    runtime: &AppRuntime,
    request: SessionMessageRequest,
) -> Result<SessionAgentDecision, RuntimeError> {
    let provider_config = LlmProviderConfig {
        provider_kind: request.provider_kind,
        base_url: request.base_url,
        model: request.model,
        api_key: normalize_optional_api_key(request.api_key),
    };

    send_session_message_with_optional_provider_config(
        runtime,
        &request.session_id,
        &request.user_message,
        request.attachments,
        Some(provider_config),
    )
    .await
}

pub async fn send_session_message_with_config_and_result(
    runtime: &AppRuntime,
    request: SessionMessageRequest,
) -> Result<SessionMessageExecutionResult, RuntimeError> {
    let provider_config = LlmProviderConfig {
        provider_kind: request.provider_kind.clone(),
        base_url: request.base_url.clone(),
        model: request.model.clone(),
        api_key: normalize_optional_api_key(request.api_key.clone()),
    };

    send_session_message_with_optional_provider_config_and_result(
        runtime,
        &request.session_id,
        &request.user_message,
        request.attachments,
        Some(provider_config),
    )
    .await
}

pub async fn create_session_and_send_first_message_with_config(
    runtime: &AppRuntime,
    request: SessionMessageRequest,
) -> Result<Session, RuntimeError> {
    let session = create_session(runtime)?;
    let session_id = session.id.clone();

    let send_result = send_session_message_with_config(
        runtime,
        SessionMessageRequest {
            session_id: session_id.clone(),
            ..request
        },
    )
    .await;

    if let Err(error) = send_result {
        cleanup_failed_first_send(runtime, &session_id)?;
        return Err(error);
    }

    Ok(session)
}

pub async fn preview_session_intake(
    runtime: &AppRuntime,
    intake: SessionIntake,
) -> Result<SessionIntakePreview, RuntimeError> {
    preview_session_intake_with_agent(runtime, intake, PreviewAgent::Basic).await
}

pub async fn preview_session_intake_with_config(
    runtime: &AppRuntime,
    intake: SessionIntake,
    config: LlmProviderConfig,
) -> Result<SessionIntakePreview, RuntimeError> {
    preview_session_intake_with_agent(runtime, intake, PreviewAgent::Llm(config)).await
}

enum PreviewAgent {
    Basic,
    Llm(LlmProviderConfig),
}

async fn preview_session_intake_with_agent(
    runtime: &AppRuntime,
    intake: SessionIntake,
    preview_agent: PreviewAgent,
) -> Result<SessionIntakePreview, RuntimeError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let session = get_session_by_id(&conn, &intake.session_id)?.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("session not found: {}", intake.session_id),
        )
    })?;

    let recent_messages = list_session_messages_for_session(&conn, &session.id)?;
    let input = SessionAgentInput {
        session,
        recent_messages,
        intake,
    };

    let attachment_count = input.intake.attachments.len();

    let decision = match preview_agent {
        PreviewAgent::Basic => {
            let session_agent = BasicSessionAgent;
            session_agent.decide(input).await?
        }
        PreviewAgent::Llm(config) => {
            let session_agent = LlmSessionAgent::new(config);
            session_agent.decide(input).await?
        }
    };

    let run_handoff_preview = if decision.intent == SessionIntent::DistillMaterial
        && decision.suggested_run_type.as_deref() == Some("import_and_distill")
    {
        let mut preview = build_import_and_distill_handoff_preview(
            decision.primary_object_type.clone().or(Some("material".to_string())),
            decision.primary_object_id.clone(),
        );

        if attachment_count > 0 {
            let count = attachment_count;
            preview.summary = format!(
                "Previewing the import-and-distill workflow for this work material with {} attachment{}.",
                count,
                if count == 1 { "" } else { "s" }
            );
        }

        Some(preview)
    } else {
        None
    };

    Ok(SessionIntakePreview {
        decision,
        run_handoff_preview,
    })
}

pub fn create_demo_session(runtime: &AppRuntime) -> Result<Session, RuntimeError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let now = Utc::now().to_string();
    let session = Session {
        id: format!("session-{}", Uuid::new_v4()),
        title: "Demo Session".to_string(),
        manual_title: None,
        pinned: false,
        status: SessionStatus::Active,
        current_intent: "idle".to_string(),
        current_object_type: "none".to_string(),
        current_object_id: "none".to_string(),
        summary: "Demo session created".to_string(),
        started_at: now.clone(),
        updated_at: now.clone(),
        last_user_message_at: now.clone(),
        last_run_at: now.clone(),
        last_compacted_at: now,
        metadata_json: "{}".to_string(),
    };

    insert_session(&conn, &session)?;
    Ok(session)
}

pub fn create_session(runtime: &AppRuntime) -> Result<Session, RuntimeError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let now = Utc::now().to_string();
    let session = Session {
        id: format!("session-{}", Uuid::new_v4()),
        title: "Untitled Session".to_string(),
        manual_title: None,
        pinned: false,
        status: SessionStatus::Active,
        current_intent: "idle".to_string(),
        current_object_type: "none".to_string(),
        current_object_id: "none".to_string(),
        summary: "Session created".to_string(),
        started_at: now.clone(),
        updated_at: now.clone(),
        last_user_message_at: now.clone(),
        last_run_at: now.clone(),
        last_compacted_at: now,
        metadata_json: "{}".to_string(),
    };

    insert_session(&conn, &session)?;
    Ok(session)
}

fn cleanup_failed_first_send(runtime: &AppRuntime, session_id: &str) -> Result<(), RuntimeError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    delete_session_messages_for_session(&conn, session_id)?;
    delete_session(&conn, session_id)?;

    Ok(())
}

pub fn delete_failed_first_send_session(runtime: &AppRuntime, session_id: &str) -> Result<(), RuntimeError> {
    cleanup_failed_first_send(runtime, session_id)
}

pub fn list_sessions(runtime: &AppRuntime) -> Result<Vec<Session>, RuntimeError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let sessions = memory_list_sessions(&conn)?;
    Ok(sessions)
}

pub fn list_session_messages(
    runtime: &AppRuntime,
    session_id: &str,
) -> Result<Vec<SessionMessage>, RuntimeError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let messages = list_session_messages_for_session(&conn, session_id)?;
    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::{
        LlmSessionDebugRequest, SessionMessageRequest, create_demo_session,
        create_session, create_session_and_send_first_message_with_config,
        decide_demo_session_message, decide_llm_session_message,
        decide_llm_session_message_with_config, preview_session_intake,
        preview_session_intake_with_config, send_session_message,
    };
    use crate::app::AppRuntime;
    use agent::{LlmProviderConfig, SessionIntent};
    use memory::db::open_database;
    use memory::run_store::list_runs as list_persisted_runs;
    use memory::session_message_store::list_session_messages_for_session;
    use memory::session_store::get_session_by_id;
    use schema::SessionIntake;
    use std::sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use uuid::Uuid;

    #[test]
    fn derive_initial_session_title_uses_first_user_message_meaningfully() {
        let title = super::derive_initial_session_title(
            "Please help me design the session selector and title lifecycle UI flow",
        );

        assert!(title.contains("session selector") || title.contains("title lifecycle"));
        assert!(title.len() <= 80);
    }

    #[test]
    fn title_refresh_checkpoint_triggers_every_six_messages() {
        assert!(!super::should_refresh_session_title(0));
        assert!(!super::should_refresh_session_title(5));
        assert!(super::should_refresh_session_title(6));
        assert!(super::should_refresh_session_title(12));
    }

    #[test]
    fn title_refresh_attempt_only_triggers_on_exact_checkpoints() {
        assert!(!super::should_attempt_session_title_refresh(2));
        assert!(!super::should_attempt_session_title_refresh(6));
        assert!(super::should_attempt_session_title_refresh(7));
        assert!(!super::should_attempt_session_title_refresh(8));
        assert!(super::should_attempt_session_title_refresh(13));
    }

    #[test]
    fn refreshed_title_is_derived_from_recent_history_not_only_latest_prompt() {
        let title = super::derive_refreshed_session_title(&[
            "Short placeholder".to_string(),
            "Session selector refinement iteration 1".to_string(),
            "Session selector refinement iteration 2".to_string(),
            "Title lifecycle update policy".to_string(),
        ]);

        assert!(title.contains("Session selector") || title.contains("Title lifecycle"));
        assert!(!title.contains("Short placeholder"));
    }

    #[test]
    fn create_session_uses_normal_non_demo_defaults() {
        let runtime = AppRuntime::new(format!("test-create-session-{}.db", Uuid::new_v4()));

        let session = create_session(&runtime).expect("runtime should create a normal session");

        assert_eq!(session.status.as_str(), "active");
        assert_eq!(session.title, "Untitled Session");
        assert_eq!(session.summary, "Session created");
        assert_ne!(session.title, "Demo Session");
        assert_ne!(session.summary, "Demo session created");
    }

    #[test]
    fn refreshed_title_prefers_recent_window_over_older_long_prompt() {
        let title = super::derive_refreshed_session_title(&[
            "Very long early title candidate that should not dominate forever once the session topic clearly shifts to session selector work".to_string(),
            "placeholder".to_string(),
            "Attachment tools".to_string(),
            "Session selector polish".to_string(),
            "Session selector dropdown behavior".to_string(),
            "Title refresh checkpoint".to_string(),
            "Recent session selector context".to_string(),
        ]);

        assert!(title.contains("Session selector") || title.contains("Title refresh"));
        assert!(!title.contains("Very long early title candidate"));
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct TestLlmEnvGuard {
        previous_base_url: Option<String>,
        previous_model: Option<String>,
        previous_api_key: Option<String>,
    }

    impl TestLlmEnvGuard {
        fn set(base_url: String, model: &str, api_key: Option<&str>) -> Self {
            let previous_base_url = std::env::var("DISTILLLAB_LLM_BASE_URL").ok();
            let previous_model = std::env::var("DISTILLLAB_LLM_MODEL").ok();
            let previous_api_key = std::env::var("DISTILLLAB_LLM_API_KEY").ok();

            unsafe {
                std::env::set_var("DISTILLLAB_LLM_BASE_URL", base_url);
                std::env::set_var("DISTILLLAB_LLM_MODEL", model);
                match api_key {
                    Some(value) => std::env::set_var("DISTILLLAB_LLM_API_KEY", value),
                    None => std::env::remove_var("DISTILLLAB_LLM_API_KEY"),
                }
            }

            Self {
                previous_base_url,
                previous_model,
                previous_api_key,
            }
        }

        fn clear() -> Self {
            let previous_base_url = std::env::var("DISTILLLAB_LLM_BASE_URL").ok();
            let previous_model = std::env::var("DISTILLLAB_LLM_MODEL").ok();
            let previous_api_key = std::env::var("DISTILLLAB_LLM_API_KEY").ok();

            unsafe {
                std::env::remove_var("DISTILLLAB_LLM_BASE_URL");
                std::env::remove_var("DISTILLLAB_LLM_MODEL");
                std::env::remove_var("DISTILLLAB_LLM_API_KEY");
            }

            Self {
                previous_base_url,
                previous_model,
                previous_api_key,
            }
        }
    }

    impl Drop for TestLlmEnvGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.previous_base_url {
                    Some(value) => std::env::set_var("DISTILLLAB_LLM_BASE_URL", value),
                    None => std::env::remove_var("DISTILLLAB_LLM_BASE_URL"),
                }
                match &self.previous_model {
                    Some(value) => std::env::set_var("DISTILLLAB_LLM_MODEL", value),
                    None => std::env::remove_var("DISTILLLAB_LLM_MODEL"),
                }
                match &self.previous_api_key {
                    Some(value) => std::env::set_var("DISTILLLAB_LLM_API_KEY", value),
                    None => std::env::remove_var("DISTILLLAB_LLM_API_KEY"),
                }
            }
        }
    }

    fn mock_distill_response_for_request(request_text: &str, chunk_inputs: &[&str]) -> String {
        if request_text.contains("RunCompletionSummarizer") {
            return r#"{
                "choices": [
                    {
                        "message": {
                            "role": "assistant",
                            "content": "{\"reply_text\":\"The distill run completed and produced 2 insight assets. The main outputs focus on launch readiness and scope control. If you want, I can next turn these into a recap report or expand one asset in detail.\",\"session_summary\":\"Distill run completed with reusable insight assets.\"}"
                        }
                    }
                ]
            }"#
            .to_string();
        }

        if request_text.contains("AssetExtractionAgent") {
            return r#"{
                "choices": [
                    {
                        "message": {
                            "role": "assistant",
                            "content": "{\"assets\":[{\"title\":\"Prototype launch readiness\",\"summary\":\"The launch is gated by scope finalization and clear coordination before next week.\"},{\"title\":\"Prototype scope control\",\"summary\":\"Scope clarity is the key stabilizer for this delivery cycle.\"}]}"
                        }
                    }
                ]
            }"#
            .to_string();
        }

        if request_text.contains("ProjectResolutionAgent") {
            return r#"{
                "choices": [
                    {
                        "message": {
                            "role": "assistant",
                            "content": "{\"decision\":\"create_new_project\",\"title\":\"Prototype Program\",\"summary\":\"Prototype planning, scope, and delivery work.\",\"reasoning_summary\":\"The extracted work belongs to a distinct prototype-focused body of work.\"}"
                        }
                    }
                ]
            }"#
            .to_string();
        }

        if request_text.contains("WorkItemExtractionAgent") {
            return r#"{
                "choices": [
                    {
                        "message": {
                            "role": "assistant",
                            "content": "{\"work_items\":[{\"title\":\"Finalize prototype scope\",\"summary\":\"Scope must be finalized before distillation output is shared.\",\"work_item_type\":\"note\"}]}"
                        }
                    }
                ]
            }"#
            .to_string();
        }

        if request_text.contains("ChunkExtractionAgent") {
            let matched_chunk_input = chunk_inputs
                .iter()
                .find(|chunk_input| request_text.contains(**chunk_input))
                .copied()
                .unwrap_or(chunk_inputs.first().copied().unwrap_or(""));
            let encoded_chunk_input = serde_json::to_string(matched_chunk_input)
                .expect("chunk input should encode for mock response");
            let chunk_content = format!(
                "{{\"chunks\":[{{\"title\":\"Progress update\",\"summary\":\"A concrete work update was captured.\",\"content\":{}}}]}}",
                encoded_chunk_input
            );

            return serde_json::json!({
                "choices": [
                    {
                        "message": {
                            "role": "assistant",
                            "content": chunk_content,
                        }
                    }
                ]
            })
            .to_string();
        }

        r#"{
            "choices": [
                {
                    "message": {
                        "role": "assistant",
                        "content": "{\"intent\":\"distill_material\",\"action_type\":\"create_run\",\"reply_text\":\"I will start a distill run for this work material.\",\"primary_object_type\":\"material\",\"primary_object_id\":null,\"suggested_run_type\":\"import_and_distill\",\"session_summary\":\"Preparing to distill work material\",\"tool_invocation\":null,\"skill_selection\":null,\"should_continue_planning\":true,\"failure_hint\":\"clarify_or_stop\"}"
                    }
                }
            ]
        }"#
        .to_string()
    }

    async fn spawn_mock_distill_llm_server(chunk_inputs: Vec<String>) -> LlmProviderConfig {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let address = listener
            .local_addr()
            .expect("listener should have local addr");

        tokio::spawn(async move {
            let chunk_input_refs = chunk_inputs.iter().map(String::as_str).collect::<Vec<_>>();
            loop {
                let (mut stream, _) = listener
                    .accept()
                    .await
                    .expect("server should accept connection");
                let mut buffer = [0_u8; 8192];
                let bytes_read = tokio::io::AsyncReadExt::read(&mut stream, &mut buffer)
                    .await
                    .expect("server should read request");
                let request_text = String::from_utf8_lossy(&buffer[..bytes_read]);
                let response_body = mock_distill_response_for_request(&request_text, &chunk_input_refs);

                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                tokio::io::AsyncWriteExt::write_all(&mut stream, response.as_bytes())
                    .await
                    .expect("server should write response");
            }
        });

        LlmProviderConfig {
            provider_kind: "openai_compatible".to_string(),
            base_url: format!("http://{}", address),
            model: "gpt-test".to_string(),
            api_key: None,
        }
    }

    async fn mock_distill_provider_config(user_message: &str) -> LlmProviderConfig {
        mock_distill_provider_config_for_chunks(&[user_message]).await
    }

    async fn mock_distill_provider_config_for_chunks(chunk_inputs: &[&str]) -> LlmProviderConfig {
        spawn_mock_distill_llm_server(
            chunk_inputs.iter().map(|value| value.to_string()).collect(),
        )
        .await
    }

    #[tokio::test]
    async fn runtime_can_get_structured_decision_from_session_agent() {
        let runtime = AppRuntime::new("/tmp/distilllab-runtime-test.db".to_string());

        let decision = decide_demo_session_message(&runtime, "Hello Distilllab")
            .await
            .expect("runtime should receive a session agent decision");

        assert_eq!(decision.intent, SessionIntent::GeneralReply);
        assert_eq!(
            decision.reply_text,
            "Hello! I am ready to help with your Distilllab session."
        );
    }

    #[tokio::test]
    async fn runtime_can_get_llm_backed_decision_from_session_agent() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
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
                            "content": "Hello from runtime llm"
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

        let _env_guard = TestLlmEnvGuard::set(format!("http://{}", address), "gpt-test", None);

        let runtime = AppRuntime::new("/tmp/distilllab-runtime-test-llm.db".to_string());

        let decision = decide_llm_session_message(&runtime, "Hello from runtime")
            .await
            .expect("runtime should receive an llm-backed session agent decision");

        assert_eq!(decision.intent, SessionIntent::GeneralReply);
        assert_eq!(decision.reply_text, "Hello from runtime llm");
    }

    #[tokio::test]
    async fn runtime_can_get_llm_backed_decision_from_explicit_config() {
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
                            "content": "Hello from explicit config"
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

        let runtime = AppRuntime::new("/tmp/distilllab-runtime-test-llm-explicit.db".to_string());
        let request = LlmSessionDebugRequest {
            provider_kind: "openai_compatible".to_string(),
            base_url: format!("http://{}", address),
            model: "gpt-test".to_string(),
            api_key: Some(String::new()),
            user_message: "Hello from runtime explicit config".to_string(),
        };

        let decision = decide_llm_session_message_with_config(&runtime, request)
            .await
            .expect("runtime should receive an llm-backed session agent decision");

        assert_eq!(decision.intent, SessionIntent::GeneralReply);
        assert_eq!(decision.reply_text, "Hello from explicit config");
    }

    #[tokio::test]
    async fn send_session_message_persists_user_and_assistant_messages() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();
        let runtime = AppRuntime::new("/tmp/distilllab-runtime-session-flow.db".to_string());
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let reply = send_session_message(&runtime, &session.id, "Hello Distilllab")
            .await
            .expect("runtime should send a session message");

        assert_eq!(reply.intent, SessionIntent::GeneralReply);

        let conn = open_database(&runtime.database_path).expect("database should open");
        let messages = list_session_messages_for_session(&conn, &session.id)
            .expect("session messages should load");

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role.as_str(), "user");
        assert_eq!(messages[0].content, "Hello Distilllab");
        assert_eq!(messages[1].role.as_str(), "assistant");
        assert_eq!(
            messages[1].content,
            "Hello! I am ready to help with your Distilllab session."
        );
    }

    #[tokio::test]
    async fn send_session_message_updates_session_intent_and_summary() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();
        let runtime = AppRuntime::new("/tmp/distilllab-runtime-session-update.db".to_string());
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let reply = send_session_message(&runtime, &session.id, "Hello again")
            .await
            .expect("runtime should send a session message");

        let conn = open_database(&runtime.database_path).expect("database should open");
        let updated_session = get_session_by_id(&conn, &session.id)
            .expect("query should succeed")
            .expect("session should exist");

        assert_eq!(updated_session.current_intent, reply.intent.as_str());
        assert_eq!(updated_session.summary, "General session assistance");
    }

    #[tokio::test]
    async fn send_session_message_assigns_initial_session_title_after_first_real_message() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();
        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-session-title-init-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        send_session_message(
            &runtime,
            &session.id,
            "Design the session selector and title lifecycle flow",
        )
        .await
        .expect("runtime should send a session message");

        let conn = open_database(&runtime.database_path).expect("database should open");
        let updated_session = get_session_by_id(&conn, &session.id)
            .expect("query should succeed")
            .expect("session should exist");

        assert_ne!(updated_session.title, "Demo Session");
        assert!(updated_session.title.contains("session selector") || updated_session.title.contains("title lifecycle"));
    }

    #[tokio::test]
    async fn send_session_message_refreshes_title_after_six_new_messages() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();
        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-session-title-refresh-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        send_session_message(
            &runtime,
            &session.id,
            "Short placeholder",
        )
        .await
        .expect("first message should succeed");

        for index in 0..6 {
            send_session_message(
                &runtime,
                &session.id,
                &format!("Session selector refinement iteration {}", index + 1),
            )
            .await
            .expect("follow-up message should succeed");
        }

        let conn = open_database(&runtime.database_path).expect("database should open");
        let updated_session = get_session_by_id(&conn, &session.id)
            .expect("query should succeed")
            .expect("session should exist");

        assert!(updated_session.title.contains("Session selector") || updated_session.title.contains("refinement"));
    }

    #[tokio::test]
    async fn send_session_message_persists_planner_primary_object_hints_back_to_session() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();
        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-session-primary-object-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");
        let provider_config = mock_distill_provider_config_for_chunks(&[
            "Please distill these work notes into Distilllab",
            "# Work notes\nshipped feature",
        ])
        .await;

        let _reply = super::send_session_message_with_optional_provider_config(
            &runtime,
            &session.id,
            "Please distill these work notes into Distilllab",
            vec![],
            Some(provider_config),
        )
        .await
        .expect("runtime should send a session message");

        let conn = open_database(&runtime.database_path).expect("database should open");
        let updated_session = get_session_by_id(&conn, &session.id)
            .expect("query should succeed")
            .expect("session should exist");

        assert_eq!(updated_session.current_object_type, "material");
    }

    #[tokio::test]
    async fn send_session_message_creates_run_for_import_and_distill_decision() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();
        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-session-create-run-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");
        let provider_config = mock_distill_provider_config_for_chunks(&[
            "Please distill these work notes into Distilllab",
            "# Work notes\nshipped feature",
        ])
        .await;

        let reply = super::send_session_message_with_optional_provider_config(
            &runtime,
            &session.id,
            "Please distill these work notes into Distilllab",
            vec![],
            Some(provider_config),
        )
        .await
        .expect("runtime should send a session message");

        assert_eq!(reply.intent, SessionIntent::DistillMaterial);

        let conn = open_database(&runtime.database_path).expect("database should open");
        let runs = list_persisted_runs(&conn).expect("runs should load");

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].run_type.as_str(), "import_and_distill");
        assert_eq!(runs[0].status.as_str(), "completed");
        assert_eq!(runs[0].primary_object_type, "asset");
    }

    #[tokio::test]
    async fn send_session_message_links_assistant_system_message_to_created_run() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();
        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-session-create-run-link-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");
        let provider_config = mock_distill_provider_config(
            "Please distill these work notes into Distilllab",
        )
        .await;

        super::send_session_message_with_optional_provider_config(
            &runtime,
            &session.id,
            "Please distill these work notes into Distilllab",
            vec![],
            Some(provider_config),
        )
        .await
        .expect("runtime should send a session message");

        let conn = open_database(&runtime.database_path).expect("database should open");
        let runs = list_persisted_runs(&conn).expect("runs should load");
        let messages = list_session_messages_for_session(&conn, &session.id)
            .expect("session messages should load");

        assert_eq!(runs.len(), 1);
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[1].message_type, "system_message");
        assert_eq!(messages[1].run_id.as_deref(), Some(runs[0].id.as_str()));
        assert_eq!(messages[2].message_type, "assistant_message");
        assert_eq!(messages[2].run_id.as_deref(), Some(runs[0].id.as_str()));
    }

    #[tokio::test]
    async fn send_session_message_with_config_and_result_returns_created_run_and_timeline_text() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();
        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-session-execution-result-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");
        let provider_config = mock_distill_provider_config(
            "Please distill these work notes into Distilllab",
        )
        .await;

        let execution = super::send_session_message_with_optional_provider_config_and_result(
            &runtime,
            &session.id,
            "Please distill these work notes into Distilllab",
            vec![],
            Some(provider_config),
        )
        .await
        .expect("execution should succeed");

        assert_eq!(execution.session_id, session.id);
        assert_eq!(execution.intent, "distill_material");
        assert_eq!(execution.action_type, "create_run");
        assert!(execution.created_run_id.is_some());
        assert!(execution.assistant_text.contains("The distill run completed and produced 2 insight assets"));
        assert!(execution.timeline_text.contains("[User]"));
        assert!(execution.timeline_text.contains("[Assistant]"));
    }

    #[tokio::test]
    async fn send_session_message_with_config_and_result_appends_run_completion_summary_message() {
        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-session-run-summary-msg-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let address = listener.local_addr().expect("listener should have local addr");

        tokio::spawn(async move {
            for response_body in [
                r#"{
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": "{\"intent\":\"distill_material\",\"action_type\":\"create_run\",\"reply_text\":\"I will start a distill run for this work material.\",\"primary_object_type\":\"material\",\"primary_object_id\":null,\"suggested_run_type\":\"import_and_distill\",\"session_summary\":\"Preparing to distill work material\",\"tool_invocation\":null,\"skill_selection\":null,\"should_continue_planning\":true,\"failure_hint\":\"clarify_or_stop\"}"
                            }
                        }
                    ]
                }"#,
                r#"{
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": "{\"chunks\":[{\"title\":\"Progress update\",\"summary\":\"A concrete work update was captured.\",\"content\":\"Please distill these work notes into Distilllab\"}]}"
                            }
                        }
                    ]
                }"#,
                r#"{
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": "{\"work_items\":[{\"title\":\"Finalize prototype scope\",\"summary\":\"Scope must be finalized before distillation output is shared.\",\"work_item_type\":\"note\"}]}"
                            }
                        }
                    ]
                }"#,
                r#"{
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": "{\"decision\":\"create_new_project\",\"title\":\"Prototype Program\",\"summary\":\"Prototype planning, scope, and delivery work.\",\"reasoning_summary\":\"The extracted work belongs to a distinct prototype-focused body of work.\"}"
                            }
                        }
                    ]
                }"#,
                r#"{
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": "{\"assets\":[{\"title\":\"Prototype launch readiness\",\"summary\":\"The launch is gated by scope finalization and clear coordination before next week.\"},{\"title\":\"Prototype scope control\",\"summary\":\"Scope clarity is the key stabilizer for this delivery cycle.\"}]}"
                            }
                        }
                    ]
                }"#,
                r#"{
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": "{\"reply_text\":\"The distill run completed and produced 2 insight assets. The main outputs focus on launch readiness and scope control. If you want, I can next turn these into a recap report or expand one asset in detail.\",\"session_summary\":\"Distill run completed with reusable insight assets.\"}"
                            }
                        }
                    ]
                }"#,
            ] {
                let (mut stream, _) = listener.accept().await.expect("server should accept connection");
                let mut buffer = [0_u8; 8192];
                let _ = tokio::io::AsyncReadExt::read(&mut stream, &mut buffer)
                    .await
                    .expect("server should read request");

                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                tokio::io::AsyncWriteExt::write_all(&mut stream, response.as_bytes())
                    .await
                    .expect("server should write response");
            }
        });

        let execution = super::send_session_message_with_optional_provider_config_and_result(
            &runtime,
            &session.id,
            "Please distill these work notes into Distilllab",
            vec![],
            Some(LlmProviderConfig {
                provider_kind: "openai_compatible".to_string(),
                base_url: format!("http://{}", address),
                model: "gpt-test".to_string(),
                api_key: None,
            }),
        )
        .await
        .expect("execution should succeed");

        assert!(execution.assistant_text.contains("The distill run completed and produced 2 insight assets"));
        assert!(execution.timeline_text.contains("The distill run completed and produced 2 insight assets"));
    }

    #[tokio::test]
    async fn send_session_message_executes_materialize_sources_and_completes_run() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();
        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-session-run-exec-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");
        let provider_config = mock_distill_provider_config(
            "Please distill these work notes into Distilllab",
        )
        .await;

        super::send_session_message_with_optional_provider_config(
            &runtime,
            &session.id,
            "Please distill these work notes into Distilllab",
            vec![],
            Some(provider_config),
        )
        .await
        .expect("runtime should send a session message");

        let conn = open_database(&runtime.database_path).expect("database should open");
        let runs = list_persisted_runs(&conn).expect("runs should load");
        let sources = memory::source_store::list_sources(&conn).expect("sources should load");

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].status.as_str(), "completed");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].run_id.as_deref(), Some(runs[0].id.as_str()));
        assert_eq!(sources[0].source_type.as_str(), "session");
    }

    #[tokio::test]
    async fn send_session_message_appends_materialize_summary_to_system_message() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();
        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-session-run-summary-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");
        let provider_config = mock_distill_provider_config(
            "Please distill these work notes into Distilllab",
        )
        .await;

        super::send_session_message_with_optional_provider_config(
            &runtime,
            &session.id,
            "Please distill these work notes into Distilllab",
            vec![],
            Some(provider_config),
        )
        .await
        .expect("runtime should send a session message");

        let conn = open_database(&runtime.database_path).expect("database should open");
        let messages = list_session_messages_for_session(&conn, &session.id)
            .expect("session messages should load");

        assert_eq!(messages.len(), 3);
        assert!(messages[1].content.contains("produced 2 insight assets"));
        assert!(messages[2].content.contains("produced 2 insight assets"));
    }

    #[tokio::test]
    async fn send_session_message_with_attachments_materializes_attachment_sources_for_run() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();
        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-session-attachment-run-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");
        let provider_config = mock_distill_provider_config(
            "Please distill these work notes into Distilllab",
        )
        .await;

        let attachment_path = format!(
            "/tmp/distilllab-runtime-session-attachment-{}.md",
            Uuid::new_v4()
        );
        std::fs::write(&attachment_path, "# Work notes\nshipped feature")
            .expect("attachment fixture should be written");

        let reply = super::send_session_message_with_optional_provider_config(
            &runtime,
            &session.id,
            "Please distill these work notes into Distilllab",
            vec![schema::AttachmentRef {
                attachment_id: "attachment-1".to_string(),
                kind: "file_path".to_string(),
                name: "notes.md".to_string(),
                mime_type: "text/markdown".to_string(),
                path_or_locator: attachment_path.clone(),
                size: 64,
                metadata_json: "{}".to_string(),
            }],
            Some(provider_config),
        )
        .await
        .expect("runtime should send attachment-aware session message");

        assert_eq!(reply.intent, SessionIntent::DistillMaterial);

        let conn = open_database(&runtime.database_path).expect("database should open");
        let sources = memory::source_store::list_sources(&conn).expect("sources should load");

        assert_eq!(sources.len(), 2);
        assert!(sources.iter().any(|source| source.source_type.as_str() == "document"));
        assert!(sources.iter().any(|source| source.source_type.as_str() == "session"));

        let _ = std::fs::remove_file(attachment_path);
    }

    #[tokio::test]
    async fn session_intake_coordinator_decides_and_records_messages_without_executing_run() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();
        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-intake-coordinator-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let outcome = crate::services::session_intake_coordinator::decide_and_record_intake(
            &runtime,
            SessionIntake {
                session_id: session.id.clone(),
                user_message: "Please distill these work notes into Distilllab".to_string(),
                attachments: vec![],
                current_object_type: None,
                current_object_id: None,
            },
            None,
        )
        .await
        .expect("intake coordinator should decide and record intake");

        assert_eq!(outcome.decision.intent, SessionIntent::DistillMaterial);
        assert!(outcome.run_input.is_some());
        assert!(outcome.created_run.is_none());

        let conn = open_database(&runtime.database_path).expect("database should open");
        let messages = list_session_messages_for_session(&conn, &session.id)
            .expect("session messages should load");
        let runs = list_persisted_runs(&conn).expect("runs should load");

        assert_eq!(messages.len(), 2);
        assert!(runs.is_empty());
    }

    #[tokio::test]
    async fn distill_run_executor_creates_and_executes_import_and_distill_run() {
        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-distill-executor-{}.db",
            Uuid::new_v4()
        ));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let address = listener
            .local_addr()
            .expect("listener should have local addr");

        tokio::spawn(async move {
            for response_body in [
                r#"{
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": "{\"chunks\":[{\"title\":\"Progress update\",\"summary\":\"A concrete work update was captured.\",\"content\":\"Please distill these work notes into Distilllab\"}]}"
                            }
                        }
                    ]
                }"#,
                r#"{
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": "{\"work_items\":[{\"title\":\"Finalize prototype scope\",\"summary\":\"Scope must be finalized before distillation output is shared.\",\"work_item_type\":\"note\"}]}"
                            }
                        }
                    ]
                }"#,
                r#"{
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": "{\"decision\":\"create_new_project\",\"title\":\"Prototype Program\",\"summary\":\"Prototype planning, scope, and delivery work.\",\"reasoning_summary\":\"The extracted work belongs to a distinct prototype-focused body of work.\"}"
                            }
                        }
                    ]
                }"#,
                r#"{
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": "{\"assets\":[{\"title\":\"Prototype launch readiness\",\"summary\":\"The launch is gated by scope finalization and clear coordination before next week.\"},{\"title\":\"Prototype scope control\",\"summary\":\"Scope clarity is the key stabilizer for this delivery cycle.\"}]}"
                            }
                        }
                    ]
                }"#,
                r#"{
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": "{\"reply_text\":\"The distill run completed and produced 2 insight assets. The main outputs focus on launch readiness and scope control. If you want, I can next turn these into a recap report or expand one asset in detail.\",\"session_summary\":\"Distill run completed with reusable insight assets.\"}"
                            }
                        }
                    ]
                }"#,
            ] {
            let (mut stream, _) = listener
                .accept()
                .await
                .expect("server should accept connection");
            let mut buffer = [0_u8; 8192];
            let _ = tokio::io::AsyncReadExt::read(&mut stream, &mut buffer)
                .await
                .expect("server should read request");

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            tokio::io::AsyncWriteExt::write_all(&mut stream, response.as_bytes())
                .await
                .expect("server should write response");
            }
        });

        let decision = agent::SessionAgentDecision {
            intent: SessionIntent::DistillMaterial,
            primary_object_type: Some("material".to_string()),
            primary_object_id: None,
            action_type: agent::SessionActionType::CreateRun,
            next_action: agent::SessionNextAction::CreateRun(agent::RunCreationRequest {
                run_type: "import_and_distill".to_string(),
                reasoning_summary: None,
            }),
            tool_invocation: None,
            skill_selection: None,
            run_creation: Some(agent::RunCreationRequest {
                run_type: "import_and_distill".to_string(),
                reasoning_summary: None,
            }),
            reply_text: "I will start a distill run for this work material.".to_string(),
            suggested_run_type: Some("import_and_distill".to_string()),
            session_summary: Some("Preparing to distill work material".to_string()),
            should_continue_planning: true,
            failure_hint: Some("clarify_or_stop".to_string()),
        };

        let run_input = crate::contracts::RunInput {
            session_id: "session-1".to_string(),
            trigger_message: "Please distill these work notes into Distilllab".to_string(),
            attachment_refs: vec![],
            current_object_type: Some("material".to_string()),
            current_object_id: None,
            decision_summary: decision.reply_text.clone(),
        };

        let outcome = crate::services::distill_run_executor::create_and_execute_from_decision(
            &runtime,
            Some(&LlmProviderConfig {
                provider_kind: "openai_compatible".to_string(),
                base_url: format!("http://{}", address),
                model: "gpt-test".to_string(),
                api_key: None,
            }),
            &decision,
            run_input,
        )
        .await
        .expect("executor should create and execute run");

        assert_eq!(outcome.run.run_type.as_str(), "import_and_distill");
        assert_eq!(outcome.run.status.as_str(), "completed");
        assert!(outcome.materialize_result.is_some());

        let conn = open_database(&runtime.database_path).expect("database should open");
        let sources = memory::source_store::list_sources(&conn).expect("sources should load");
        assert_eq!(sources.len(), 1);

        let chunks = memory::chunk_store::list_chunks_by_source(&conn, &sources[0].id)
            .expect("chunks should load");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].title, "Progress update");
        assert_eq!(chunks[0].summary, "A concrete work update was captured.");
        assert!(outcome.output.is_some());
        assert_eq!(outcome.run.primary_object_type, "asset");
        assert_eq!(
            outcome.output.as_ref().map(|value| value.execution_summary.as_str()),
            Some("materialized sources, created 1 chunks, extracted 1 work item drafts, resolved project Prototype Program, created 2 assets")
        );

        let projects = memory::project_store::list_projects(&conn).expect("projects should load");
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "Prototype Program");

        let assets = memory::asset_store::list_assets(&conn).expect("assets should load");
        assert_eq!(assets.len(), 2);
        assert_eq!(assets[0].project_id, projects[0].id);
        assert_eq!(assets[0].asset_type.as_str(), "insight");
        assert_eq!(outcome.run.primary_object_id, assets[0].id);
        assert_eq!(
            outcome.output.as_ref().map(|value| value.asset_ids.len()),
            Some(2)
        );
        assert_eq!(
            outcome.output.as_ref().and_then(|value| value.primary_asset_id.as_ref()),
            Some(&assets[0].id)
        );
        assert_eq!(
            outcome.output.as_ref().map(|value| value.execution_summary.as_str()),
            Some("materialized sources, created 1 chunks, extracted 1 work item drafts, resolved project Prototype Program, created 2 assets")
        );

    }

    #[tokio::test]
    async fn send_session_message_does_not_leave_misleading_handoff_message_when_execution_fails() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();
        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-send-failure-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let error = super::send_session_message_with_optional_provider_config(
            &runtime,
            &session.id,
            "Please distill these work notes into Distilllab",
            vec![],
            Some(LlmProviderConfig {
                provider_kind: "openai_compatible".to_string(),
                base_url: "http://127.0.0.1:1".to_string(),
                model: "gpt-test".to_string(),
                api_key: Some(String::new()),
            }),
        )
        .await
        .expect_err("send_session_message should fail");

        assert!(error.to_string().contains("error sending request"));

        let conn = open_database(&runtime.database_path).expect("database should open");
        let messages = list_session_messages_for_session(&conn, &session.id)
            .expect("session messages should load");

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].message_type, "user_message");
    }

    #[tokio::test]
    async fn create_session_and_send_first_message_with_config_rolls_back_failed_first_send() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();
        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-first-send-rollback-{}.db",
            Uuid::new_v4()
        ));

        let error = create_session_and_send_first_message_with_config(
            &runtime,
            SessionMessageRequest {
                session_id: String::new(),
                user_message: "Please distill these work notes into Distilllab".to_string(),
                attachments: vec![],
                provider_kind: "openai_compatible".to_string(),
                base_url: "http://127.0.0.1:1".to_string(),
                model: "gpt-test".to_string(),
                api_key: Some(String::new()),
            },
        )
        .await
        .expect_err("first send should fail");

        assert!(error.to_string().contains("error sending request"));

        let sessions = super::list_sessions(&runtime).expect("sessions should load");

        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn send_session_message_uses_llm_path_when_env_config_is_present() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
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
            let bytes_read = stream
                .read(&mut buffer)
                .await
                .expect("server should read request");
            let request_text = String::from_utf8_lossy(&buffer[..bytes_read]);

            assert!(request_text.contains("Earlier question"));
            assert!(request_text.contains("Hello with context"));

            let response_body = r#"{
                "choices": [
                    {
                        "message": {
                            "role": "assistant",
                            "content": "LLM reply with history"
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

        let _env_guard = TestLlmEnvGuard::set(format!("http://{}", address), "gpt-test", None);

        let runtime = AppRuntime::new(
            format!(
                "/tmp/distilllab-runtime-session-llm-flow-{}.db",
                Uuid::new_v4()
            ),
        );
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let conn = open_database(&runtime.database_path).expect("database should open");
        let earlier_message = schema::SessionMessage {
            id: "message-seeded-1".to_string(),
            session_id: session.id.clone(),
            run_id: None,
            message_type: "user_message".to_string(),
            role: schema::SessionMessageRole::User,
            content: "Earlier question".to_string(),
            data_json: "{}".to_string(),
            created_at: "2026-03-29T00:00:00Z".to_string(),
        };
        memory::session_message_store::insert_session_message(&conn, &earlier_message)
            .expect("seed message should insert");
        drop(conn);

        let reply = send_session_message(&runtime, &session.id, "Hello with context")
            .await
            .expect("runtime should send llm-backed session message");

        assert_eq!(reply.intent, SessionIntent::GeneralReply);
        assert_eq!(reply.reply_text, "LLM reply with history");
    }

    #[tokio::test]
    async fn send_session_message_with_config_uses_llm_without_env_variables() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();

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
            let bytes_read = stream
                .read(&mut buffer)
                .await
                .expect("server should read request");
            let request_text = String::from_utf8_lossy(&buffer[..bytes_read]);

            assert!(request_text.contains("Earlier explicit message"));
            assert!(request_text.contains("Current explicit message"));

            let response_body = r#"{
                "choices": [
                    {
                        "message": {
                            "role": "assistant",
                            "content": "LLM reply from explicit session config"
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

        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-session-explicit-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let conn = open_database(&runtime.database_path).expect("database should open");
        memory::session_message_store::insert_session_message(
            &conn,
            &schema::SessionMessage {
                id: "message-explicit-1".to_string(),
                session_id: session.id.clone(),
                run_id: None,
                message_type: "user_message".to_string(),
                role: schema::SessionMessageRole::User,
                content: "Earlier explicit message".to_string(),
                data_json: "{}".to_string(),
                created_at: "2026-03-29T00:00:00Z".to_string(),
            },
        )
        .expect("seed message should insert");
        drop(conn);

        let reply = super::send_session_message_with_config(
            &runtime,
            SessionMessageRequest {
                session_id: session.id.clone(),
                user_message: "Current explicit message".to_string(),
                attachments: vec![],
                provider_kind: "openai_compatible".to_string(),
                base_url: format!("http://{}", address),
                model: "gpt-test".to_string(),
                api_key: Some(String::new()),
            },
        )
        .await
        .expect("runtime should send llm-backed session message with explicit config");

        assert_eq!(reply.intent, SessionIntent::GeneralReply);
        assert_eq!(reply.reply_text, "LLM reply from explicit session config");
    }

    #[tokio::test]
    async fn send_session_message_with_config_executes_tool_call_before_follow_up_reply() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let address = listener
            .local_addr()
            .expect("listener should have local addr");

        tokio::spawn(async move {
            for request_index in 0..2 {
                let (mut stream, _) = listener
                    .accept()
                    .await
                    .expect("server should accept connection");
                let mut buffer = [0_u8; 8192];
                let bytes_read = stream
                    .read(&mut buffer)
                    .await
                    .expect("server should read request");
                let request_text = String::from_utf8_lossy(&buffer[..bytes_read]);

                let response_body = if request_index == 0 {
                    assert!(request_text.contains("Search memory for related notes before answering"));
                    r#"{
                        "choices": [
                            {
                                "message": {
                                    "role": "assistant",
                                    "content": "{\"intent\":\"general_reply\",\"action_type\":\"tool_call\",\"reply_text\":\"I will look up related notes before replying.\",\"primary_object_type\":null,\"primary_object_id\":null,\"suggested_run_type\":null,\"session_summary\":\"Preparing a memory lookup before answering\",\"tool_invocation\":{\"tool_name\":\"search_memory\",\"arguments\":{\"query\":\"related notes\"},\"reasoning_summary\":null,\"expected_follow_up\":null},\"should_continue_planning\":true,\"failure_hint\":\"reply_or_clarify\"}"
                                }
                            }
                        ]
                    }"#
                    .to_string()
                } else {
                    assert!(request_text.contains("Memory search is not yet implemented"));
                    r#"{
                        "choices": [
                            {
                                "message": {
                                    "role": "assistant",
                                    "content": "{\"intent\":\"general_reply\",\"action_type\":\"direct_reply\",\"reply_text\":\"I checked memory and there are no matching notes yet.\",\"primary_object_type\":null,\"primary_object_id\":null,\"suggested_run_type\":null,\"session_summary\":\"Reported memory lookup result\",\"tool_invocation\":null,\"should_continue_planning\":false,\"failure_hint\":null}"
                                }
                            }
                        ]
                    }"#
                    .to_string()
                };

                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );

                stream
                    .write_all(response.as_bytes())
                    .await
                    .expect("server should write response");
            }
        });

        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-session-tool-call-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let reply = super::send_session_message_with_config(
            &runtime,
            SessionMessageRequest {
                session_id: session.id.clone(),
                user_message: "Search memory for related notes before answering".to_string(),
                attachments: vec![],
                provider_kind: "openai_compatible".to_string(),
                base_url: format!("http://{}", address),
                model: "gpt-test".to_string(),
                api_key: Some(String::new()),
            },
        )
        .await
        .expect("runtime should execute tool call and follow up");

        assert_eq!(reply.intent, SessionIntent::GeneralReply);
        assert_eq!(reply.action_type, agent::SessionActionType::DirectReply);
        assert_eq!(reply.reply_text, "I checked memory and there are no matching notes yet.");

        let conn = open_database(&runtime.database_path).expect("database should open");
        let messages = list_session_messages_for_session(&conn, &session.id)
            .expect("session messages should load");
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[1].message_type, "tool_result_message");
        assert!(messages[1].content.contains("Memory search is not yet implemented"));
        assert_eq!(messages[2].message_type, "assistant_message");
        assert_eq!(messages[2].content, "I checked memory and there are no matching notes yet.");
    }

    #[tokio::test]
    async fn send_session_message_with_config_can_answer_attachment_content_questions_via_tool_call() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();

        let temp_dir = std::env::temp_dir().join(format!(
            "distilllab-runtime-attachment-question-{}",
            Uuid::new_v4()
        ));
        std::fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let attachment_path = temp_dir.join("notes.md");
        std::fs::write(
            &attachment_path,
            "Attachment heading\nThis attachment contains project notes about tool execution.",
        )
        .expect("attachment should be written");

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let address = listener
            .local_addr()
            .expect("listener should have local addr");
        tokio::spawn(async move {
            for request_index in 0..2 {
                let (mut stream, _) = listener
                    .accept()
                    .await
                    .expect("server should accept connection");
                let mut buffer = [0_u8; 8192];
                let bytes_read = stream
                    .read(&mut buffer)
                    .await
                    .expect("server should read request");
                let request_text = String::from_utf8_lossy(&buffer[..bytes_read]);

                let response_body = if request_index == 0 {
                    assert!(request_text.contains("current_intake_attachments:"));
                    assert!(request_text.contains("notes.md"));
                    r#"{
                        "choices": [
                            {
                                "message": {
                                    "role": "assistant",
                                    "content": "{\"intent\":\"general_reply\",\"action_type\":\"tool_call\",\"reply_text\":\"I will inspect the current attachment before answering.\",\"primary_object_type\":null,\"primary_object_id\":null,\"suggested_run_type\":null,\"session_summary\":\"Preparing to inspect the current attachment before answering\",\"tool_invocation\":{\"tool_name\":\"read_attachment_excerpt\",\"arguments\":{\"attachment_index\":0,\"max_chars\":400},\"reasoning_summary\":null,\"expected_follow_up\":null},\"skill_selection\":null,\"should_continue_planning\":true,\"failure_hint\":\"reply_or_clarify\"}"
                                }
                            }
                        ]
                    }"#
                    .to_string()
                } else {
                    assert!(request_text.contains("Attachment excerpt:"));
                    assert!(request_text.contains("project notes about tool execution"));
                    r#"{
                        "choices": [
                            {
                                "message": {
                                    "role": "assistant",
                                    "content": "{\"intent\":\"general_reply\",\"action_type\":\"direct_reply\",\"reply_text\":\"The attachment contains project notes about tool execution.\",\"primary_object_type\":null,\"primary_object_id\":null,\"suggested_run_type\":null,\"session_summary\":\"Answered using the attachment excerpt\",\"tool_invocation\":null,\"skill_selection\":null,\"should_continue_planning\":false,\"failure_hint\":null}"
                                }
                            }
                        ]
                    }"#
                    .to_string()
                };

                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );

                stream
                    .write_all(response.as_bytes())
                    .await
                    .expect("server should write response");
            }
        });

        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-attachment-send-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let attachment = schema::AttachmentRef {
            attachment_id: "attachment-1".to_string(),
            kind: "file_copy".to_string(),
            name: "notes.md".to_string(),
            mime_type: "text/markdown".to_string(),
            path_or_locator: attachment_path.to_string_lossy().to_string(),
            size: 128,
            metadata_json: "{}".to_string(),
        };

        let reply = super::send_session_message_with_config(
            &runtime,
            SessionMessageRequest {
                session_id: session.id.clone(),
                user_message: "What is inside attachment paths?".to_string(),
                attachments: vec![attachment],
                provider_kind: "openai_compatible".to_string(),
                base_url: format!("http://{}", address),
                model: "gpt-test".to_string(),
                api_key: Some(String::new()),
            },
        )
        .await
        .expect("runtime should answer attachment content question");

        assert_eq!(reply.reply_text, "The attachment contains project notes about tool execution.");

        let conn = open_database(&runtime.database_path).expect("database should open");
        let messages = list_session_messages_for_session(&conn, &session.id)
            .expect("session messages should load");
        assert!(messages.iter().any(|value| value.content.contains("Attachment excerpt:")));
        assert!(messages.iter().any(|value| value.content.contains("The attachment contains project notes about tool execution.")));

        let _ = std::fs::remove_file(&attachment_path);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn send_session_message_with_config_can_execute_two_sequential_tool_calls_before_final_reply() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();

        let temp_dir = std::env::temp_dir().join(format!(
            "distilllab-runtime-two-step-tool-loop-{}",
            Uuid::new_v4()
        ));
        std::fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let attachment_path = temp_dir.join("notes.md");
        std::fs::write(
            &attachment_path,
            "Attachment notes about session tool loops and follow-up reasoning.",
        )
        .expect("attachment should be written");

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let address = listener.local_addr().expect("listener should have local addr");

        tokio::spawn(async move {
            for request_index in 0..3 {
                let (mut stream, _) = listener.accept().await.expect("server should accept");
                let mut buffer = [0_u8; 8192];
                let bytes_read = stream.read(&mut buffer).await.expect("server should read request");
                let request_text = String::from_utf8_lossy(&buffer[..bytes_read]);

                let response_body = if request_index == 0 {
                    assert!(request_text.contains("https://platform.claude.com/docs/en/build-with-claude/overview"));
                    r#"{
                        "choices": [
                            {
                                "message": {
                                    "role": "assistant",
                                    "content": "{\"intent\":\"general_reply\",\"action_type\":\"tool_call\",\"reply_text\":\"I will fetch the webpage first.\",\"primary_object_type\":null,\"primary_object_id\":null,\"suggested_run_type\":null,\"session_summary\":\"Preparing web fetch\",\"tool_invocation\":{\"tool_name\":\"web_fetch\",\"arguments\":{\"url\":\"https://platform.claude.com/docs/en/build-with-claude/overview\"},\"reasoning_summary\":null,\"expected_follow_up\":null},\"skill_selection\":null,\"should_continue_planning\":true,\"failure_hint\":\"reply_or_clarify\"}"
                                }
                            }
                        ]
                    }"#
                    .to_string()
                } else if request_index == 1 {
                    assert!(request_text.contains("[Tool] web_fetch") || request_text.contains("Web content:"));
                    r#"{
                        "choices": [
                            {
                                "message": {
                                    "role": "assistant",
                                    "content": "{\"intent\":\"general_reply\",\"action_type\":\"tool_call\",\"reply_text\":\"I will read the attachment next.\",\"primary_object_type\":null,\"primary_object_id\":null,\"suggested_run_type\":null,\"session_summary\":\"Preparing attachment read\",\"tool_invocation\":{\"tool_name\":\"read_text\",\"arguments\":{\"attachment_index\":0},\"reasoning_summary\":null,\"expected_follow_up\":null},\"skill_selection\":null,\"should_continue_planning\":true,\"failure_hint\":\"reply_or_clarify\"}"
                                }
                            }
                        ]
                    }"#
                    .to_string()
                } else {
                    r#"{
                        "choices": [
                            {
                                "message": {
                                    "role": "assistant",
                                    "content": "{\"intent\":\"general_reply\",\"action_type\":\"direct_reply\",\"reply_text\":\"The website is an overview of Claude platform capabilities, and the attachment discusses session tool loops.\",\"primary_object_type\":null,\"primary_object_id\":null,\"suggested_run_type\":null,\"session_summary\":\"Summarized both sources\",\"tool_invocation\":null,\"skill_selection\":null,\"should_continue_planning\":false,\"failure_hint\":null}"
                                }
                            }
                        ]
                    }"#
                    .to_string()
                };

                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                stream.write_all(response.as_bytes()).await.expect("server should write");
            }
        });

        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-two-step-tool-loop-db-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let reply = super::send_session_message_with_config(
            &runtime,
            SessionMessageRequest {
                session_id: session.id.clone(),
                user_message: "帮我先总结一下网站：https://platform.claude.com/docs/en/build-with-claude/overview 以及附件的内容，不要进行蒸馏"
                    .to_string(),
                attachments: vec![schema::AttachmentRef {
                    attachment_id: "attachment-1".to_string(),
                    kind: "file_copy".to_string(),
                    name: "notes.md".to_string(),
                    mime_type: "text/markdown".to_string(),
                    path_or_locator: attachment_path.to_string_lossy().to_string(),
                    size: 128,
                    metadata_json: "{}".to_string(),
                }],
                provider_kind: "openai_compatible".to_string(),
                base_url: format!("http://{}", address),
                model: "gpt-test".to_string(),
                api_key: Some(String::new()),
            },
        )
        .await
        .expect("runtime should complete iterative tool loop");

        assert_eq!(reply.action_type, agent::SessionActionType::DirectReply);
        assert!(reply.reply_text.contains("website") || reply.reply_text.contains("attachment"));

        let conn = open_database(&runtime.database_path).expect("database should open");
        let messages = list_session_messages_for_session(&conn, &session.id)
            .expect("messages should load");
        let tool_messages = messages
            .iter()
            .filter(|message| message.message_type == "tool_result_message")
            .count();

        assert_eq!(tool_messages, 2);

        let _ = std::fs::remove_file(&attachment_path);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn send_session_message_with_config_stops_repeated_identical_tool_call_without_progress() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let address = listener.local_addr().expect("listener should have local addr");

        tokio::spawn(async move {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().await.expect("server should accept");
                let mut buffer = [0_u8; 8192];
                let _ = stream.read(&mut buffer).await.expect("server should read request");

                let response_body = r#"{
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": "{\"intent\":\"general_reply\",\"action_type\":\"tool_call\",\"reply_text\":\"I will fetch the webpage first.\",\"primary_object_type\":null,\"primary_object_id\":null,\"suggested_run_type\":null,\"session_summary\":\"Preparing web fetch\",\"tool_invocation\":{\"tool_name\":\"web_fetch\",\"arguments\":{\"url\":\"https://platform.claude.com/docs/en/build-with-claude/overview\"},\"reasoning_summary\":null,\"expected_follow_up\":null},\"skill_selection\":null,\"should_continue_planning\":true,\"failure_hint\":\"reply_or_clarify\"}"
                            }
                        }
                    ]
                }"#;

                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                stream.write_all(response.as_bytes()).await.expect("server should write");
            }
        });

        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-repeat-tool-loop-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let reply = super::send_session_message_with_config(
            &runtime,
            SessionMessageRequest {
                session_id: session.id.clone(),
                user_message: "帮我看这个网址：https://platform.claude.com/docs/en/build-with-claude/overview"
                    .to_string(),
                attachments: vec![],
                provider_kind: "openai_compatible".to_string(),
                base_url: format!("http://{}", address),
                model: "gpt-test".to_string(),
                api_key: Some(String::new()),
            },
        )
        .await
        .expect("runtime should terminate repeated identical tool loop safely");

        assert_eq!(reply.action_type, agent::SessionActionType::RequestClarification);
    }

    #[tokio::test]
    async fn send_session_message_with_config_stops_repeated_failing_tool_call() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let address = listener.local_addr().expect("listener should have local addr");

        let request_counter = Arc::new(AtomicUsize::new(0));
        let request_counter_for_server = request_counter.clone();
        tokio::spawn(async move {
            for _ in 0..3 {
                let (mut stream, _) = listener.accept().await.expect("server should accept");
                request_counter_for_server.fetch_add(1, Ordering::SeqCst);
                let mut buffer = [0_u8; 8192];
                let _ = stream.read(&mut buffer).await.expect("server should read request");

                let response_body = r#"{
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": "{\"intent\":\"general_reply\",\"action_type\":\"tool_call\",\"reply_text\":\"I will read the current attachment.\",\"primary_object_type\":null,\"primary_object_id\":null,\"suggested_run_type\":null,\"session_summary\":\"Preparing attachment read\",\"tool_invocation\":{\"tool_name\":\"read_text\",\"arguments\":{\"attachment_index\":0},\"reasoning_summary\":null,\"expected_follow_up\":null},\"skill_selection\":null,\"should_continue_planning\":true,\"failure_hint\":\"reply_or_clarify\"}"
                            }
                        }
                    ]
                }"#;

                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                stream.write_all(response.as_bytes()).await.expect("server should write");
            }
        });

        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-repeat-failing-tool-loop-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let reply = super::send_session_message_with_config(
            &runtime,
            SessionMessageRequest {
                session_id: session.id.clone(),
                user_message: "读取附件".to_string(),
                attachments: vec![],
                provider_kind: "openai_compatible".to_string(),
                base_url: format!("http://{}", address),
                model: "gpt-test".to_string(),
                api_key: Some(String::new()),
            },
        )
        .await
        .expect("runtime should terminate repeated failing tool loop safely");

        assert_eq!(reply.action_type, agent::SessionActionType::RequestClarification);
        assert_eq!(request_counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn send_session_message_with_config_does_not_finish_on_tool_call_when_tool_result_disables_planning() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let address = listener.local_addr().expect("listener should have local addr");

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("server should accept");
            let mut buffer = [0_u8; 8192];
            let _ = stream.read(&mut buffer).await.expect("server should read request");

            let response_body = r#"{
                "choices": [
                    {
                        "message": {
                            "role": "assistant",
                            "content": "{\"intent\":\"general_reply\",\"action_type\":\"tool_call\",\"reply_text\":\"I will fetch the webpage first.\",\"primary_object_type\":null,\"primary_object_id\":null,\"suggested_run_type\":null,\"session_summary\":\"Preparing web fetch\",\"tool_invocation\":{\"tool_name\":\"web_fetch\",\"arguments\":{\"url\":\"https://platform.claude.com/docs/en/build-with-claude/overview\"},\"reasoning_summary\":null,\"expected_follow_up\":null},\"skill_selection\":null,\"should_continue_planning\":true,\"failure_hint\":\"reply_or_clarify\"}"
                        }
                    }
                ]
            }"#;

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            stream.write_all(response.as_bytes()).await.expect("server should write");
        });

        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-nonterminal-stop-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let reply = super::send_session_message_with_config(
            &runtime,
            SessionMessageRequest {
                session_id: session.id.clone(),
                user_message: "帮我看这个网址：https://platform.claude.com/docs/en/build-with-claude/overview"
                    .to_string(),
                attachments: vec![],
                provider_kind: "openai_compatible".to_string(),
                base_url: format!("http://{}", address),
                model: "gpt-test".to_string(),
                api_key: Some(String::new()),
            },
        )
        .await;

        assert!(reply.is_err() || reply.as_ref().is_ok_and(|value| value.action_type != agent::SessionActionType::ToolCall));
    }

    #[tokio::test]
    async fn send_session_message_with_config_allows_one_retry_before_repeated_failure_clarification() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let address = listener.local_addr().expect("listener should have local addr");

        let request_counter = Arc::new(AtomicUsize::new(0));
        let request_counter_for_server = request_counter.clone();
        tokio::spawn(async move {
            for _ in 0..3 {
                let (mut stream, _) = listener.accept().await.expect("server should accept");
                request_counter_for_server.fetch_add(1, Ordering::SeqCst);
                let mut buffer = [0_u8; 8192];
                let _ = stream.read(&mut buffer).await.expect("server should read request");

                let response_body = r#"{
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": "{\"intent\":\"general_reply\",\"action_type\":\"tool_call\",\"reply_text\":\"I will read the current attachment.\",\"primary_object_type\":null,\"primary_object_id\":null,\"suggested_run_type\":null,\"session_summary\":\"Preparing attachment read\",\"tool_invocation\":{\"tool_name\":\"read_text\",\"arguments\":{\"attachment_index\":0},\"reasoning_summary\":null,\"expected_follow_up\":null},\"skill_selection\":null,\"should_continue_planning\":true,\"failure_hint\":\"reply_or_clarify\"}"
                            }
                        }
                    ]
                }"#;

                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                stream.write_all(response.as_bytes()).await.expect("server should write");
            }
        });

        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-failure-retry-loop-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let reply = super::send_session_message_with_config(
            &runtime,
            SessionMessageRequest {
                session_id: session.id.clone(),
                user_message: "读取附件".to_string(),
                attachments: vec![],
                provider_kind: "openai_compatible".to_string(),
                base_url: format!("http://{}", address),
                model: "gpt-test".to_string(),
                api_key: Some(String::new()),
            },
        )
        .await
        .expect("runtime should stop repeated failing loop safely");

        assert_eq!(reply.action_type, agent::SessionActionType::RequestClarification);
        assert_eq!(request_counter.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn list_session_messages_returns_timeline_messages_for_session() {
        let runtime = AppRuntime::new(format!("/tmp/distilllab-runtime-list-messages-{}.db", Uuid::new_v4()));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let conn = open_database(&runtime.database_path).expect("database should open");
        memory::session_message_store::insert_session_message(
            &conn,
            &schema::SessionMessage {
                id: "message-list-1".to_string(),
                session_id: session.id.clone(),
                run_id: None,
                message_type: "user_message".to_string(),
                role: schema::SessionMessageRole::User,
                content: "Timeline hello".to_string(),
                data_json: "{}".to_string(),
                created_at: "2026-03-29T00:00:00Z".to_string(),
            },
        )
        .expect("seed message should insert");
        drop(conn);

        let messages = super::list_session_messages(&runtime, &session.id)
            .expect("runtime should list session messages");

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Timeline hello");
        assert_eq!(messages[0].role.as_str(), "user");
    }

    #[tokio::test]
    async fn preview_session_intake_returns_distill_run_handoff_with_planned_steps() {
        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-session-intake-preview-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let preview = preview_session_intake(
            &runtime,
            SessionIntake {
                session_id: session.id.clone(),
                user_message: "Please distill these work notes into Distilllab".to_string(),
                attachments: vec![],
                current_object_type: None,
                current_object_id: None,
            },
        )
        .await
        .expect("runtime should preview session intake");

        assert_eq!(preview.decision.intent, SessionIntent::DistillMaterial);

        let handoff = preview
            .run_handoff_preview
            .expect("distill intake should produce a handoff preview");

        assert_eq!(handoff.run_type, "import_and_distill");
        assert_eq!(handoff.planned_steps.len(), 5);
        assert_eq!(handoff.planned_steps[0].step_key, "materialize_sources");
        assert_eq!(handoff.planned_steps[1].step_key, "chunk_sources");
        assert_eq!(handoff.planned_steps[2].step_key, "extract_work_items");
        assert_eq!(handoff.planned_steps[3].step_key, "resolve_project_context");
        assert_eq!(handoff.planned_steps[4].step_key, "extract_assets");
    }

    #[tokio::test]
    async fn preview_session_intake_mentions_attachment_count_in_handoff_summary() {
        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-session-intake-preview-attachments-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let preview = preview_session_intake(
            &runtime,
            SessionIntake {
                session_id: session.id.clone(),
                user_message: "请帮我提炼一下".to_string(),
                attachments: vec![schema::AttachmentRef {
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
        )
        .await
        .expect("runtime should preview session intake");

        let handoff = preview
            .run_handoff_preview
            .expect("distill intake should produce a handoff preview");

        assert!(handoff.summary.contains("1 attachment"));
    }

    #[tokio::test]
    async fn preview_session_intake_with_config_uses_llm_backed_decision() {
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
            let _ = stream
                .read(&mut buffer)
                .await
                .expect("server should read request");

            let response_body = r#"{
                "choices": [
                    {
                        "message": {
                            "role": "assistant",
                            "content": "{\"intent\":\"distill_material\",\"action_type\":\"create_run\",\"reply_text\":\"I will start a distillation workflow for this work material.\",\"primary_object_type\":null,\"primary_object_id\":null,\"suggested_run_type\":\"import_and_distill\",\"session_summary\":\"Preparing to distill work material\",\"tool_call_key\":null}"
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

        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-session-intake-preview-llm-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let preview = preview_session_intake_with_config(
            &runtime,
            SessionIntake {
                session_id: session.id.clone(),
                user_message: "Please distill these work notes into Distilllab".to_string(),
                attachments: vec![],
                current_object_type: None,
                current_object_id: None,
            },
            LlmProviderConfig {
                provider_kind: "openai_compatible".to_string(),
                base_url: format!("http://{}", address),
                model: "gpt-test".to_string(),
                api_key: None,
            },
        )
        .await
        .expect("runtime should preview intake with llm decision");

        assert_eq!(preview.decision.intent, SessionIntent::DistillMaterial);
        assert_eq!(preview.decision.action_type, agent::SessionActionType::CreateRun);
        assert_eq!(
            preview
                .run_handoff_preview
                .expect("preview should include handoff")
                .run_type,
            "import_and_distill"
        );
    }
}
