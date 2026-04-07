use agent::{LlmProviderConfig, SessionActionType, SessionAgentDecision};
use runtime::flows::attachment_storage::{remove_session_attachment_storage, store_attachment_copy};
use runtime::{
    create_session, default_app_config_path, delete_failed_first_send_session,
    delete_provider_entry, import_providers_from_opencode_path, load_app_config_from_path,
    resolve_current_provider_model, save_app_config_to_path, set_current_provider_model,
    upsert_provider_entry, AppConfig, AppRuntime, CanvasDetailViewDto, CanvasGlobalViewDto,
    ChatStreamEvent, ChatStreamPhase,
    DesktopUiConfig, LiveRunEvent, LiveRunState, LiveRunStepStatus, LiveToolEvent,
    LiveToolStatus, LlmSessionDebugRequest, ModelConfigEntry, ProviderConfigEntry,
    ProviderOptions, RunProgressPhase, RunProgressUpdate, SessionIntakePreview,
    SessionMessageExecutionResult, SessionMessageRequest,
};
use schema::{SessionIntake, SessionMessage, SessionMessageRole};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tauri::Emitter;

static STREAM_REQUEST_TASKS: OnceLock<Mutex<HashMap<String, tauri::async_runtime::JoinHandle<()>>>> =
    OnceLock::new();

fn stream_request_tasks() -> &'static Mutex<HashMap<String, tauri::async_runtime::JoinHandle<()>>> {
    STREAM_REQUEST_TASKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn register_stream_request_task(request_id: String, handle: tauri::async_runtime::JoinHandle<()>) {
    if let Ok(mut tasks) = stream_request_tasks().lock() {
        tasks.insert(request_id, handle);
    }
}

fn remove_stream_request_task(request_id: &str) {
    if let Ok(mut tasks) = stream_request_tasks().lock() {
        tasks.remove(request_id);
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigBarForm {
    current_provider: String,
    current_model: String,
    provider_name: String,
    provider_npm: String,
    base_url: String,
    api_key: Option<String>,
    raw_provider_json: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportProvidersForm {
    source_path: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionMessageForm {
    session_id: String,
    user_message: String,
    attachment_paths: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionSelectorOption {
    session_id: String,
    title: String,
    manual_title: Option<String>,
    pinned: bool,
    updated_at: String,
    status: String,
    label: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameSessionForm {
    session_id: String,
    manual_title: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PinSessionForm {
    session_id: String,
    pinned: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PendingAttachmentOption {
    path: String,
    name: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FirstSendCommandResponse {
    session_id: String,
    timeline_text: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StreamSessionMessageForm {
    request_id: String,
    form: SessionMessageForm,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CancelStreamRequestForm {
    session_id: String,
    request_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopUiPreferences {
    theme: String,
    locale: String,
    show_debug_panel: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MaxAgentConcurrencyPayload {
    max_agent_concurrency: u8,
}

fn requested_max_agent_concurrency_to_config_value(requested_value: i64) -> u8 {
    requested_value.clamp(0, u8::MAX as i64) as u8
}

impl Default for DesktopUiPreferences {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            locale: "en".to_string(),
            show_debug_panel: true,
        }
    }
}

fn is_valid_desktop_theme(value: &str) -> bool {
    matches!(value, "system" | "light" | "dark")
}

fn is_valid_desktop_locale(value: &str) -> bool {
    matches!(value, "en" | "zh-CN")
}

fn validate_desktop_ui_preferences(preferences: &DesktopUiPreferences) -> Result<(), String> {
    if !is_valid_desktop_theme(&preferences.theme) {
        return Err("theme must be one of: system, light, dark".to_string());
    }

    if !is_valid_desktop_locale(&preferences.locale) {
        return Err("locale must be one of: en, zh-CN".to_string());
    }

    Ok(())
}

fn desktop_ui_preferences_from_config(config: &AppConfig) -> DesktopUiPreferences {
    let defaults = DesktopUiPreferences::default();

    match config.distilllab.desktop_ui.as_ref() {
        Some(preferences) => DesktopUiPreferences {
            theme: if is_valid_desktop_theme(&preferences.theme) {
                preferences.theme.clone()
            } else {
                defaults.theme
            },
            locale: if is_valid_desktop_locale(&preferences.locale) {
                preferences.locale.clone()
            } else {
                defaults.locale
            },
            show_debug_panel: preferences.show_debug_panel,
        },
        None => defaults,
    }
}

fn desktop_ui_config_from_preferences(
    preferences: &DesktopUiPreferences,
    existing_config: Option<&DesktopUiConfig>,
) -> DesktopUiConfig {
    DesktopUiConfig {
        theme: preferences.theme.clone(),
        locale: preferences.locale.clone(),
        show_debug_panel: preferences.show_debug_panel,
        last_opened_canvas_project_id: existing_config
            .and_then(|config| config.last_opened_canvas_project_id.clone()),
    }
}

fn load_desktop_ui_preferences_from_path(config_path: &std::path::Path) -> Result<String, String> {
    let preferences = if config_path.exists() {
        let config = load_app_config_from_path(config_path).map_err(|e| e.to_string())?;
        desktop_ui_preferences_from_config(&config)
    } else {
        DesktopUiPreferences::default()
    };

    serde_json::to_string_pretty(&preferences).map_err(|e| e.to_string())
}

fn save_desktop_ui_preferences_to_path(
    config_path: &std::path::Path,
    preferences: DesktopUiPreferences,
) -> Result<String, String> {
    validate_desktop_ui_preferences(&preferences)?;

    let mut config = if config_path.exists() {
        load_app_config_from_path(config_path).map_err(|e| e.to_string())?
    } else {
        AppConfig {
            schema: Some("https://opencode.ai/config.json".to_string()),
            ..Default::default()
        }
    };

    let existing_desktop_ui = config.distilllab.desktop_ui.clone();
    config.distilllab.desktop_ui = Some(desktop_ui_config_from_preferences(
        &preferences,
        existing_desktop_ui.as_ref(),
    ));
    save_app_config_to_path(&config, config_path).map_err(|e| e.to_string())?;
    load_desktop_ui_preferences_from_path(config_path)
}

fn load_max_agent_concurrency_from_path(config_path: &std::path::Path) -> Result<String, String> {
    let max_agent_concurrency = if config_path.exists() {
        load_app_config_from_path(config_path)
            .map_err(|e| e.to_string())?
            .distilllab
            .max_agent_concurrency
    } else {
        AppConfig::default().distilllab.max_agent_concurrency
    };

    serde_json::to_string(&MaxAgentConcurrencyPayload {
        max_agent_concurrency,
    })
    .map_err(|e| e.to_string())
}

fn save_max_agent_concurrency_to_path(
    config_path: &std::path::Path,
    requested_value: i64,
) -> Result<String, String> {
    let mut config = if config_path.exists() {
        load_app_config_from_path(config_path).map_err(|e| e.to_string())?
    } else {
        AppConfig {
            schema: Some("https://opencode.ai/config.json".to_string()),
            ..Default::default()
        }
    };

    config.distilllab.max_agent_concurrency = requested_max_agent_concurrency_to_config_value(requested_value);
    save_app_config_to_path(&config, config_path).map_err(|e| e.to_string())?;
    load_max_agent_concurrency_from_path(config_path)
}

fn format_action_type(action_type: &SessionActionType) -> &'static str {
    match action_type {
        SessionActionType::DirectReply => "direct_reply",
        SessionActionType::RequestClarification => "request_clarification",
        SessionActionType::ToolCall => "tool_call",
        SessionActionType::SkillCall => "skill_call",
        SessionActionType::CreateRun => "create_run",
        SessionActionType::Stop => "stop",
    }
}

fn format_optional_text(value: Option<&str>) -> &str {
    value.unwrap_or("none")
}

fn format_session_selector_label(session: &schema::Session) -> String {
    format!("{} ({})", session.title, session.id)
}

fn format_session_agent_decision_text(decision: &SessionAgentDecision) -> String {
    let tool_name = decision
        .tool_invocation
        .as_ref()
        .map(|invocation| invocation.tool_name.as_str());

    [
        format!("intent: {}", decision.intent.as_str()),
        format!(
            "action_type: {}",
            format_action_type(&decision.action_type)
        ),
        format!(
            "primary_object_type: {}",
            format_optional_text(decision.primary_object_type.as_deref())
        ),
        format!(
            "primary_object_id: {}",
            format_optional_text(decision.primary_object_id.as_deref())
        ),
        format!("reply_text: {}", decision.reply_text),
        format!(
            "suggested_run_type: {}",
            format_optional_text(decision.suggested_run_type.as_deref())
        ),
        format!(
            "session_summary: {}",
            format_optional_text(decision.session_summary.as_deref())
        ),
        format!("tool_name: {}", format_optional_text(tool_name)),
        format!(
            "should_continue_planning: {}",
            decision.should_continue_planning
        ),
        format!(
            "failure_hint: {}",
            format_optional_text(decision.failure_hint.as_deref())
        ),
    ]
    .join("\n")
}

fn format_intake_preview_text(preview: &SessionIntakePreview) -> String {
    let mut sections = vec![
        "SessionAgent Decision".to_string(),
        format_session_agent_decision_text(&preview.decision),
    ];

    if let Some(handoff) = &preview.run_handoff_preview {
        sections.push(String::new());
        sections.push("Run Handoff Preview".to_string());
        sections.push(format!("run_type: {}", handoff.run_type));
        sections.push(format!(
            "primary_object_type: {}",
            handoff.primary_object_type.as_deref().unwrap_or("none")
        ));
        sections.push(format!(
            "primary_object_id: {}",
            handoff.primary_object_id.as_deref().unwrap_or("none")
        ));
        sections.push(format!("summary: {}", handoff.summary));
        sections.push("planned_steps:".to_string());
        for step in &handoff.planned_steps {
            sections.push(format!("- {}", step.step_key));
            sections.push(format!("  {}", step.summary));
        }
    }

    sections.join("\n")
}

fn format_session_messages_text(messages: &[SessionMessage]) -> String {
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
                    SessionMessageRole::User => "[User]",
                    SessionMessageRole::Assistant => "[Assistant]",
                    SessionMessageRole::System => "[System]",
                };

                let formatted_content = if message.message_type == "user_message" {
                    let attachment_json = serde_json::from_str::<serde_json::Value>(&message.data_json)
                        .ok()
                        .and_then(|value| value.get("attachments").cloned())
                        .filter(|value| value.is_array() && !value.as_array().unwrap_or(&vec![]).is_empty())
                        .map(|attachments| {
                            serde_json::json!({ "attachments": attachments }).to_string()
                        });

                    match attachment_json {
                        Some(json) if !message.content.trim().is_empty() => {
                            format!("{}\n{}", message.content, json)
                        }
                        Some(json) => json,
                        None => message.content.clone(),
                    }
                } else {
                    message.content.clone()
                };

                format!("{}\n{}", role_header, indent_block(&formatted_content))
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopTimelineAttachment {
    name: String,
    size: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopRunStepMeta {
    key: String,
    summary: String,
    status: String,
    index: Option<u32>,
    total: Option<u32>,
    detail_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopRunCardMeta {
    run_id: String,
    state: String,
    progress_percent: u8,
    run_type: Option<String>,
    step_key: Option<String>,
    step_summary: Option<String>,
    step_status: Option<String>,
    step_index: Option<u32>,
    steps_total: Option<u32>,
    detail_text: Option<String>,
    current_step_key: Option<String>,
    steps: Vec<DesktopRunStepMeta>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopTimelineMessage {
    id: String,
    role: String,
    kind: String,
    source_message_type: Option<String>,
    content: String,
    summary: Option<String>,
    details: Option<String>,
    attachments: Vec<DesktopTimelineAttachment>,
    run_meta: Option<DesktopRunCardMeta>,
    created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ToolPresentation {
    content: String,
    summary: String,
    details: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RunPresentation {
    content: String,
    summary: String,
    details: String,
}

fn desktop_timeline_from_session_messages(messages: &[SessionMessage]) -> Vec<DesktopTimelineMessage> {
    let run_anchor_messages = earliest_run_progress_message_by_run_id(messages);
    let run_cards = desktop_run_cards_from_progress_messages(messages)
        .into_iter()
        .map(|message| {
            (
                message
                    .run_meta
                    .as_ref()
                    .map(|meta| meta.run_id.clone())
                    .unwrap_or_default(),
                message,
            )
        })
        .collect::<HashMap<_, _>>();
    let mut inserted_run_cards = HashMap::<String, bool>::new();
    let mut timeline = Vec::new();

    for message in messages {
        if message.message_type == "run_progress_message" {
            let Some(run_id) = message.run_id.as_ref() else {
                continue;
            };

            if inserted_run_cards.contains_key(run_id) {
                continue;
            }

            if run_anchor_messages
                .get(run_id)
                .map(|anchor_message| anchor_message.id != message.id)
                .unwrap_or(true)
            {
                continue;
            }

            if let Some(run_card) = run_cards.get(run_id) {
                timeline.push(run_card.clone());
                inserted_run_cards.insert(run_id.clone(), true);
            }

            continue;
        }

        if let Some(timeline_message) = desktop_message_from_session_message(message) {
            timeline.push(timeline_message);
        }
    }

    timeline
}

fn list_session_messages_structured_payload(
    messages: &[SessionMessage],
) -> Vec<DesktopTimelineMessage> {
    desktop_timeline_from_session_messages(messages)
}

fn session_message_chronology_cmp(left: &SessionMessage, right: &SessionMessage) -> std::cmp::Ordering {
    left.created_at
        .cmp(&right.created_at)
        .then(left.id.cmp(&right.id))
}

fn earliest_run_progress_message_by_run_id<'a>(
    messages: &'a [SessionMessage],
) -> HashMap<String, &'a SessionMessage> {
    let mut anchors = HashMap::<String, &'a SessionMessage>::new();

    for message in messages {
        if message.message_type != "run_progress_message" {
            continue;
        }

        let Some(run_id) = message.run_id.as_ref() else {
            continue;
        };

        match anchors.get(run_id) {
            Some(current_anchor)
                if session_message_chronology_cmp(message, current_anchor)
                    != std::cmp::Ordering::Less => {}
            _ => {
                anchors.insert(run_id.clone(), message);
            }
        }
    }

    anchors
}

fn load_session_messages_for_timeline(session_id: &str) -> Result<Vec<SessionMessage>, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    runtime::list_session_messages(&runtime, session_id).map_err(|e| e.to_string())
}

fn load_session_messages_text_for_timeline(session_id: &str) -> Result<String, String> {
    let messages = load_session_messages_for_timeline(session_id)?;
    Ok(format_session_messages_text(&messages))
}

fn desktop_message_from_session_message(message: &SessionMessage) -> Option<DesktopTimelineMessage> {
    if message.message_type == "run_progress_message" {
        return None;
    }

    if message.message_type == "tool_result_message" {
        return Some(desktop_tool_message_from_session_message(message));
    }

    let attachments = if message.message_type == "user_message" {
        serde_json::from_str::<serde_json::Value>(&message.data_json)
            .ok()
            .and_then(|value| value.get("attachments").cloned())
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .map(|attachment| DesktopTimelineAttachment {
                name: attachment
                    .get("name")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string(),
                size: attachment.get("size").and_then(|value| value.as_u64()),
            })
            .collect()
    } else {
        Vec::new()
    };

    Some(DesktopTimelineMessage {
        id: message.id.clone(),
        role: message.role.as_str().to_string(),
        kind: "message".to_string(),
        source_message_type: Some(message.message_type.clone()),
        content: message.content.clone(),
        summary: None,
        details: None,
        attachments,
        run_meta: None,
        created_at: message.created_at.clone(),
    })
}

fn live_tool_status_label(status: &LiveToolStatus) -> &'static str {
    match status {
        LiveToolStatus::Started => "started",
        LiveToolStatus::Succeeded => "success",
        LiveToolStatus::Failed => "failed",
    }
}

fn persisted_tool_status_from_message_data(
    data: Option<&serde_json::Value>,
    content: &str,
) -> LiveToolStatus {
    match data
        .and_then(|value| value.get("status"))
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_lowercase())
    {
        Some(status) if matches!(status.as_str(), "started" | "running") => LiveToolStatus::Started,
        Some(status) if matches!(status.as_str(), "failed" | "error") => LiveToolStatus::Failed,
        Some(status) if matches!(status.as_str(), "succeeded" | "success" | "completed" | "finished") => {
            LiveToolStatus::Succeeded
        }
        Some(_) | None => {
            let content_lower = content.to_lowercase();
            if content_lower.contains("error")
                || content_lower.contains("failed")
                || content_lower.contains("failure")
            {
                LiveToolStatus::Failed
            } else {
                LiveToolStatus::Succeeded
            }
        }
    }
}

fn build_tool_presentation(
    tool_name: &str,
    status: LiveToolStatus,
    arguments_text: Option<&str>,
    result_text: Option<&str>,
) -> ToolPresentation {
    let status_label = live_tool_status_label(&status);
    let content = result_text.unwrap_or("Tool result unavailable.").to_string();
    let summary = format!("{} · {}", tool_name, status_label);
    let details = format!(
        "tool: {}\nstatus: {}\narguments: {}\n\nresult:\n{}",
        tool_name,
        status_label,
        arguments_text.unwrap_or("{}"),
        content
    );

    ToolPresentation {
        content,
        summary,
        details,
    }
}

fn desktop_tool_message_from_session_message(message: &SessionMessage) -> DesktopTimelineMessage {
    let data = serde_json::from_str::<serde_json::Value>(&message.data_json).ok();
    let tool_name = data
        .as_ref()
        .and_then(|value| value.get("tool_name"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unknown_tool");
    let arguments = data
        .as_ref()
        .and_then(|value| value.get("arguments"))
        .and_then(|value| serde_json::to_string(value).ok())
        .unwrap_or_else(|| "{}".to_string());
    // Historical persisted rows only store raw tool metadata plus result content, so when a
    // status field is absent we still need to infer it here. The bridge owns that fallback.
    let presentation = build_tool_presentation(
        tool_name,
        persisted_tool_status_from_message_data(data.as_ref(), &message.content),
        Some(arguments.as_str()),
        Some(message.content.as_str()).filter(|value| !value.is_empty()),
    );

    DesktopTimelineMessage {
        id: message.id.clone(),
        role: message.role.as_str().to_string(),
        kind: "tool".to_string(),
        source_message_type: Some(message.message_type.clone()),
        content: presentation.content,
        summary: Some(presentation.summary),
        details: Some(presentation.details),
        attachments: Vec::new(),
        run_meta: None,
        created_at: message.created_at.clone(),
    }
}

fn normalize_structured_run_state(value: Option<&str>) -> String {
    match value.unwrap_or_default().trim().to_lowercase().as_str() {
        "queued" | "created" => "queued".to_string(),
        "running" => "running".to_string(),
        "completed" | "finished" => "completed".to_string(),
        "failed" | "error" => "failed".to_string(),
        _ => "pending".to_string(),
    }
}

fn normalize_structured_step_status(value: Option<&str>) -> Option<String> {
    value.map(|status| match status.trim().to_lowercase().as_str() {
        "started" => "started".to_string(),
        "running" => "running".to_string(),
        "completed" | "finished" => "completed".to_string(),
        "failed" | "error" => "failed".to_string(),
        _ => "pending".to_string(),
    })
}

fn run_step_status_text(phase: &RunProgressPhase) -> &'static str {
    match phase {
        RunProgressPhase::Created | RunProgressPhase::StateChanged => "progress updated",
        RunProgressPhase::StepStarted => "step started",
        RunProgressPhase::StepFinished => "step finished",
    }
}

fn build_run_presentation(
    run_id: &str,
    run_type: Option<&str>,
    state: &LiveRunState,
    progress_percent: Option<u8>,
    detail_text: Option<&str>,
    phase: Option<&RunProgressPhase>,
    step_key: Option<&str>,
    step_summary: Option<&str>,
    status_text: Option<&str>,
) -> RunPresentation {
    let summary = match (step_summary.filter(|value| !value.trim().is_empty()), run_type) {
        (Some(summary), _) => summary.to_string(),
        (None, Some(run_type)) if !run_type.trim().is_empty() => run_type.to_string(),
        _ => format!("run {}", run_id),
    };

    let content = status_text
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .or_else(|| {
            detail_text
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| match run_type {
            Some(run_type) if !run_type.trim().is_empty() => {
                format!("{} · {}", run_type, live_run_state_label(state))
            }
            _ => format!("run {} {}", run_id, live_run_state_label(state)),
        });

    let mut detail_lines = vec![
        format!("run: {}", run_id),
        format!("state: {}", live_run_state_label(state)),
    ];

    if let Some(run_type) = run_type.filter(|value| !value.trim().is_empty()) {
        detail_lines.push(format!("type: {}", run_type));
    }

    if let Some(progress_percent) = progress_percent {
        detail_lines.push(format!("progress: {}%", progress_percent));
    }

    if let Some(phase) = phase {
        detail_lines.push(format!("phase: {}", run_step_status_text(phase)));
    }

    if let Some(step_key) = step_key.filter(|value| !value.trim().is_empty()) {
        detail_lines.push(format!("stepKey: {}", step_key));
    }

    if let Some(step_summary) = step_summary.filter(|value| !value.trim().is_empty()) {
        detail_lines.push(format!("stepSummary: {}", step_summary));
    }

    if let Some(detail_text) = detail_text.filter(|value| !value.trim().is_empty()) {
        detail_lines.push(format!("detail: {}", detail_text));
    }

    if let Some(status_text) = status_text.filter(|value| !value.trim().is_empty()) {
        detail_lines.push(format!("statusText: {}", status_text));
    }

    RunPresentation {
        content,
        summary,
        details: detail_lines.join("\n"),
    }
}

fn desktop_run_cards_from_progress_messages(messages: &[SessionMessage]) -> Vec<DesktopTimelineMessage> {
    let run_anchor_messages = earliest_run_progress_message_by_run_id(messages);
    let mut grouped_messages = HashMap::<String, Vec<&SessionMessage>>::new();

    for message in messages {
        if message.message_type != "run_progress_message" {
            continue;
        }

        let Some(run_id) = message.run_id.as_ref() else {
            continue;
        };

        grouped_messages.entry(run_id.clone()).or_default().push(message);
    }

    let mut run_cards = grouped_messages
        .into_iter()
        .filter_map(|(run_id, messages)| {
            let anchor_message = run_anchor_messages.get(&run_id).copied()?;
            let mut latest_progress = None;
            let mut steps_by_key = HashMap::<String, (String, String, DesktopRunStepMeta)>::new();

            for message in &messages {
                let Some(progress) = serde_json::from_str::<serde_json::Value>(&message.data_json)
                    .ok()
                    .and_then(|value| value.get("runProgress").cloned())
                else {
                    continue;
                };

                match &latest_progress {
                    Some((current_created_at, current_id, _))
                        if (&message.created_at, &message.id) <= (current_created_at, current_id) => {}
                    _ => {
                        latest_progress =
                            Some((message.created_at.clone(), message.id.clone(), progress.clone()));
                    }
                }

                if let Some(step_key) = progress.get("stepKey").and_then(|value| value.as_str()) {
                    let next_step = DesktopRunStepMeta {
                        key: step_key.to_string(),
                        summary: progress
                            .get("stepSummary")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        status: normalize_structured_step_status(
                            progress.get("stepStatus").and_then(|value| value.as_str()),
                        )
                        .unwrap_or_else(|| "pending".to_string()),
                        index: progress
                            .get("stepIndex")
                            .and_then(|value| value.as_u64())
                            .and_then(|value| u32::try_from(value).ok()),
                        total: progress
                            .get("stepsTotal")
                            .and_then(|value| value.as_u64())
                            .and_then(|value| u32::try_from(value).ok()),
                        detail_text: progress
                            .get("detailText")
                            .and_then(|value| value.as_str())
                            .map(str::to_string),
                    };

                    match steps_by_key.get(step_key) {
                        Some((current_created_at, current_id, _))
                            if (&message.created_at, &message.id) <= (current_created_at, current_id) => {}
                        _ => {
                            steps_by_key.insert(
                                step_key.to_string(),
                                (message.created_at.clone(), message.id.clone(), next_step),
                            );
                        }
                    }
                }
            }

            let (_, _, latest_progress) = latest_progress?;
            let mut steps = steps_by_key
                .into_values()
                .map(|(_, _, step)| step)
                .collect::<Vec<_>>();
            steps.sort_by(|left, right| left.index.cmp(&right.index).then(left.key.cmp(&right.key)));

            let latest_status_text = messages
                .iter()
                .max_by(|left, right| {
                    (&left.created_at, &left.id).cmp(&(&right.created_at, &right.id))
                })
                .map(|message| message.content.clone())
                .unwrap_or_else(|| anchor_message.content.clone());
            let latest_progress_phase = latest_progress
                .get("phase")
                .and_then(|value| value.as_str())
                .map(|value| match value {
                    "created" => RunProgressPhase::Created,
                    "state_changed" => RunProgressPhase::StateChanged,
                    "step_started" => RunProgressPhase::StepStarted,
                    "step_finished" => RunProgressPhase::StepFinished,
                    _ => RunProgressPhase::StateChanged,
                });
            let run_state = normalize_live_run_state(
                latest_progress.get("runState").and_then(|value| value.as_str()),
            );
            let run_presentation = build_run_presentation(
                &run_id,
                latest_progress.get("runType").and_then(|value| value.as_str()),
                &run_state,
                latest_progress
                    .get("progressPercent")
                    .and_then(|value| value.as_u64())
                    .and_then(|value| u8::try_from(value).ok()),
                latest_progress.get("detailText").and_then(|value| value.as_str()),
                latest_progress_phase.as_ref(),
                latest_progress.get("stepKey").and_then(|value| value.as_str()),
                latest_progress
                    .get("stepSummary")
                    .and_then(|value| value.as_str()),
                Some(latest_status_text.as_str()),
            );

            Some(DesktopTimelineMessage {
                id: format!("run-card-{}", run_id),
                role: SessionMessageRole::System.as_str().to_string(),
                kind: "run".to_string(),
                source_message_type: Some("run_progress_message".to_string()),
                content: run_presentation.content,
                summary: Some(run_presentation.summary),
                details: Some(run_presentation.details),
                attachments: Vec::new(),
                run_meta: Some(DesktopRunCardMeta {
                    run_id: run_id.clone(),
                    state: normalize_structured_run_state(
                        latest_progress.get("runState").and_then(|value| value.as_str()),
                    ),
                    progress_percent: latest_progress
                        .get("progressPercent")
                        .and_then(|value| value.as_u64())
                        .and_then(|value| u8::try_from(value).ok())
                        .unwrap_or(0),
                    run_type: latest_progress
                        .get("runType")
                        .and_then(|value| value.as_str())
                        .map(str::to_string),
                    step_key: latest_progress
                        .get("stepKey")
                        .and_then(|value| value.as_str())
                        .map(str::to_string),
                    step_summary: latest_progress
                        .get("stepSummary")
                        .and_then(|value| value.as_str())
                        .map(str::to_string),
                    step_status: normalize_structured_step_status(
                        latest_progress.get("stepStatus").and_then(|value| value.as_str()),
                    ),
                    step_index: latest_progress
                        .get("stepIndex")
                        .and_then(|value| value.as_u64())
                        .and_then(|value| u32::try_from(value).ok()),
                    steps_total: latest_progress
                        .get("stepsTotal")
                        .and_then(|value| value.as_u64())
                        .and_then(|value| u32::try_from(value).ok()),
                    detail_text: latest_progress
                        .get("detailText")
                        .and_then(|value| value.as_str())
                        .map(str::to_string),
                    current_step_key: latest_progress
                        .get("stepKey")
                        .and_then(|value| value.as_str())
                        .map(str::to_string),
                    steps,
                }),
                created_at: anchor_message.created_at.clone(),
            })
        })
        .collect::<Vec<_>>();

    run_cards.sort_by(|left, right| left.created_at.cmp(&right.created_at).then(left.id.cmp(&right.id)));
    run_cards
}

#[cfg(test)]
mod tests {
    use super::{
        build_chat_stream_event_with_run_progress, build_live_run_event, build_live_tool_event,
        desktop_timeline_from_session_messages, format_session_messages_text,
        load_canvas_global_view, load_canvas_object_detail,
        list_session_messages_structured_payload, load_max_agent_concurrency_command,
        normalize_live_run_progress_update, save_max_agent_concurrency_command,
    };
    use runtime::{
        build_demo_assets, default_app_config_path, group_demo_project, load_app_config_from_path,
        ChatStreamPhase, LiveRunState,
        LiveRunStepStatus, LiveToolStatus, RunProgressPhase, RunProgressUpdate, AppRuntime,
    };
    use schema::{SessionMessage, SessionMessageRole};
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    fn temp_config_dir() -> std::path::PathBuf {
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("distilllab-desktop-react-test-{unique_id}"));
        std::fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        temp_dir
    }

    struct TestConfigHomeGuard {
        _lock_guard: std::sync::MutexGuard<'static, ()>,
        previous_xdg: Option<std::ffi::OsString>,
    }

    impl TestConfigHomeGuard {
        fn new(config_home: &std::path::Path) -> Self {
            let lock_guard = env_lock()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let previous_xdg = std::env::var_os("XDG_CONFIG_HOME");

            unsafe {
                std::env::set_var("XDG_CONFIG_HOME", config_home);
            }

            Self {
                _lock_guard: lock_guard,
                previous_xdg,
            }
        }
    }

    impl Drop for TestConfigHomeGuard {
        fn drop(&mut self) {
            unsafe {
                match self.previous_xdg.as_ref() {
                    Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                    None => std::env::remove_var("XDG_CONFIG_HOME"),
                }
            }
        }
    }

    struct CanvasCommandRuntimeGuard {
        _lock_guard: std::sync::MutexGuard<'static, ()>,
        original_dir: std::path::PathBuf,
        temp_dir: std::path::PathBuf,
        database_path: std::path::PathBuf,
    }

    impl CanvasCommandRuntimeGuard {
        fn new(temp_dir: std::path::PathBuf) -> Self {
            let lock_guard = env_lock()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let original_dir = std::env::current_dir().expect("current dir should resolve");
            let database_path = temp_dir.join("distilllab-dev.db");

            std::env::set_current_dir(&temp_dir).expect("test dir should become current dir");

            Self {
                _lock_guard: lock_guard,
                original_dir,
                temp_dir,
                database_path,
            }
        }
    }

    impl Drop for CanvasCommandRuntimeGuard {
        fn drop(&mut self) {
            std::env::set_current_dir(&self.original_dir).expect("original dir should be restored");
            let _ = std::fs::remove_file(&self.database_path);
            let _ = std::fs::remove_dir_all(&self.temp_dir);
        }
    }

    fn with_test_config_home<T>(test: impl FnOnce(std::path::PathBuf) -> T) -> T {
        let config_home = temp_config_dir();
        let _guard = TestConfigHomeGuard::new(&config_home);

        test(config_home)
    }

    fn with_canvas_command_runtime<T>(test: impl FnOnce(&AppRuntime) -> T) -> T {
        let temp_dir = temp_config_dir();
        let guard = CanvasCommandRuntimeGuard::new(temp_dir.clone());
        let database_path = guard.database_path.clone();
        let runtime = AppRuntime::new(database_path.to_string_lossy().to_string());

        let result = test(&runtime);
        drop(guard);
        result
    }

    #[test]
    fn with_test_config_home_restores_xdg_config_home_after_panic() {
        let original_xdg = std::env::var_os("XDG_CONFIG_HOME");
        let panic_result = std::panic::catch_unwind(|| {
            with_test_config_home(|_| panic!("expected panic inside config helper"));
        });

        assert!(panic_result.is_err());
        assert_eq!(std::env::var_os("XDG_CONFIG_HOME"), original_xdg);
    }

    #[test]
    fn with_canvas_command_runtime_restores_current_dir_after_panic() {
        let expected_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let panic_result = std::panic::catch_unwind(|| {
            with_canvas_command_runtime(|_| panic!("expected panic inside runtime helper"));
        });

        assert!(panic_result.is_err());
        assert_eq!(
            std::env::current_dir().expect("current dir should still resolve"),
            expected_dir
        );
    }

    #[test]
    fn load_canvas_global_view_command_returns_runtime_canvas_dto_shape() {
        with_canvas_command_runtime(|runtime| {
            let (_source, _chunks, _work_items, project, assets) =
                build_demo_assets(runtime).expect("demo assets should build");

            let projection =
                load_canvas_global_view(Some(project.id.clone())).expect("projection should load");

            assert_eq!(projection.current_project_id.as_deref(), Some(project.id.as_str()));
            assert!(projection.graph.nodes.iter().any(|node| node.id == project.id && node.node_type == "project"));
            assert!(projection
                .graph
                .nodes
                .iter()
                .any(|node| node.id == assets[0].id && node.node_type == "asset"));
            assert!(projection
                .graph
                .nodes
                .iter()
                .all(|node| matches!(node.node_type.as_str(), "project" | "work_item" | "asset")));
            assert!(!projection
                .graph
                .nodes
                .iter()
                .any(|node| node.node_type == "source" || node.node_type == "chunk"));
            assert_eq!(
                projection.inspectors_by_node_id[&project.id].node_type,
                "project"
            );

            let serialized = serde_json::to_value(&projection).expect("projection should serialize");
            assert!(serialized.get("currentProjectId").is_some());
            assert!(serialized.get("inspectorsByNodeId").is_some());
            assert!(serialized.get("current_project_id").is_none());
            assert!(serialized.get("inspectors_by_node_id").is_none());

            let first_node = serialized["graph"]["nodes"]
                .as_array()
                .and_then(|nodes| nodes.first())
                .expect("graph should include nodes");
            assert!(first_node.get("nodeType").is_some());
            assert!(first_node.get("node_type").is_none());
        });
    }

    #[test]
    fn load_canvas_object_detail_command_returns_typed_detail_payloads() {
        with_canvas_command_runtime(|runtime| {
            let (source, chunks, _work_items, project, assets) =
                build_demo_assets(runtime).expect("demo assets should build");

            let project_projection = load_canvas_object_detail(
                "project".to_string(),
                project.id.clone(),
                None,
            )
            .expect("project detail should load");
            assert_eq!(project_projection.focus_node_type, "project");
            assert_eq!(project_projection.focus_node_id, project.id);

            let asset_projection = load_canvas_object_detail(
                "asset".to_string(),
                assets[0].id.clone(),
                None,
            )
            .expect("asset detail should load");
            assert_eq!(asset_projection.focus_node_type, "asset");
            assert_eq!(
                asset_projection.inspectors_by_node_id[&assets[0].id]
                    .fields
                    .get("projectId")
                    .map(String::as_str),
                Some(project.id.as_str())
            );

            let source_projection = load_canvas_object_detail(
                "source".to_string(),
                source.id.clone(),
                Some(project.id.clone()),
            )
            .expect("source detail should load");
            assert_eq!(source_projection.focus_node_type, "source");
            assert_eq!(
                source_projection.inspectors_by_node_id[&source.id]
                    .fields
                    .get("title")
                    .map(String::as_str),
                Some(source.title.as_str())
            );

            let chunk_projection = load_canvas_object_detail(
                "chunk".to_string(),
                chunks[0].id.clone(),
                Some(project.id.clone()),
            )
            .expect("chunk detail should load");
            assert_eq!(chunk_projection.focus_node_type, "chunk");
            assert_eq!(
                chunk_projection.inspectors_by_node_id[&chunks[0].id]
                    .fields
                    .get("parentSource")
                    .map(String::as_str),
                Some(source.id.as_str())
            );

            let serialized =
                serde_json::to_value(&chunk_projection).expect("detail projection should serialize");
            assert!(serialized.get("focusNodeId").is_some());
            assert!(serialized.get("focusNodeType").is_some());
            assert!(serialized.get("inspectorsByNodeId").is_some());
            assert!(serialized.get("focus_node_id").is_none());
            assert!(serialized.get("focus_node_type").is_none());

            let first_edge = serialized["graph"]["edges"]
                .as_array()
                .and_then(|edges| edges.first())
                .expect("detail graph should include edges");
            assert!(first_edge.get("edgeType").is_some());
            assert!(first_edge.get("edge_type").is_none());
        });
    }

    #[test]
    fn load_canvas_object_detail_command_keeps_source_and_chunk_project_context_contextual() {
        with_canvas_command_runtime(|runtime| {
            let (source, chunks, _work_items, project) =
                group_demo_project(runtime).expect("demo project should build");
            let source_projection = load_canvas_object_detail(
                "source".to_string(),
                source.id.clone(),
                Some(project.id.clone()),
            )
            .expect("source detail should load");

            assert!(source_projection
                .graph
                .nodes
                .iter()
                .any(|node| node.id == project.id && node.node_type == "project"));
            assert_eq!(
                source_projection.inspectors_by_node_id[&source.id]
                    .fields
                    .get("parentProject"),
                None
            );

            let chunk_projection = load_canvas_object_detail(
                "chunk".to_string(),
                chunks[0].id.clone(),
                Some(project.id.clone()),
            )
            .expect("chunk detail should load");

            assert!(chunk_projection
                .graph
                .nodes
                .iter()
                .any(|node| node.id == source.id && node.node_type == "source"));
            assert_eq!(
                chunk_projection.inspectors_by_node_id[&chunks[0].id]
                    .fields
                    .get("parentProject"),
                None
            );
        });
    }

    #[test]
    fn load_max_agent_concurrency_command_returns_default_when_config_has_no_value() {
        with_test_config_home(|_| {
            let config_path = default_app_config_path().expect("config path should resolve");
            let config_parent = config_path.parent().expect("config path should have parent");
            std::fs::create_dir_all(config_parent).expect("config parent should be created");

            std::fs::write(
                &config_path,
                r#"{
                    "distilllab": {
                        "currentProvider": "ice"
                    }
                }"#,
            )
            .expect("config file should be written");

            let payload = load_max_agent_concurrency_command().expect("value should load");
            let value: serde_json::Value = serde_json::from_str(&payload).expect("payload should be valid json");

            assert_eq!(value, serde_json::json!({ "maxAgentConcurrency": 4 }));
        });
    }

    #[test]
    fn save_max_agent_concurrency_command_persists_and_returns_normalized_value() {
        with_test_config_home(|_| {
            let config_path = default_app_config_path().expect("config path should resolve");

            let saved_low = save_max_agent_concurrency_command(0).expect("low value should save");
            let saved_low_value: serde_json::Value =
                serde_json::from_str(&saved_low).expect("low payload should be valid json");
            assert_eq!(saved_low_value, serde_json::json!({ "maxAgentConcurrency": 1 }));

            let persisted_low = load_app_config_from_path(&config_path).expect("config should reload after low save");
            assert_eq!(persisted_low.distilllab.max_agent_concurrency, 1);

            let saved_high = save_max_agent_concurrency_command(i64::from(u8::MAX))
                .expect("high value should save");
            let saved_high_value: serde_json::Value =
                serde_json::from_str(&saved_high).expect("high payload should be valid json");
            assert_eq!(saved_high_value, serde_json::json!({ "maxAgentConcurrency": 16 }));

            let persisted_high = load_app_config_from_path(&config_path).expect("config should reload after high save");
            assert_eq!(persisted_high.distilllab.max_agent_concurrency, 16);
        });
    }

    #[test]
    fn save_max_agent_concurrency_command_normalizes_negative_and_large_inputs() {
        with_test_config_home(|_| {
            let config_path = default_app_config_path().expect("config path should resolve");

            let saved_negative =
                save_max_agent_concurrency_command(-1).expect("negative value should save");
            let saved_negative_value: serde_json::Value =
                serde_json::from_str(&saved_negative).expect("negative payload should be valid json");
            assert_eq!(saved_negative_value, serde_json::json!({ "maxAgentConcurrency": 1 }));

            let saved_large = save_max_agent_concurrency_command(999).expect("large value should save");
            let saved_large_value: serde_json::Value =
                serde_json::from_str(&saved_large).expect("large payload should be valid json");
            assert_eq!(saved_large_value, serde_json::json!({ "maxAgentConcurrency": 16 }));

            let persisted = load_app_config_from_path(&config_path).expect("config should reload after saves");
            assert_eq!(persisted.distilllab.max_agent_concurrency, 16);
        });
    }

    fn session_message(
        id: &str,
        run_id: Option<&str>,
        message_type: &str,
        role: SessionMessageRole,
        content: &str,
        data_json: serde_json::Value,
        created_at: &str,
    ) -> SessionMessage {
        SessionMessage {
            id: id.to_string(),
            session_id: "session-1".to_string(),
            run_id: run_id.map(str::to_string),
            message_type: message_type.to_string(),
            role,
            content: content.to_string(),
            data_json: data_json.to_string(),
            created_at: created_at.to_string(),
        }
    }

    #[test]
    fn format_session_messages_text_preserves_user_attachment_metadata_for_reopen() {
        let timeline = format_session_messages_text(&[
            SessionMessage {
                id: "message-user".to_string(),
                session_id: "session-1".to_string(),
                run_id: None,
                message_type: "user_message".to_string(),
                role: SessionMessageRole::User,
                content: "Please review the attached notes".to_string(),
                data_json: serde_json::json!({
                    "attachments": [
                        {
                            "name": "notes.md",
                            "size": 2048
                        }
                    ]
                })
                .to_string(),
                created_at: "2026-04-05T00:00:00Z".to_string(),
            },
        ]);

        assert!(timeline.contains("[User]"));
        assert!(timeline.contains("Please review the attached notes"));
        assert!(timeline.contains("\"attachments\""));
        assert!(timeline.contains("\"name\":\"notes.md\""));
    }

    #[test]
    fn desktop_timeline_mapper_preserves_user_attachments() {
        let timeline = desktop_timeline_from_session_messages(&[session_message(
            "message-user",
            None,
            "user_message",
            SessionMessageRole::User,
            "Please review the attached notes",
            serde_json::json!({
                "attachments": [
                    {
                        "name": "notes.md",
                        "size": 2048,
                        "mimeType": "text/markdown"
                    }
                ]
            }),
            "2026-04-05T00:00:00Z",
        )]);

        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline[0].id, "message-user");
        assert_eq!(timeline[0].role, "user");
        assert_eq!(timeline[0].kind, "message");
        assert_eq!(timeline[0].source_message_type.as_deref(), Some("user_message"));
        assert_eq!(timeline[0].content, "Please review the attached notes");
        assert_eq!(timeline[0].attachments.len(), 1);
        assert_eq!(timeline[0].attachments[0].name, "notes.md");
        assert_eq!(timeline[0].attachments[0].size, Some(2048));
        assert_eq!(timeline[0].created_at, "2026-04-05T00:00:00Z");
    }

    #[test]
    fn desktop_timeline_mapper_converts_tool_result_messages() {
        let timeline = desktop_timeline_from_session_messages(&[session_message(
            "message-tool",
            None,
            "tool_result_message",
            SessionMessageRole::System,
            "Attachment excerpt: hello",
            serde_json::json!({
                "tool_name": "read_attachment_excerpt",
                "arguments": {
                    "attachment_index": 0,
                    "max_chars": 400
                }
            }),
            "2026-04-05T00:00:01Z",
        )]);

        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline[0].id, "message-tool");
        assert_eq!(timeline[0].role, "system");
        assert_eq!(timeline[0].kind, "tool");
        assert_eq!(timeline[0].source_message_type.as_deref(), Some("tool_result_message"));
        assert_eq!(timeline[0].content, "Attachment excerpt: hello");
        let details = timeline[0].details.as_deref().expect("tool details");
        assert_eq!(timeline[0].summary.as_deref(), Some("read_attachment_excerpt · success"));
        assert!(details.contains("tool: read_attachment_excerpt"));
        assert!(details.contains("status: success"));
        assert!(details.contains("attachment_index"));
        assert!(details.contains("max_chars"));
        assert!(details.contains("Attachment excerpt: hello"));
        assert!(timeline[0].details.is_some());
        assert!(timeline[0].run_meta.is_none());
    }

    #[test]
    fn desktop_timeline_mapper_aggregates_run_progress_into_one_run_card() {
        let timeline = desktop_timeline_from_session_messages(&[
            session_message(
                "progress-1",
                Some("run-123"),
                "run_progress_message",
                SessionMessageRole::System,
                "run created: run-123 (distill)",
                serde_json::json!({
                    "statusText": "run created: run-123 (distill)",
                    "runProgress": {
                        "phase": "created",
                        "runId": "run-123",
                        "runType": "distill",
                        "runState": "queued",
                        "progressPercent": 10,
                        "stepKey": "gather",
                        "stepSummary": "Gather material",
                        "stepStatus": "started",
                        "stepIndex": 1,
                        "stepsTotal": 2,
                        "detailText": "Collecting sources"
                    }
                }),
                "2026-04-05T00:00:02Z",
            ),
            session_message(
                "progress-2",
                Some("run-123"),
                "run_progress_message",
                SessionMessageRole::System,
                "run step finished: run-123 [draft] (80%)",
                serde_json::json!({
                    "statusText": "run step finished: run-123 [draft] (80%)",
                    "runProgress": {
                        "phase": "step_finished",
                        "runId": "run-123",
                        "runType": "distill",
                        "runState": "running",
                        "progressPercent": 80,
                        "stepKey": "draft",
                        "stepSummary": "Draft answer",
                        "stepStatus": "completed",
                        "stepIndex": 2,
                        "stepsTotal": 2,
                        "detailText": "Draft complete"
                    }
                }),
                "2026-04-05T00:00:03Z",
            ),
        ]);

        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline[0].id, "run-card-run-123");
        assert_eq!(timeline[0].kind, "run");
        assert_eq!(timeline[0].role, "system");
        assert_eq!(timeline[0].source_message_type.as_deref(), Some("run_progress_message"));
        assert_eq!(timeline[0].created_at, "2026-04-05T00:00:02Z");

        let run_meta = timeline[0].run_meta.as_ref().expect("run card meta");
        assert_eq!(run_meta.run_id, "run-123");
        assert_eq!(run_meta.state, "running");
        assert_eq!(run_meta.progress_percent, 80);
        assert_eq!(run_meta.run_type.as_deref(), Some("distill"));
        assert_eq!(run_meta.current_step_key.as_deref(), Some("draft"));
        assert_eq!(run_meta.steps.len(), 2);
        assert_eq!(run_meta.steps[0].key, "gather");
        assert_eq!(run_meta.steps[1].key, "draft");
    }

    #[test]
    fn desktop_timeline_mapper_places_run_card_at_earliest_run_progress_position() {
        let timeline = desktop_timeline_from_session_messages(&[
            session_message(
                "message-user",
                None,
                "user_message",
                SessionMessageRole::User,
                "Start the run",
                serde_json::json!({}),
                "2026-04-05T00:00:00Z",
            ),
            session_message(
                "progress-2",
                Some("run-123"),
                "run_progress_message",
                SessionMessageRole::System,
                "run step finished: run-123 [draft] (100%)",
                serde_json::json!({
                    "statusText": "run step finished: run-123 [draft] (100%)",
                    "runProgress": {
                        "phase": "step_finished",
                        "runId": "run-123",
                        "runType": "distill",
                        "runState": "completed",
                        "progressPercent": 100,
                        "stepKey": "draft",
                        "stepSummary": "Draft answer",
                        "stepStatus": "completed",
                        "stepIndex": 2,
                        "stepsTotal": 2,
                        "detailText": "Draft complete"
                    }
                }),
                "2026-04-05T00:00:03Z",
            ),
            session_message(
                "progress-1",
                Some("run-123"),
                "run_progress_message",
                SessionMessageRole::System,
                "run created: run-123 (distill)",
                serde_json::json!({
                    "statusText": "run created: run-123 (distill)",
                    "runProgress": {
                        "phase": "created",
                        "runId": "run-123",
                        "runType": "distill",
                        "runState": "queued",
                        "progressPercent": 5,
                        "stepKey": "gather",
                        "stepSummary": "Gather material",
                        "stepStatus": "started",
                        "stepIndex": 1,
                        "stepsTotal": 2,
                        "detailText": "Collecting sources"
                    }
                }),
                "2026-04-05T00:00:01Z",
            ),
            session_message(
                "message-assistant",
                None,
                "assistant_message",
                SessionMessageRole::Assistant,
                "Run complete",
                serde_json::json!({}),
                "2026-04-05T00:00:02Z",
            ),
        ]);

        assert_eq!(timeline.len(), 3);
        assert_eq!(timeline[0].id, "message-user");
        assert_eq!(timeline[1].id, "run-card-run-123");
        assert_eq!(timeline[2].id, "message-assistant");
        assert_eq!(timeline[1].created_at, "2026-04-05T00:00:01Z");
    }

    #[test]
    fn desktop_timeline_mapper_preserves_user_handoff_run_and_completion_order() {
        let timeline = desktop_timeline_from_session_messages(&[
            session_message(
                "message-user",
                None,
                "user_message",
                SessionMessageRole::User,
                "Please generate the brief",
                serde_json::json!({}),
                "2026-04-05T00:00:00Z",
            ),
            session_message(
                "message-handoff",
                None,
                "assistant_message",
                SessionMessageRole::Assistant,
                "I will start a distill run.",
                serde_json::json!({}),
                "2026-04-05T00:00:01Z",
            ),
            session_message(
                "progress-1",
                Some("run-123"),
                "run_progress_message",
                SessionMessageRole::System,
                "run created: run-123 (distill)",
                serde_json::json!({
                    "statusText": "run created: run-123 (distill)",
                    "runProgress": {
                        "phase": "created",
                        "runId": "run-123",
                        "runType": "distill",
                        "runState": "queued",
                        "progressPercent": 10,
                        "stepKey": "gather",
                        "stepSummary": "Gather material",
                        "stepStatus": "started",
                        "stepIndex": 1,
                        "stepsTotal": 2,
                        "detailText": "Collecting sources"
                    }
                }),
                "2026-04-05T00:00:02Z",
            ),
            session_message(
                "message-complete",
                None,
                "assistant_message",
                SessionMessageRole::Assistant,
                "The brief is ready.",
                serde_json::json!({}),
                "2026-04-05T00:00:03Z",
            ),
        ]);

        let ids = timeline.iter().map(|message| message.id.as_str()).collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![
                "message-user",
                "message-handoff",
                "run-card-run-123",
                "message-complete",
            ]
        );
    }

    #[test]
    fn desktop_timeline_mapper_keeps_run_card_when_one_progress_row_is_malformed() {
        let timeline = desktop_timeline_from_session_messages(&[
            session_message(
                "message-user",
                None,
                "user_message",
                SessionMessageRole::User,
                "Run it",
                serde_json::json!({}),
                "2026-04-05T00:00:00Z",
            ),
            session_message(
                "progress-bad",
                Some("run-123"),
                "run_progress_message",
                SessionMessageRole::System,
                "bad progress row",
                serde_json::json!({"statusText": "bad progress row"}),
                "2026-04-05T00:00:01Z",
            ),
            session_message(
                "progress-good",
                Some("run-123"),
                "run_progress_message",
                SessionMessageRole::System,
                "run state: run-123 running (40%)",
                serde_json::json!({
                    "statusText": "run state: run-123 running (40%)",
                    "runProgress": {
                        "phase": "state_changed",
                        "runId": "run-123",
                        "runType": "distill",
                        "runState": "running",
                        "progressPercent": 40,
                        "detailText": "Still working"
                    }
                }),
                "2026-04-05T00:00:02Z",
            ),
        ]);

        assert_eq!(timeline.len(), 2);
        assert_eq!(timeline[0].id, "message-user");
        assert_eq!(timeline[1].id, "run-card-run-123");
        assert_eq!(timeline[1].kind, "run");
        assert_eq!(timeline[1].created_at, "2026-04-05T00:00:01Z");
        let run_meta = timeline[1].run_meta.as_ref().expect("run meta");
        assert_eq!(run_meta.run_id, "run-123");
        assert_eq!(run_meta.state, "running");
        assert_eq!(run_meta.progress_percent, 40);
    }

    #[test]
    fn desktop_timeline_mapper_keeps_one_evolving_step_per_step_key() {
        let timeline = desktop_timeline_from_session_messages(&[
            session_message(
                "progress-1",
                Some("run-123"),
                "run_progress_message",
                SessionMessageRole::System,
                "run step started: run-123 [draft] (20%)",
                serde_json::json!({
                    "statusText": "run step started: run-123 [draft] (20%)",
                    "runProgress": {
                        "phase": "step_started",
                        "runId": "run-123",
                        "runType": "distill",
                        "runState": "running",
                        "progressPercent": 20,
                        "stepKey": "draft",
                        "stepSummary": "Draft answer",
                        "stepStatus": "started",
                        "stepIndex": 1,
                        "stepsTotal": 2,
                        "detailText": "Starting draft"
                    }
                }),
                "2026-04-05T00:00:01Z",
            ),
            session_message(
                "progress-2",
                Some("run-123"),
                "run_progress_message",
                SessionMessageRole::System,
                "run step finished: run-123 [draft] (60%)",
                serde_json::json!({
                    "statusText": "run step finished: run-123 [draft] (60%)",
                    "runProgress": {
                        "phase": "step_finished",
                        "runId": "run-123",
                        "runType": "distill",
                        "runState": "running",
                        "progressPercent": 60,
                        "stepKey": "draft",
                        "stepSummary": "Draft answer",
                        "stepStatus": "completed",
                        "stepIndex": 1,
                        "stepsTotal": 2,
                        "detailText": "Draft done"
                    }
                }),
                "2026-04-05T00:00:02Z",
            ),
        ]);

        assert_eq!(timeline.len(), 1);
        let run_meta = timeline[0].run_meta.as_ref().expect("run meta");
        assert_eq!(run_meta.steps.len(), 1);
        assert_eq!(run_meta.steps[0].key, "draft");
        assert_eq!(run_meta.steps[0].status, "completed");
        assert_eq!(run_meta.steps[0].detail_text.as_deref(), Some("Draft done"));
    }

    #[test]
    fn desktop_timeline_mapper_uses_latest_valid_step_row_by_persisted_chronology() {
        let timeline = desktop_timeline_from_session_messages(&[
            session_message(
                "progress-late",
                Some("run-123"),
                "run_progress_message",
                SessionMessageRole::System,
                "run step finished: run-123 [draft] (60%)",
                serde_json::json!({
                    "statusText": "run step finished: run-123 [draft] (60%)",
                    "runProgress": {
                        "phase": "step_finished",
                        "runId": "run-123",
                        "runType": "distill",
                        "runState": "running",
                        "progressPercent": 60,
                        "stepKey": "draft",
                        "stepSummary": "Draft answer",
                        "stepStatus": "completed",
                        "stepIndex": 1,
                        "stepsTotal": 2,
                        "detailText": "Draft done"
                    }
                }),
                "2026-04-05T00:00:02Z",
            ),
            session_message(
                "progress-early",
                Some("run-123"),
                "run_progress_message",
                SessionMessageRole::System,
                "run step started: run-123 [draft] (20%)",
                serde_json::json!({
                    "statusText": "run step started: run-123 [draft] (20%)",
                    "runProgress": {
                        "phase": "step_started",
                        "runId": "run-123",
                        "runType": "distill",
                        "runState": "running",
                        "progressPercent": 20,
                        "stepKey": "draft",
                        "stepSummary": "Draft answer",
                        "stepStatus": "started",
                        "stepIndex": 1,
                        "stepsTotal": 2,
                        "detailText": "Starting draft"
                    }
                }),
                "2026-04-05T00:00:01Z",
            ),
        ]);

        assert_eq!(timeline.len(), 1);
        let run_meta = timeline[0].run_meta.as_ref().expect("run meta");
        assert_eq!(run_meta.steps.len(), 1);
        assert_eq!(run_meta.steps[0].key, "draft");
        assert_eq!(run_meta.steps[0].status, "completed");
        assert_eq!(run_meta.steps[0].detail_text.as_deref(), Some("Draft done"));
    }

    #[test]
    fn desktop_timeline_mapper_uses_latest_valid_progress_row_by_persisted_chronology() {
        let timeline = desktop_timeline_from_session_messages(&[
            session_message(
                "progress-late",
                Some("run-123"),
                "run_progress_message",
                SessionMessageRole::System,
                "run step finished: run-123 [draft] (100%)",
                serde_json::json!({
                    "statusText": "run step finished: run-123 [draft] (100%)",
                    "runProgress": {
                        "phase": "step_finished",
                        "runId": "run-123",
                        "runType": "distill",
                        "runState": "completed",
                        "progressPercent": 100,
                        "stepKey": "draft",
                        "stepSummary": "Draft answer",
                        "stepStatus": "completed",
                        "stepIndex": 2,
                        "stepsTotal": 2,
                        "detailText": "Draft complete"
                    }
                }),
                "2026-04-05T00:00:03Z",
            ),
            session_message(
                "progress-early",
                Some("run-123"),
                "run_progress_message",
                SessionMessageRole::System,
                "run created: run-123 (distill)",
                serde_json::json!({
                    "statusText": "run created: run-123 (distill)",
                    "runProgress": {
                        "phase": "created",
                        "runId": "run-123",
                        "runType": "distill",
                        "runState": "queued",
                        "progressPercent": 10,
                        "stepKey": "gather",
                        "stepSummary": "Gather material",
                        "stepStatus": "started",
                        "stepIndex": 1,
                        "stepsTotal": 2,
                        "detailText": "Collecting sources"
                    }
                }),
                "2026-04-05T00:00:01Z",
            ),
        ]);

        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline[0].created_at, "2026-04-05T00:00:01Z");
        let run_meta = timeline[0].run_meta.as_ref().expect("run meta");
        assert_eq!(run_meta.state, "completed");
        assert_eq!(run_meta.progress_percent, 100);
        assert_eq!(run_meta.current_step_key.as_deref(), Some("draft"));
        assert_eq!(run_meta.step_summary.as_deref(), Some("Draft answer"));
    }

    #[test]
    fn structured_timeline_command_payload_contains_messages_tools_and_runs() {
        let timeline = list_session_messages_structured_payload(&[
            session_message(
                "message-user",
                None,
                "user_message",
                SessionMessageRole::User,
                "Please summarize the attachment",
                serde_json::json!({
                    "attachments": [
                        {
                            "name": "notes.md",
                            "size": 2048
                        }
                    ]
                }),
                "2026-04-05T00:00:00Z",
            ),
            session_message(
                "message-tool",
                None,
                "tool_result_message",
                SessionMessageRole::System,
                "Attachment excerpt: hello",
                serde_json::json!({
                    "tool_name": "read_attachment_excerpt",
                    "arguments": {
                        "attachment_index": 0,
                        "max_chars": 400
                    }
                }),
                "2026-04-05T00:00:01Z",
            ),
            session_message(
                "progress-1",
                Some("run-123"),
                "run_progress_message",
                SessionMessageRole::System,
                "run created: run-123 (distill)",
                serde_json::json!({
                    "statusText": "run created: run-123 (distill)",
                    "runProgress": {
                        "phase": "created",
                        "runId": "run-123",
                        "runType": "distill",
                        "runState": "queued",
                        "progressPercent": 10,
                        "stepKey": "gather",
                        "stepSummary": "Gather material",
                        "stepStatus": "started",
                        "stepIndex": 1,
                        "stepsTotal": 2,
                        "detailText": "Collecting sources"
                    }
                }),
                "2026-04-05T00:00:02Z",
            ),
        ]);

        assert_eq!(timeline.len(), 3);
        assert_eq!(timeline[0].id, "message-user");
        assert_eq!(timeline[0].kind, "message");
        assert_eq!(timeline[0].attachments.len(), 1);
        assert_eq!(timeline[1].id, "message-tool");
        assert_eq!(timeline[1].kind, "tool");
        assert_eq!(timeline[1].summary.as_deref(), Some("read_attachment_excerpt · success"));
        assert!(
            timeline[1]
                .details
                .as_deref()
                .expect("structured tool details")
                .contains("Attachment excerpt: hello")
        );
        assert_eq!(timeline[2].id, "run-card-run-123");
        assert_eq!(timeline[2].kind, "run");
        assert_eq!(
            timeline[2]
                .run_meta
                .as_ref()
                .and_then(|meta| meta.run_type.as_deref()),
            Some("distill")
        );
    }

    #[test]
    fn structured_tool_card_contains_status_arguments_and_result_body() {
        let timeline = desktop_timeline_from_session_messages(&[session_message(
            "message-tool-error",
            None,
            "tool_result_message",
            SessionMessageRole::System,
            "Tool failed: file missing",
            serde_json::json!({
                "tool_name": "read_file",
                "arguments": {
                    "path": "missing.txt"
                }
            }),
            "2026-04-05T00:00:03Z",
        )]);

        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline[0].content, "Tool failed: file missing");
        assert_eq!(timeline[0].summary.as_deref(), Some("read_file · failed"));
        let details = timeline[0].details.as_deref().expect("tool details");
        assert!(details.contains("tool: read_file"));
        assert!(details.contains("status: failed"));
        assert!(details.contains("missing.txt"));
        assert!(details.contains("Tool failed: file missing"));
    }

    #[test]
    fn structured_run_card_uses_only_supported_run_states() {
        let timeline = desktop_timeline_from_session_messages(&[session_message(
            "progress-unsupported-state",
            Some("run-unsupported-state"),
            "run_progress_message",
            SessionMessageRole::System,
            "run state changed",
            serde_json::json!({
                "statusText": "run state changed",
                "runProgress": {
                    "phase": "state_changed",
                    "runId": "run-unsupported-state",
                    "runType": "distill",
                    "runState": "created",
                    "progressPercent": 5
                }
            }),
            "2026-04-05T00:00:04Z",
        )]);

        let run_state = timeline[0]
            .run_meta
            .as_ref()
            .map(|meta| meta.state.as_str())
            .expect("run state");
        assert_eq!(run_state, "queued");
    }

    #[test]
    fn structured_step_meta_uses_only_supported_step_statuses() {
        let timeline = desktop_timeline_from_session_messages(&[
            session_message(
                "progress-step-status",
                Some("run-step-status"),
                "run_progress_message",
                SessionMessageRole::System,
                "run step finished",
                serde_json::json!({
                    "statusText": "run step finished",
                    "runProgress": {
                        "phase": "step_finished",
                        "runId": "run-step-status",
                        "runType": "distill",
                        "runState": "running",
                        "progressPercent": 80,
                        "stepKey": "draft",
                        "stepSummary": "Draft answer",
                        "stepStatus": "finished",
                        "stepIndex": 2,
                        "stepsTotal": 3,
                        "detailText": "Draft complete"
                    }
                }),
                "2026-04-05T00:00:05Z",
            ),
        ]);

        let run_meta = timeline[0].run_meta.as_ref().expect("run meta");
        assert_eq!(run_meta.step_status.as_deref(), Some("completed"));
        assert_eq!(run_meta.steps[0].status, "completed");
    }

    #[test]
    fn live_and_structured_run_states_share_the_same_supported_values() {
        let timeline = desktop_timeline_from_session_messages(&[
            session_message(
                "progress-queued",
                Some("run-queued"),
                "run_progress_message",
                SessionMessageRole::System,
                "run queued",
                serde_json::json!({
                    "statusText": "run queued",
                    "runProgress": {
                        "phase": "created",
                        "runId": "run-queued",
                        "runType": "distill",
                        "runState": "queued",
                        "progressPercent": 10,
                        "stepKey": "gather",
                        "stepSummary": "Gather material",
                        "stepStatus": "started",
                        "stepIndex": 1,
                        "stepsTotal": 2
                    }
                }),
                "2026-04-05T00:00:06Z",
            ),
        ]);

        let run_meta = timeline[0].run_meta.as_ref().expect("run meta");
        assert!(matches!(run_meta.state.as_str(), "queued" | "pending" | "running" | "completed" | "failed"));
        assert!(matches!(
            run_meta.step_status.as_deref().expect("step status"),
            "started" | "pending" | "running" | "completed" | "failed"
        ));
        assert!(matches!(run_meta.steps[0].status.as_str(), "started" | "pending" | "running" | "completed" | "failed"));
    }

    #[test]
    fn emitted_live_run_progress_uses_supported_run_and_step_states() {
        let normalized = normalize_live_run_progress_update(&RunProgressUpdate {
            phase: RunProgressPhase::StepFinished,
            run_id: "run-live".to_string(),
            run_type: "distill".to_string(),
            run_state: LiveRunState::Pending,
            progress_percent: Some(80),
            step_key: Some("draft".to_string()),
            step_summary: Some("Draft answer".to_string()),
            step_status: Some(LiveRunStepStatus::Pending),
            step_index: Some(2),
            steps_total: Some(3),
            detail_text: Some("Draft complete".to_string()),
        });

        assert_eq!(normalized.run_state, LiveRunState::Pending);
        assert_eq!(normalized.step_status, Some(LiveRunStepStatus::Pending));
    }

    #[test]
    fn emitted_live_run_progress_preserves_started_step_status() {
        let normalized = normalize_live_run_progress_update(&RunProgressUpdate {
            phase: RunProgressPhase::StepStarted,
            run_id: "run-live-started".to_string(),
            run_type: "distill".to_string(),
            run_state: LiveRunState::Running,
            progress_percent: Some(25),
            step_key: Some("gather".to_string()),
            step_summary: Some("Gather material".to_string()),
            step_status: Some(LiveRunStepStatus::Started),
            step_index: Some(1),
            steps_total: Some(3),
            detail_text: Some("Collecting sources".to_string()),
        });

        assert_eq!(normalized.run_state, LiveRunState::Running);
        assert_eq!(normalized.step_status, Some(LiveRunStepStatus::Started));
    }

    #[test]
    fn emitted_tool_started_includes_structured_tool_event() {
        let event = build_live_tool_event(
            "tool-call-1",
            "read_file",
            LiveToolStatus::Started,
            Some("{\"path\":\"notes.md\"}".to_string()),
            None,
        );

        assert_eq!(event.tool_call_id, "tool-call-1");
        assert_eq!(event.tool_name, "read_file");
        assert_eq!(event.status, LiveToolStatus::Started);
        assert_eq!(event.arguments_text.as_deref(), Some("{\"path\":\"notes.md\"}"));
        assert_eq!(event.result_text, None);
        assert_eq!(event.content, "Tool result unavailable.");
        assert_eq!(event.details, "tool: read_file\nstatus: started\narguments: {\"path\":\"notes.md\"}\n\nresult:\nTool result unavailable.");
    }

    #[test]
    fn emitted_tool_finished_includes_structured_tool_event() {
        let event = build_live_tool_event(
            "tool-call-2",
            "read_file",
            LiveToolStatus::Failed,
            Some("{\"path\":\"missing.md\"}".to_string()),
            Some("file missing".to_string()),
        );

        assert_eq!(event.status, LiveToolStatus::Failed);
        assert_eq!(event.content, "file missing");
        assert_eq!(event.result_text.as_deref(), Some("file missing"));
        assert!(event.summary.contains("read_file"));
        assert!(event.details.contains("status: failed"));
    }

    #[test]
    fn persisted_and_live_tool_presentations_match_for_failed_tool_results() {
        let timeline = desktop_timeline_from_session_messages(&[session_message(
            "message-tool-failed",
            None,
            "tool_result_message",
            SessionMessageRole::System,
            "file missing",
            serde_json::json!({
                "tool_name": "read_file",
                "arguments": {
                    "path": "missing.md"
                },
                "status": "failed"
            }),
            "2026-04-05T00:00:07Z",
        )]);
        let persisted = &timeline[0];
        let live = build_live_tool_event(
            "tool-call-parity-failed",
            "read_file",
            LiveToolStatus::Failed,
            Some("{\"path\":\"missing.md\"}".to_string()),
            Some("file missing".to_string()),
        );

        assert_eq!(persisted.summary.as_deref(), Some(live.summary.as_str()));
        assert_eq!(persisted.details.as_deref(), Some(live.details.as_str()));
        assert_eq!(persisted.content, live.result_text.expect("live result text"));
    }

    #[test]
    fn persisted_and_live_tool_presentations_share_unknown_result_text() {
        let timeline = desktop_timeline_from_session_messages(&[session_message(
            "message-tool-missing-result",
            None,
            "tool_result_message",
            SessionMessageRole::System,
            "",
            serde_json::json!({
                "tool_name": "read_file",
                "arguments": {
                    "path": "notes.md"
                },
                "status": "succeeded"
            }),
            "2026-04-05T00:00:08Z",
        )]);
        let persisted = &timeline[0];
        let live = build_live_tool_event(
            "tool-call-parity-unknown",
            "read_file",
            LiveToolStatus::Succeeded,
            Some("{\"path\":\"notes.md\"}".to_string()),
            None,
        );

        assert_eq!(persisted.summary.as_deref(), Some(live.summary.as_str()));
        assert_eq!(persisted.details.as_deref(), Some(live.details.as_str()));
        assert_eq!(persisted.content, "Tool result unavailable.");
        assert_eq!(persisted.content, live.details.lines().last().expect("unknown result line"));
    }

    #[test]
    fn emitted_run_started_includes_structured_run_event() {
        let event = build_live_run_event(
            "run-123",
            Some("distill".to_string()),
            LiveRunState::Queued,
            Some(10),
            Some("Queued for execution".to_string()),
            Some(RunProgressPhase::Created),
            None,
            None,
            Some("run created: run-123 (distill)".to_string()),
        );

        assert_eq!(event.run_id, "run-123");
        assert_eq!(event.run_type.as_deref(), Some("distill"));
        assert_eq!(event.state, LiveRunState::Queued);
        assert_eq!(event.progress_percent, Some(10));
    }

    #[test]
    fn emitted_run_finished_includes_structured_run_event() {
        let event = build_live_run_event(
            "run-456",
            Some("distill".to_string()),
            LiveRunState::Failed,
            Some(100),
            Some("Run failed at draft step".to_string()),
            Some(RunProgressPhase::StateChanged),
            Some("draft".to_string()),
            Some("Draft answer".to_string()),
            Some("run state: run-456 failed (100%) - Run failed at draft step".to_string()),
        );

        assert_eq!(event.state, LiveRunState::Failed);
        assert_eq!(event.detail_text.as_deref(), Some("Run failed at draft step"));
    }

    #[test]
    fn persisted_and_live_run_presentations_match_for_created_run_semantics() {
        let timeline = desktop_timeline_from_session_messages(&[session_message(
            "progress-created",
            Some("run-bridge-created"),
            "run_progress_message",
            SessionMessageRole::System,
            "run created: run-bridge-created (distill)",
            serde_json::json!({
                "statusText": "run created: run-bridge-created (distill)",
                "runProgress": {
                    "phase": "created",
                    "runId": "run-bridge-created",
                    "runType": "distill",
                    "runState": "queued",
                    "progressPercent": 10,
                    "detailText": "Queued for execution"
                }
            }),
            "2026-04-05T00:00:09Z",
        )]);
        let persisted = &timeline[0];
        let live = serde_json::to_value(build_live_run_event(
            "run-bridge-created",
            Some("distill".to_string()),
            LiveRunState::Queued,
            Some(10),
            Some("Queued for execution".to_string()),
            Some(RunProgressPhase::Created),
            None,
            None,
            Some("run created: run-bridge-created (distill)".to_string()),
        ))
        .expect("live run event json");

        assert_eq!(persisted.content, live["content"].as_str().expect("live content"));
        assert_eq!(persisted.summary.as_deref(), live["summary"].as_str());
        assert_eq!(persisted.details.as_deref(), live["details"].as_str());
    }

    #[test]
    fn persisted_and_live_run_presentations_match_latest_meaningful_status_text_semantics() {
        let timeline = desktop_timeline_from_session_messages(&[
            session_message(
                "progress-started",
                Some("run-bridge-progress"),
                "run_progress_message",
                SessionMessageRole::System,
                "run step started: run-bridge-progress [draft] (40%) - Starting draft",
                serde_json::json!({
                    "statusText": "run step started: run-bridge-progress [draft] (40%) - Starting draft",
                    "runProgress": {
                        "phase": "step_started",
                        "runId": "run-bridge-progress",
                        "runType": "distill",
                        "runState": "running",
                        "progressPercent": 40,
                        "stepKey": "draft",
                        "stepSummary": "Draft answer",
                        "stepStatus": "started",
                        "stepIndex": 1,
                        "stepsTotal": 2,
                        "detailText": "Starting draft"
                    }
                }),
                "2026-04-05T00:00:10Z",
            ),
            session_message(
                "progress-finished",
                Some("run-bridge-progress"),
                "run_progress_message",
                SessionMessageRole::System,
                "run step finished: run-bridge-progress [draft] (80%) - Draft complete",
                serde_json::json!({
                    "statusText": "run step finished: run-bridge-progress [draft] (80%) - Draft complete",
                    "runProgress": {
                        "phase": "step_finished",
                        "runId": "run-bridge-progress",
                        "runType": "distill",
                        "runState": "running",
                        "progressPercent": 80,
                        "stepKey": "draft",
                        "stepSummary": "Draft answer",
                        "stepStatus": "completed",
                        "stepIndex": 1,
                        "stepsTotal": 2,
                        "detailText": "Draft complete"
                    }
                }),
                "2026-04-05T00:00:11Z",
            ),
        ]);
        let persisted = &timeline[0];
        let live = serde_json::to_value(build_live_run_event(
            "run-bridge-progress",
            Some("distill".to_string()),
            LiveRunState::Running,
            Some(80),
            Some("Draft complete".to_string()),
            Some(RunProgressPhase::StepFinished),
            Some("draft".to_string()),
            Some("Draft answer".to_string()),
            Some("run step finished: run-bridge-progress [draft] (80%) - Draft complete".to_string()),
        ))
        .expect("live run event json");

        assert_eq!(
            persisted.content,
            "run step finished: run-bridge-progress [draft] (80%) - Draft complete"
        );
        assert_eq!(persisted.content, live["content"].as_str().expect("live content"));
        assert_eq!(persisted.summary.as_deref(), live["summary"].as_str());
        assert_eq!(persisted.details.as_deref(), live["details"].as_str());
    }

    #[test]
    fn live_run_progress_normalizes_legacy_aliases_to_supported_enum_values() {
        let normalized = normalize_live_run_progress_update(&RunProgressUpdate {
            phase: RunProgressPhase::StepFinished,
            run_id: "run-live-alias".to_string(),
            run_type: "distill".to_string(),
            run_state: LiveRunState::Pending,
            progress_percent: Some(90),
            step_key: Some("draft".to_string()),
            step_summary: Some("Draft answer".to_string()),
            step_status: Some(LiveRunStepStatus::Pending),
            step_index: Some(2),
            steps_total: Some(3),
            detail_text: Some("Draft complete".to_string()),
        });

        assert_eq!(normalized.run_state, LiveRunState::Pending);
        assert_eq!(normalized.step_status, Some(LiveRunStepStatus::Pending));
    }

    #[test]
    fn emitted_run_progress_event_keeps_run_progress_authoritative_when_run_event_conflicts() {
        let event = build_chat_stream_event_with_run_progress(
            "request-1",
            "session-1",
            ChatStreamPhase::RunProgress,
            None,
            None,
            None,
            Some("run state changed".to_string()),
            None,
            None,
            None,
            Some("run-authority".to_string()),
            None,
            normalize_live_run_progress_update(&RunProgressUpdate {
                phase: RunProgressPhase::StateChanged,
                run_id: "run-authority".to_string(),
                run_type: "distill".to_string(),
                run_state: LiveRunState::Failed,
                progress_percent: Some(100),
                step_key: None,
                step_summary: None,
                step_status: None,
                step_index: None,
                steps_total: None,
                detail_text: Some("Run failed at draft step".to_string()),
            }),
        );

        let run_progress = event.run_progress.as_ref().expect("run progress");
        let run_event = event.run_event.as_ref().expect("run event");

        assert_eq!(run_progress.run_state, LiveRunState::Failed);
        assert_eq!(run_event.state, LiveRunState::Failed);
        assert_eq!(run_event.detail_text.as_deref(), Some("Run failed at draft step"));
    }

    #[test]
    fn emitted_run_progress_event_preserves_missing_step_status_when_no_step_context_exists() {
        let event = build_chat_stream_event_with_run_progress(
            "request-2",
            "session-1",
            ChatStreamPhase::RunProgress,
            None,
            None,
            None,
            Some("run state changed".to_string()),
            None,
            None,
            None,
            Some("run-no-step".to_string()),
            None,
            normalize_live_run_progress_update(&RunProgressUpdate {
                phase: RunProgressPhase::StateChanged,
                run_id: "run-no-step".to_string(),
                run_type: "distill".to_string(),
                run_state: LiveRunState::Running,
                progress_percent: Some(55),
                step_key: None,
                step_summary: None,
                step_status: None,
                step_index: None,
                steps_total: None,
                detail_text: Some("Still running".to_string()),
            }),
        );

        assert_eq!(event.run_progress.as_ref().expect("run progress").step_status, None);
    }
}

fn indent_block(text: &str) -> String {
    text.lines()
        .map(|line| format!("  {}", line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_live_run_state(value: Option<&str>) -> LiveRunState {
    match value.unwrap_or_default().trim().to_lowercase().as_str() {
        "queued" | "created" => LiveRunState::Queued,
        "running" => LiveRunState::Running,
        "completed" | "finished" => LiveRunState::Completed,
        "failed" | "error" => LiveRunState::Failed,
        _ => LiveRunState::Pending,
    }
}

fn normalize_live_run_step_status(value: Option<&str>) -> Option<LiveRunStepStatus> {
    value.map(|status| match status.trim().to_lowercase().as_str() {
        "started" => LiveRunStepStatus::Started,
        "running" => LiveRunStepStatus::Running,
        "completed" | "finished" => LiveRunStepStatus::Completed,
        "failed" | "error" => LiveRunStepStatus::Failed,
        _ => LiveRunStepStatus::Pending,
    })
}

fn live_run_state_label(value: &LiveRunState) -> &'static str {
    match value {
        LiveRunState::Queued => "queued",
        LiveRunState::Pending => "pending",
        LiveRunState::Running => "running",
        LiveRunState::Completed => "completed",
        LiveRunState::Failed => "failed",
    }
}

fn live_run_step_status_label(value: &LiveRunStepStatus) -> &'static str {
    match value {
        LiveRunStepStatus::Started => "started",
        LiveRunStepStatus::Pending => "pending",
        LiveRunStepStatus::Running => "running",
        LiveRunStepStatus::Completed => "completed",
        LiveRunStepStatus::Failed => "failed",
    }
}

fn build_live_tool_event(
    tool_call_id: &str,
    tool_name: &str,
    status: LiveToolStatus,
    arguments_text: Option<String>,
    result_text: Option<String>,
) -> LiveToolEvent {
    let presentation = build_tool_presentation(
        tool_name,
        status.clone(),
        arguments_text.as_deref(),
        result_text.as_deref(),
    );

    LiveToolEvent {
        tool_call_id: tool_call_id.to_string(),
        tool_name: tool_name.to_string(),
        status,
        content: presentation.content,
        arguments_text,
        result_text,
        summary: presentation.summary,
        details: presentation.details,
    }
}

fn build_live_run_event(
    run_id: &str,
    run_type: Option<String>,
    state: LiveRunState,
    progress_percent: Option<u8>,
    detail_text: Option<String>,
    phase: Option<RunProgressPhase>,
    step_key: Option<String>,
    step_summary: Option<String>,
    status_text: Option<String>,
) -> LiveRunEvent {
    let presentation = build_run_presentation(
        run_id,
        run_type.as_deref(),
        &state,
        progress_percent,
        detail_text.as_deref(),
        phase.as_ref(),
        step_key.as_deref(),
        step_summary.as_deref(),
        status_text.as_deref(),
    );

    LiveRunEvent {
        run_id: run_id.to_string(),
        run_type,
        state,
        progress_percent,
        detail_text,
        content: presentation.content,
        summary: presentation.summary,
        details: presentation.details,
    }
}

fn build_chat_stream_event(
    request_id: &str,
    session_id: &str,
    phase: ChatStreamPhase,
    action_type: Option<String>,
    intent: Option<String>,
    chunk_text: Option<String>,
    status_text: Option<String>,
    assistant_text: Option<String>,
    timeline_text: Option<String>,
    error_text: Option<String>,
    created_run_id: Option<String>,
    tool_event: Option<LiveToolEvent>,
    run_event: Option<LiveRunEvent>,
) -> ChatStreamEvent {
    ChatStreamEvent {
        request_id: request_id.to_string(),
        session_id: session_id.to_string(),
        phase,
        action_type,
        intent,
        chunk_text,
        status_text,
        assistant_text,
        timeline_text,
        error_text,
        created_run_id,
        tool_event,
        run_event,
        run_progress: None,
    }
}

fn build_chat_stream_event_with_run_progress(
    request_id: &str,
    session_id: &str,
    phase: ChatStreamPhase,
    action_type: Option<String>,
    intent: Option<String>,
    chunk_text: Option<String>,
    status_text: Option<String>,
    assistant_text: Option<String>,
    timeline_text: Option<String>,
    error_text: Option<String>,
    created_run_id: Option<String>,
    tool_event: Option<LiveToolEvent>,
    run_progress: RunProgressUpdate,
) -> ChatStreamEvent {
    let authoritative_run_event = Some(build_live_run_event(
        &run_progress.run_id,
        Some(run_progress.run_type.clone()),
        run_progress.run_state.clone(),
        run_progress.progress_percent,
        run_progress.detail_text.clone(),
        Some(run_progress.phase.clone()),
        run_progress.step_key.clone(),
        run_progress.step_summary.clone(),
        status_text.clone(),
    ));

    ChatStreamEvent {
        request_id: request_id.to_string(),
        session_id: session_id.to_string(),
        phase,
        action_type,
        intent,
        chunk_text,
        status_text,
        assistant_text,
        timeline_text,
        error_text,
        created_run_id,
        tool_event,
        run_event: authoritative_run_event,
        run_progress: Some(run_progress),
    }
}

fn progress_status_text(update: &RunProgressUpdate) -> String {
    let percent = update
        .progress_percent
        .map(|value| format!("{}%", value))
        .unwrap_or_else(|| "n/a".to_string());
    let step_key = update.step_key.as_deref().unwrap_or("run");
    let detail = update.detail_text.as_deref().unwrap_or("");
    match update.phase {
        RunProgressPhase::Created => format!(
            "run created: {} ({})",
            update.run_id, update.run_type
        ),
        RunProgressPhase::StateChanged => format!(
            "run state: {} {} ({}){}",
            update.run_id,
            live_run_state_label(&update.run_state),
            percent,
            if detail.is_empty() {
                "".to_string()
            } else {
                format!(" - {}", detail)
            }
        ),
        RunProgressPhase::StepStarted => format!(
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
        RunProgressPhase::StepFinished => format!(
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

fn stream_phase_from_progress(update: &RunProgressUpdate) -> ChatStreamPhase {
    match update.phase {
        RunProgressPhase::Created => ChatStreamPhase::RunCreated,
        RunProgressPhase::StateChanged => ChatStreamPhase::RunProgress,
        RunProgressPhase::StepStarted => ChatStreamPhase::RunStepStarted,
        RunProgressPhase::StepFinished => ChatStreamPhase::RunStepFinished,
    }
}

fn normalize_live_run_progress_update(update: &RunProgressUpdate) -> RunProgressUpdate {
    RunProgressUpdate {
        phase: update.phase.clone(),
        run_id: update.run_id.clone(),
        run_type: update.run_type.clone(),
        run_state: normalize_live_run_state(Some(live_run_state_label(&update.run_state))),
        progress_percent: update.progress_percent,
        step_key: update.step_key.clone(),
        step_summary: update.step_summary.clone(),
        step_status: normalize_live_run_step_status(
            update.step_status.as_ref().map(live_run_step_status_label),
        ),
        step_index: update.step_index,
        steps_total: update.steps_total,
        detail_text: update.detail_text.clone(),
    }
}

fn emit_chat_stream_event(app: &tauri::AppHandle, event: &ChatStreamEvent) -> Result<(), String> {
    app.emit("distilllab://chat-stream", event)
        .map_err(|e| e.to_string())
}

fn emit_execution_result_stream(
    app: &tauri::AppHandle,
    request_id: &str,
    result: &SessionMessageExecutionResult,
) -> Result<(), String> {
    emit_chat_stream_event(
        app,
        &build_chat_stream_event(
            request_id,
            &result.session_id,
            ChatStreamPhase::DecisionReady,
            Some(result.action_type.clone()),
            Some(result.intent.clone()),
            None,
            Some(format!(
                "decision: intent={} action={}{}",
                result.intent,
                result.action_type,
                result
                    .created_run_id
                    .as_ref()
                    .map(|run_id| format!(" run={}", run_id))
                    .unwrap_or_default()
            )),
            if result.action_type == "create_run" {
                Some(result.assistant_text.clone())
            } else {
                None
            },
            None,
            None,
            result.created_run_id.clone(),
            None,
            None,
        ),
    )?;

    if let Some(tool_name) = &result.tool_name {
        let tool_started_event = build_live_tool_event(
            &format!("tool-call-{}", request_id),
            tool_name,
            LiveToolStatus::Started,
            None,
            None,
        );
        emit_chat_stream_event(
            app,
            &build_chat_stream_event(
                request_id,
                &result.session_id,
                ChatStreamPhase::ToolStarted,
                Some(result.action_type.clone()),
                Some(result.intent.clone()),
                None,
                Some(format!("tool started: {}", tool_name)),
                None,
                None,
                None,
                result.created_run_id.clone(),
                Some(tool_started_event),
                None,
            ),
        )?;

        let tool_finished_status = match result.tool_ok {
            Some(true) => LiveToolStatus::Succeeded,
            Some(false) => LiveToolStatus::Failed,
            None => LiveToolStatus::Started,
        };
        let tool_finished_event = build_live_tool_event(
            &format!("tool-call-{}", request_id),
            tool_name,
            tool_finished_status,
            None,
            result.tool_summary.clone(),
        );
        emit_chat_stream_event(
            app,
            &build_chat_stream_event(
                request_id,
                &result.session_id,
                ChatStreamPhase::ToolFinished,
                Some(result.action_type.clone()),
                Some(result.intent.clone()),
                None,
                Some(match result.tool_ok {
                    Some(true) => format!(
                        "tool succeeded: {}",
                        result
                            .tool_summary
                            .clone()
                            .unwrap_or_else(|| tool_name.clone())
                    ),
                    Some(false) => format!(
                        "tool failed: {}",
                        result
                            .tool_summary
                            .clone()
                            .unwrap_or_else(|| tool_name.clone())
                    ),
                    None => format!("tool finished: {}", tool_name),
                }),
                None,
                None,
                None,
                result.created_run_id.clone(),
                Some(tool_finished_event),
                None,
            ),
        )?;
    }

    if let Some(run_id) = &result.created_run_id {
        let run_started_event = build_live_run_event(
            run_id,
            None,
            LiveRunState::Queued,
            None,
            None,
            Some(RunProgressPhase::Created),
            None,
            None,
            Some(format!("run started: {}", run_id)),
        );
        emit_chat_stream_event(
            app,
            &build_chat_stream_event(
                request_id,
                &result.session_id,
                ChatStreamPhase::RunStarted,
                Some(result.action_type.clone()),
                Some(result.intent.clone()),
                None,
                Some(format!("run started: {}", run_id)),
                None,
                None,
                None,
                result.created_run_id.clone(),
                None,
                Some(run_started_event),
            ),
        )?;

        let run_finished_event = build_live_run_event(
            run_id,
            None,
            result
                .run_status
                .as_deref()
                .map(|status| normalize_live_run_state(Some(status)))
                .unwrap_or(LiveRunState::Completed),
            None,
            result.run_status.clone(),
            Some(RunProgressPhase::StateChanged),
            None,
            None,
            Some(match result.run_status.as_deref() {
                Some(status) => format!("run {} status: {}", run_id, status),
                None => format!("run {} finished", run_id),
            }),
        );
        emit_chat_stream_event(
            app,
            &build_chat_stream_event(
                request_id,
                &result.session_id,
                ChatStreamPhase::RunFinished,
                Some(result.action_type.clone()),
                Some(result.intent.clone()),
                None,
                Some(match result.run_status.as_deref() {
                    Some(status) => format!("run {} status: {}", run_id, status),
                    None => format!("run {} finished", run_id),
                }),
                None,
                None,
                None,
                result.created_run_id.clone(),
                None,
                Some(run_finished_event),
            ),
        )?;
    }

    emit_chat_stream_event(
        app,
        &build_chat_stream_event(
            request_id,
            &result.session_id,
            ChatStreamPhase::Completed,
            Some(result.action_type.clone()),
            Some(result.intent.clone()),
            None,
            None,
            Some(result.assistant_text.clone()),
            Some(result.timeline_text.clone()),
            None,
            result.created_run_id.clone(),
            None,
            None,
        ),
    )?;

    Ok(())
}

fn emit_run_progress_stream(
    app: &tauri::AppHandle,
    request_id: &str,
    session_id: &str,
    update: &RunProgressUpdate,
) -> Result<(), String> {
    let normalized_update = normalize_live_run_progress_update(update);
    let phase = stream_phase_from_progress(&normalized_update);
    let status_text = progress_status_text(&normalized_update);

    emit_chat_stream_event(
        app,
        &build_chat_stream_event_with_run_progress(
            request_id,
            session_id,
            phase,
            None,
            None,
            None,
            Some(status_text),
            None,
            None,
            None,
            Some(normalized_update.run_id.clone()),
            None,
            normalized_update,
        ),
    )
}

fn format_app_config_text(config_json: &str) -> Result<String, String> {
    let value: serde_json::Value = serde_json::from_str(config_json).map_err(|e| e.to_string())?;
    let current_provider = value
        .get("distilllab")
        .and_then(|v| v.get("currentProvider"))
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    let current_model = value
        .get("distilllab")
        .and_then(|v| v.get("currentModel"))
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    let providers = value
        .get("provider")
        .and_then(|v| v.as_object())
        .map(|map| map.keys().cloned().collect::<Vec<_>>().join(", "))
        .unwrap_or_else(|| "none".to_string());

    Ok([
        format!("current provider: {}", current_provider),
        format!("current model: {}", current_model),
        format!("providers: {}", providers),
    ]
    .join("\n"))
}

fn format_provider_test_text(
    provider_id: &str,
    model_id: &str,
    status: &str,
    message: &str,
) -> String {
    [
        format!("provider: {}", provider_id),
        format!("model: {}", model_id),
        format!("status: {}", status),
        format!("message: {}", message),
    ]
    .join("\n")
}

#[cfg(test)]
fn format_llm_debug_comparison_text(raw_output: &str, decision: &SessionAgentDecision) -> String {
    [
        "Raw LLM Output".to_string(),
        raw_output.to_string(),
        String::new(),
        "Parsed Decision".to_string(),
        format_session_agent_decision_text(decision),
    ]
    .join("\n")
}

fn load_or_create_app_config() -> Result<(std::path::PathBuf, AppConfig), String> {
    let config_path = default_app_config_path().map_err(|e| e.to_string())?;

    match load_app_config_from_path(&config_path) {
        Ok(config) => Ok((config_path, config)),
        Err(_) => {
            let mut config = AppConfig::default();
            config.schema = Some("https://opencode.ai/config.json".to_string());
            Ok((config_path, config))
        }
    }
}

fn default_opencode_config_path() -> Result<std::path::PathBuf, String> {
    let home_dir = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .map_err(|e| e.to_string())?;

    Ok(home_dir.join(".config/opencode/opencode.json"))
}

fn build_provider_entry_from_form(form: &ConfigBarForm) -> Result<ProviderConfigEntry, String> {
    if !form.raw_provider_json.trim().is_empty() {
        return serde_json::from_str::<ProviderConfigEntry>(&form.raw_provider_json)
            .map_err(|e| e.to_string());
    }

    let model_key = form.current_model.trim();
    if model_key.is_empty() {
        return Err("current model is required".to_string());
    }

    let provider = ProviderConfigEntry {
        npm: Some(form.provider_npm.trim().to_string()),
        name: form.provider_name.trim().to_string(),
        options: ProviderOptions {
            base_url: Some(form.base_url.trim().to_string()),
            api_key: form.api_key.clone().map(|value| value.trim().to_string()),
        },
        models: std::collections::BTreeMap::from([(
            model_key.to_string(),
            ModelConfigEntry {
                name: model_key.to_string(),
                ..Default::default()
            },
        )]),
    };

    Ok(provider)
}

#[tauri::command]
fn load_canvas_global_view(project_id: Option<String>) -> Result<CanvasGlobalViewDto, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    runtime::load_canvas_global_view(&runtime, project_id.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_canvas_object_detail(
    object_type: String,
    object_id: String,
    project_id: Option<String>,
) -> Result<CanvasDetailViewDto, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    runtime::load_canvas_detail_view(&runtime, &object_type, &object_id, project_id.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_demo_run() -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let run = runtime::create_demo_run(&runtime).map_err(|e| e.to_string())?;
    Ok(format!("created run: {} ({:?})", run.id, run.run_type))
}

#[tauri::command]
fn create_demo_source() -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let source = runtime::create_demo_source(&runtime).map_err(|e| e.to_string())?;
    Ok(format!(
        "created source: {} ({:?})",
        source.id, source.source_type
    ))
}

#[tauri::command]
fn create_demo_session() -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let session = runtime::create_demo_session(&runtime).map_err(|e| e.to_string())?;
    Ok(format!(
        "created session: {} [{}]",
        session.id,
        session.status.as_str()
    ))
}

#[tauri::command]
fn create_session_command() -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let session = create_session(&runtime).map_err(|e| e.to_string())?;
    Ok(format!(
        "created session: {} [{}]",
        session.id,
        session.status.as_str()
    ))
}

#[tauri::command]
fn list_sources() -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let sources = runtime::list_sources(&runtime).map_err(|e| e.to_string())?;

    if sources.is_empty() {
        return Ok("no sources found".to_string());
    }

    let summary = sources
        .iter()
        .map(|source| {
            format!(
                "{} [{}] {}",
                source.id,
                source.source_type.as_str(),
                source.title
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(summary)
}

#[tauri::command]
fn list_sessions() -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let sessions = runtime::list_sessions(&runtime).map_err(|e| e.to_string())?;

    if sessions.is_empty() {
        return Ok("no sessions found".to_string());
    }

    let summary = sessions
        .iter()
        .map(|session| {
            format!(
                "{} [{}] {}",
                session.id,
                session.status.as_str(),
                session.title
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(summary)
}

#[tauri::command]
fn list_session_selector_options() -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let sessions = runtime::list_sessions(&runtime).map_err(|e| e.to_string())?;
    let options = sessions
        .iter()
        .map(|session| SessionSelectorOption {
            session_id: session.id.clone(),
            title: session
                .manual_title
                .clone()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| session.title.clone()),
            manual_title: session.manual_title.clone(),
            pinned: session.pinned,
            updated_at: session.updated_at.clone(),
            status: session.status.as_str().to_string(),
            label: format_session_selector_label(session),
        })
        .collect::<Vec<_>>();

    serde_json::to_string(&options).map_err(|e| e.to_string())
}

#[tauri::command]
fn rename_session_command(payload: RenameSessionForm) -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let session = runtime::rename_session(&runtime, &payload.session_id, payload.manual_title)
        .map_err(|e| e.to_string())?;
    let manual_title = session.manual_title.clone();

    serde_json::to_string(&SessionSelectorOption {
        session_id: session.id.clone(),
        title: session
            .manual_title
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| session.title.clone()),
        manual_title,
        pinned: session.pinned,
        updated_at: session.updated_at.clone(),
        status: session.status.as_str().to_string(),
        label: format_session_selector_label(&session),
    })
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn pin_session_command(payload: PinSessionForm) -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let session = runtime::pin_session(&runtime, &payload.session_id, payload.pinned)
        .map_err(|e| e.to_string())?;
    let manual_title = session.manual_title.clone();

    serde_json::to_string(&SessionSelectorOption {
        session_id: session.id.clone(),
        title: session
            .manual_title
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| session.title.clone()),
        manual_title,
        pinned: session.pinned,
        updated_at: session.updated_at.clone(),
        status: session.status.as_str().to_string(),
        label: format_session_selector_label(&session),
    })
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_session_command(session_id: String) -> Result<(), String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    runtime::delete_session_and_related(&runtime, &session_id).map_err(|e| e.to_string())?;

    let storage_root = std::path::PathBuf::from("distilllab-storage");
    remove_session_attachment_storage(&storage_root, &session_id).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn cancel_stream_request_command(app: tauri::AppHandle, payload: CancelStreamRequestForm) -> Result<(), String> {
    let mut tasks = stream_request_tasks()
        .lock()
        .map_err(|_| "failed to lock stream request tasks".to_string())?;

    if let Some(handle) = tasks.remove(&payload.request_id) {
        handle.abort();
    }

    emit_chat_stream_event(
        &app,
        &build_chat_stream_event(
            &payload.request_id,
            &payload.session_id,
            ChatStreamPhase::Stopped,
            None,
            None,
            None,
            Some("request stopped".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
        ),
    )?;

    Ok(())
}

#[tauri::command]
async fn pick_attachments_command(app: tauri::AppHandle) -> Result<String, String> {
    let (sender, receiver) =
        std::sync::mpsc::sync_channel::<Option<Vec<tauri_plugin_dialog::FilePath>>>(1);
    tauri_plugin_dialog::DialogExt::dialog(&app)
        .file()
        .pick_files(move |files| {
            let _ = sender.send(files);
        });

    let files = tauri::async_runtime::spawn_blocking(move || receiver.recv().ok())
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or(None);

    let attachments = files
        .unwrap_or_default()
        .into_iter()
        .filter_map(|file_path: tauri_plugin_dialog::FilePath| {
            let path = file_path.into_path().ok()?;
            let name = path.file_name()?.to_str()?.to_string();
            Some(PendingAttachmentOption {
                path: path.to_string_lossy().to_string(),
                name,
            })
        })
        .collect::<Vec<_>>();

    serde_json::to_string(&attachments).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_runs() -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let runs = runtime::list_runs(&runtime).map_err(|e| e.to_string())?;

    if runs.is_empty() {
        return Ok("no runs found".to_string());
    }

    let summary = runs
        .iter()
        .map(|run| {
            format!(
                "{} [{}] {}:{}",
                run.id,
                run.run_type.as_str(),
                run.primary_object_type,
                run.primary_object_id
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(summary)
}

#[tauri::command]
fn chunk_demo_source() -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let (source, chunks) = runtime::chunk_demo_source(&runtime).map_err(|e| e.to_string())?;

    Ok(format!(
        "chunked source: {} [{}] into {} chunks",
        source.id,
        source.title,
        chunks.len()
    ))
}

#[tauri::command]
fn list_chunks_for_source(source_id: String) -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let chunks = runtime::list_chunks_for_source(&runtime, &source_id).map_err(|e| e.to_string())?;

    if chunks.is_empty() {
        return Ok(format!("no chunks found for source {}", source_id));
    }

    let summary = chunks
        .iter()
        .map(|chunk| format!("{} [{}] {}", chunk.id, chunk.sequence, chunk.content))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(summary)
}

#[tauri::command]
fn extract_demo_work_items() -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let (source, chunks, work_items) =
        runtime::extract_demo_work_items(&runtime).map_err(|e| e.to_string())?;

    Ok(format!(
        "extracted {} work items from {} chunks for source {}",
        work_items.len(),
        chunks.len(),
        source.id
    ))
}

#[tauri::command]
fn list_work_items() -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let work_items = runtime::list_work_items(&runtime).map_err(|e| e.to_string())?;

    if work_items.is_empty() {
        return Ok("no work items found".to_string());
    }

    let summary = work_items
        .iter()
        .map(|item| {
            format!(
                "{} [{}] {} -- {}",
                item.id,
                item.work_item_type.as_str(),
                item.title,
                item.summary
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(summary)
}

#[tauri::command]
fn group_demo_project() -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let (_source, _chunks, work_items, project) =
        runtime::group_demo_project(&runtime).map_err(|e| e.to_string())?;

    Ok(format!(
        "grouped project: {} with {} work items",
        project.name,
        work_items.len()
    ))
}

#[tauri::command]
fn list_projects() -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let projects = runtime::list_projects(&runtime).map_err(|e| e.to_string())?;

    if projects.is_empty() {
        return Ok("no projects found".to_string());
    }

    let summary = projects
        .iter()
        .map(|project| format!("{} -- {}", project.id, project.name))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(summary)
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CanvasProjectListItemDto {
    id: String,
    name: String,
}

#[tauri::command]
fn list_canvas_projects() -> Result<Vec<CanvasProjectListItemDto>, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let projects = runtime::list_projects(&runtime).map_err(|e| e.to_string())?;

    Ok(projects
        .into_iter()
        .map(|project| CanvasProjectListItemDto {
            id: project.id,
            name: project.name,
        })
        .collect())
}

#[tauri::command]
fn build_demo_assets() -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let (_source, _chunks, _work_items, project, assets) =
        runtime::build_demo_assets(&runtime).map_err(|e| e.to_string())?;

    Ok(format!(
        "built {} asset(s) for project {}",
        assets.len(),
        project.name
    ))
}

#[tauri::command]
fn list_assets() -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let assets = runtime::list_assets(&runtime).map_err(|e| e.to_string())?;

    if assets.is_empty() {
        return Ok("no assets found".to_string());
    }

    let summary = assets
        .iter()
        .map(|asset| {
            format!(
                "{} [{}] {} -- {}",
                asset.id,
                asset.asset_type.as_str(),
                asset.title,
                asset.summary
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(summary)
}

#[tauri::command]
async fn test_current_provider_command() -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let (config_path, config) = load_or_create_app_config()?;
    let resolved =
        resolve_current_provider_model(&config, &config_path).map_err(|e| e.to_string())?;

    let request = LlmSessionDebugRequest {
        provider_kind: resolved.provider_type.replace('-', "_"),
        base_url: resolved.base_url,
        model: resolved.model_id.clone(),
        api_key: resolved.api_key,
        user_message: "Reply with a short connectivity acknowledgement.".to_string(),
    };

    match runtime::decide_llm_session_message_with_config(&runtime, request).await {
        Ok(_) => Ok(format_provider_test_text(
            &resolved.provider_id,
            &resolved.model_id,
            "ok",
            "connected successfully",
        )),
        Err(error) => Ok(format_provider_test_text(
            &resolved.provider_id,
            &resolved.model_id,
            "error",
            &error.to_string(),
        )),
    }
}

#[tauri::command]
async fn send_session_message_command(form: SessionMessageForm) -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let (config_path, config) = load_or_create_app_config()?;
    let resolved =
        resolve_current_provider_model(&config, &config_path).map_err(|e| e.to_string())?;
    let session_id = form.session_id.clone();
    let storage_root = config_path
        .parent()
        .ok_or_else(|| "failed to resolve distilllab storage root".to_string())?;
    let attachments = form
        .attachment_paths
        .iter()
        .map(|path| store_attachment_copy(storage_root, &form.session_id, path).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;

    runtime::send_session_message_with_config(
        &runtime,
        SessionMessageRequest {
            session_id: session_id.clone(),
            user_message: form.user_message,
            attachments,
            provider_kind: resolved.provider_type.replace('-', "_"),
            base_url: resolved.base_url,
            model: resolved.model_id,
            api_key: resolved.api_key,
        },
    )
    .await
    .map_err(|e| e.to_string())?;

    load_session_messages_text_for_timeline(&session_id)
}

#[tauri::command]
async fn stream_session_message_command(
    app: tauri::AppHandle,
    payload: StreamSessionMessageForm,
) -> Result<(), String> {
    let request_id = payload.request_id.clone();
    let form = payload.form.clone();
    let app_handle = app.clone();

    let request_id_for_task = request_id.clone();
    let request_id_for_cleanup = request_id.clone();
    let handle = tauri::async_runtime::spawn(async move {
        let task = async {
            let runtime = AppRuntime::new("distilllab-dev.db".to_string());
            let (config_path, config) = load_or_create_app_config()?;
            let resolved =
                resolve_current_provider_model(&config, &config_path).map_err(|e| e.to_string())?;
            let storage_root = config_path
                .parent()
                .ok_or_else(|| "failed to resolve distilllab storage root".to_string())?
                .to_path_buf();

            emit_chat_stream_event(
                &app_handle,
                &build_chat_stream_event(
                    &request_id_for_task,
                    &form.session_id,
                    ChatStreamPhase::Started,
                    None,
                    None,
                    None,
                    Some("message send started".to_string()),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ),
            )?;

            let attachments = form
                .attachment_paths
                .iter()
                .map(|path| store_attachment_copy(&storage_root, &form.session_id, path).map_err(|e| e.to_string()))
                .collect::<Result<Vec<_>, _>>();

            let attachments = match attachments {
                Ok(attachments) => attachments,
                Err(error) => {
                    emit_chat_stream_event(
                        &app_handle,
                        &build_chat_stream_event(
                            &request_id_for_task,
                            &form.session_id,
                            ChatStreamPhase::Error,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            Some(error.clone()),
                            None,
                            None,
                            None,
                        ),
                    )?;
                    return Err(error);
                }
            };

            let mut emitted_assistant_started = false;
            let result = runtime::send_session_message_with_config_and_result_streaming_with_progress(
                &runtime,
                SessionMessageRequest {
                    session_id: form.session_id.clone(),
                    user_message: form.user_message,
                    attachments,
                    provider_kind: resolved.provider_type.replace('-', "_"),
                    base_url: resolved.base_url,
                    model: resolved.model_id,
                    api_key: resolved.api_key,
                },
                |chunk| {
                    if !emitted_assistant_started {
                        emitted_assistant_started = true;
                        let _ = emit_chat_stream_event(
                            &app_handle,
                            &build_chat_stream_event(
                                &request_id_for_task,
                                &form.session_id,
                                ChatStreamPhase::AssistantStarted,
                                None,
                                None,
                                None,
                                Some("assistant response started".to_string()),
                                None,
                                None,
                                None,
                                None,
                                None,
                                None,
                            ),
                        );
                    }

                    let _ = emit_chat_stream_event(
                        &app_handle,
                        &build_chat_stream_event(
                            &request_id_for_task,
                            &form.session_id,
                            ChatStreamPhase::AssistantChunk,
                            None,
                            None,
                            Some(chunk.to_string()),
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                        ),
                    );
                },
                |update| {
                    let _ = emit_run_progress_stream(
                        &app_handle,
                        &request_id_for_task,
                        &form.session_id,
                        &update,
                    );
                },
            )
            .await;

            match result {
                Ok(execution) => emit_execution_result_stream(&app_handle, &request_id_for_task, &execution),
                Err(error) => {
                    let error_text = error.to_string();
                    emit_chat_stream_event(
                        &app_handle,
                        &build_chat_stream_event(
                            &request_id_for_task,
                            &form.session_id,
                            ChatStreamPhase::Error,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            Some(error_text.clone()),
                            None,
                            None,
                            None,
                        ),
                    )?;
                    Err(error_text)
                }
            }
        }
        .await;

        if let Err(error) = task {
            log::error!("stream_session_message_command failed: {}", error);
        }
        remove_stream_request_task(&request_id_for_cleanup);
    });
    register_stream_request_task(request_id, handle);

    Ok(())
}

#[tauri::command]
async fn create_session_and_send_first_message_command(
    form: SessionMessageForm,
) -> Result<FirstSendCommandResponse, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let (config_path, config) = load_or_create_app_config()?;
    let resolved =
        resolve_current_provider_model(&config, &config_path).map_err(|e| e.to_string())?;
    let storage_root = config_path
        .parent()
        .ok_or_else(|| "failed to resolve distilllab storage root".to_string())?
        .to_path_buf();

    let session = create_session(&runtime).map_err(|e| e.to_string())?;
    let session_id = session.id.clone();

    let attachments = form
        .attachment_paths
        .iter()
        .map(|path| store_attachment_copy(&storage_root, &session_id, path).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>();

    let attachments = match attachments {
        Ok(attachments) => attachments,
        Err(error) => {
            delete_failed_first_send_session(&runtime, &session_id).map_err(|e| e.to_string())?;
            remove_session_attachment_storage(&storage_root, &session_id).map_err(|e| e.to_string())?;
            return Err(error);
        }
    };

    let send_result = runtime::send_session_message_with_config(
        &runtime,
        SessionMessageRequest {
            session_id: session_id.clone(),
            user_message: form.user_message,
            attachments,
            provider_kind: resolved.provider_type.replace('-', "_"),
            base_url: resolved.base_url,
            model: resolved.model_id,
            api_key: resolved.api_key,
        },
    )
    .await;

    if let Err(error) = send_result {
        delete_failed_first_send_session(&runtime, &session_id).map_err(|e| e.to_string())?;
        remove_session_attachment_storage(&storage_root, &session_id).map_err(|e| e.to_string())?;
        return Err(error.to_string());
    }

    let timeline_text = load_session_messages_text_for_timeline(&session_id)?;

    Ok(FirstSendCommandResponse {
        session_id,
        timeline_text,
    })
}

#[tauri::command]
async fn stream_first_session_message_command(
    app: tauri::AppHandle,
    payload: StreamSessionMessageForm,
) -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let (config_path, config) = load_or_create_app_config()?;
    let resolved =
        resolve_current_provider_model(&config, &config_path).map_err(|e| e.to_string())?;
    let storage_root = config_path
        .parent()
        .ok_or_else(|| "failed to resolve distilllab storage root".to_string())?
        .to_path_buf();
    let form = payload.form;

    let session = create_session(&runtime).map_err(|e| e.to_string())?;
    let session_id = session.id.clone();

    let request_id = payload.request_id.clone();
    let app_handle = app.clone();
    let first_form = form.clone();
    let first_session_id = session_id.clone();

    let request_id_for_task = request_id.clone();
    let request_id_for_cleanup = request_id.clone();
    let handle = tauri::async_runtime::spawn(async move {
        let task = async {
            emit_chat_stream_event(
                &app_handle,
                &build_chat_stream_event(
                    &request_id_for_task,
                    &first_session_id,
                    ChatStreamPhase::Started,
                    None,
                    None,
                    None,
                    Some("first message send started".to_string()),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ),
            )?;

            let attachments = first_form
                .attachment_paths
                .iter()
                .map(|path| store_attachment_copy(&storage_root, &first_session_id, path).map_err(|e| e.to_string()))
                .collect::<Result<Vec<_>, _>>();

            let attachments = match attachments {
                Ok(attachments) => attachments,
                Err(error) => {
                    delete_failed_first_send_session(&runtime, &first_session_id).map_err(|e| e.to_string())?;
                    remove_session_attachment_storage(&storage_root, &first_session_id).map_err(|e| e.to_string())?;
                    emit_chat_stream_event(
                        &app_handle,
                        &build_chat_stream_event(
                            &request_id_for_task,
                            &first_session_id,
                            ChatStreamPhase::Error,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            Some(error.clone()),
                            None,
                            None,
                            None,
                        ),
                    )?;
                    return Err(error);
                }
            };

            let mut emitted_assistant_started = false;
            let result = runtime::send_session_message_with_config_and_result_streaming_with_progress(
                &runtime,
                SessionMessageRequest {
                    session_id: first_session_id.clone(),
                    user_message: first_form.user_message,
                    attachments,
                    provider_kind: resolved.provider_type.replace('-', "_"),
                    base_url: resolved.base_url,
                    model: resolved.model_id,
                    api_key: resolved.api_key,
                },
                |chunk| {
                    if !emitted_assistant_started {
                        emitted_assistant_started = true;
                        let _ = emit_chat_stream_event(
                            &app_handle,
                            &build_chat_stream_event(
                                &request_id_for_task,
                                &first_session_id,
                                ChatStreamPhase::AssistantStarted,
                                None,
                                None,
                                None,
                                Some("assistant response started".to_string()),
                                None,
                                None,
                                None,
                                None,
                                None,
                                None,
                            ),
                        );
                    }

                    let _ = emit_chat_stream_event(
                        &app_handle,
                        &build_chat_stream_event(
                            &request_id_for_task,
                            &first_session_id,
                            ChatStreamPhase::AssistantChunk,
                            None,
                            None,
                            Some(chunk.to_string()),
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                        ),
                    );
                },
                |update| {
                    let _ = emit_run_progress_stream(
                        &app_handle,
                        &request_id_for_task,
                        &first_session_id,
                        &update,
                    );
                },
            )
            .await;

            match result {
                Ok(execution) => emit_execution_result_stream(&app_handle, &request_id_for_task, &execution),
                Err(error) => {
                    delete_failed_first_send_session(&runtime, &first_session_id).map_err(|e| e.to_string())?;
                    remove_session_attachment_storage(&storage_root, &first_session_id).map_err(|e| e.to_string())?;
                    let error_text = error.to_string();
                    emit_chat_stream_event(
                        &app_handle,
                        &build_chat_stream_event(
                            &request_id_for_task,
                            &first_session_id,
                            ChatStreamPhase::Error,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            Some(error_text.clone()),
                            None,
                            None,
                            None,
                        ),
                    )?;
                    Err(error_text)
                }
            }
        }
        .await;

        if let Err(error) = task {
            log::error!("stream_first_session_message_command failed: {}", error);
        }
        remove_stream_request_task(&request_id_for_cleanup);
    });
    register_stream_request_task(request_id.clone(), handle);

    Ok(session_id)
}

#[tauri::command]
async fn preview_session_intake_command(form: SessionMessageForm) -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let config_path = default_app_config_path().map_err(|e| e.to_string())?;
    let storage_root = config_path
        .parent()
        .ok_or_else(|| "failed to resolve distilllab storage root".to_string())?
        .to_path_buf();
    let (_, config) = load_or_create_app_config()?;
    let resolved =
        resolve_current_provider_model(&config, &config_path).map_err(|e| e.to_string())?;

    let attachments = form
        .attachment_paths
        .iter()
        .map(|path| store_attachment_copy(&storage_root, &form.session_id, path).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;

    let preview = runtime::preview_session_intake_with_config(
        &runtime,
        SessionIntake {
            session_id: form.session_id,
            user_message: form.user_message,
            attachments,
            current_object_type: None,
            current_object_id: None,
        },
        LlmProviderConfig {
            provider_kind: resolved.provider_type.replace('-', "_"),
            base_url: resolved.base_url,
            model: resolved.model_id,
            api_key: resolved.api_key,
        },
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(format_intake_preview_text(&preview))
}

#[tauri::command]
fn load_llm_config_command() -> Result<String, String> {
    let (_, config) = load_or_create_app_config()?;
    let config_json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;

    format_app_config_text(&config_json)
}

#[tauri::command]
fn load_llm_config_json_command() -> Result<String, String> {
    let (_, config) = load_or_create_app_config()?;
    serde_json::to_string_pretty(&config).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_desktop_ui_preferences_command() -> Result<String, String> {
    let (config_path, _) = load_or_create_app_config()?;
    load_desktop_ui_preferences_from_path(&config_path)
}

#[tauri::command]
fn save_desktop_ui_preferences_command(preferences: DesktopUiPreferences) -> Result<String, String> {
    let (config_path, _) = load_or_create_app_config()?;
    save_desktop_ui_preferences_to_path(&config_path, preferences)
}

#[tauri::command]
fn load_max_agent_concurrency_command() -> Result<String, String> {
    let (config_path, _) = load_or_create_app_config()?;
    load_max_agent_concurrency_from_path(&config_path)
}

#[tauri::command]
fn save_max_agent_concurrency_command(max_agent_concurrency: i64) -> Result<String, String> {
    let (config_path, _) = load_or_create_app_config()?;
    save_max_agent_concurrency_to_path(&config_path, max_agent_concurrency)
}

#[tauri::command]
fn save_llm_config_command(form: ConfigBarForm) -> Result<String, String> {
    let (config_path, _) = load_or_create_app_config()?;
    let provider_key = form.current_provider.trim();
    if provider_key.is_empty() {
        return Err("current provider is required".to_string());
    }

    let provider_entry = build_provider_entry_from_form(&form)?;
    let config = upsert_provider_entry(
        &config_path,
        provider_key,
        provider_entry,
        Some(form.current_model.trim().to_string()),
    )
    .map_err(|e| e.to_string())?;
    let config_json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;

    format_app_config_text(&config_json)
}

#[tauri::command]
fn create_provider_command(provider_id: String) -> Result<String, String> {
    let provider_key = provider_id.trim();
    if provider_key.is_empty() {
        return Err("provider id is required".to_string());
    }

    let config_path = default_app_config_path().map_err(|e| e.to_string())?;
    let config = upsert_provider_entry(
        &config_path,
        provider_key,
        ProviderConfigEntry {
            npm: Some("@ai-sdk/openai-compatible".to_string()),
            name: provider_key.to_string(),
            options: ProviderOptions::default(),
            models: std::collections::BTreeMap::from([(
                "gpt-5.4".to_string(),
                ModelConfigEntry {
                    name: "GPT-5.4".to_string(),
                    ..Default::default()
                },
            )]),
        },
        Some("gpt-5.4".to_string()),
    )
    .map_err(|e| e.to_string())?;

    let config_json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    format_app_config_text(&config_json)
}

#[tauri::command]
fn delete_provider_command(provider_id: String) -> Result<String, String> {
    let provider_key = provider_id.trim();
    if provider_key.is_empty() {
        return Err("provider id is required".to_string());
    }

    let config_path = default_app_config_path().map_err(|e| e.to_string())?;
    let config = delete_provider_entry(&config_path, provider_key).map_err(|e| e.to_string())?;
    let config_json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    format_app_config_text(&config_json)
}

#[tauri::command]
fn set_current_provider_model_command(provider_id: String, model_id: String) -> Result<String, String> {
    let config_path = default_app_config_path().map_err(|e| e.to_string())?;
    let config =
        set_current_provider_model(&config_path, &provider_id, &model_id).map_err(|e| e.to_string())?;
    let config_json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    format_app_config_text(&config_json)
}

#[tauri::command]
fn import_opencode_providers_command(form: Option<ImportProvidersForm>) -> Result<String, String> {
    let source_path = match form.and_then(|value| value.source_path) {
        Some(path) if !path.trim().is_empty() => std::path::PathBuf::from(path),
        _ => default_opencode_config_path()?,
    };

    let config_path = default_app_config_path().map_err(|e| e.to_string())?;
    let config =
        import_providers_from_opencode_path(&source_path, &config_path).map_err(|e| e.to_string())?;
    let config_json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;

    format_app_config_text(&config_json)
}

#[tauri::command]
fn list_session_messages_command(session_id: String) -> Result<String, String> {
    load_session_messages_text_for_timeline(&session_id)
}

#[tauri::command]
fn list_session_messages_structured_command(
    session_id: String,
) -> Result<Vec<DesktopTimelineMessage>, String> {
    let messages = load_session_messages_for_timeline(&session_id)?;

    Ok(list_session_messages_structured_payload(&messages))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut context = tauri::generate_context!();
    let default_window_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))
        .expect("embedded PNG icon should decode")
        .to_owned();
    context.set_default_window_icon(Some(default_window_icon));

    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            app.handle().plugin(tauri_plugin_dialog::init())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_canvas_global_view,
            load_canvas_object_detail,
            create_demo_run,
            create_demo_session,
            create_session_command,
            create_session_and_send_first_message_command,
            stream_first_session_message_command,
            create_demo_source,
            list_runs,
            list_sessions,
            list_session_selector_options,
            cancel_stream_request_command,
            pick_attachments_command,
            rename_session_command,
            pin_session_command,
            delete_session_command,
            list_sources,
            chunk_demo_source,
            list_chunks_for_source,
            extract_demo_work_items,
            list_work_items,
            group_demo_project,
            list_projects,
            list_canvas_projects,
            build_demo_assets,
            list_assets,
            test_current_provider_command,
            send_session_message_command,
            stream_session_message_command,
            list_session_messages_command,
            list_session_messages_structured_command,
            load_llm_config_command,
            load_llm_config_json_command,
            load_desktop_ui_preferences_command,
            save_desktop_ui_preferences_command,
            load_max_agent_concurrency_command,
            save_max_agent_concurrency_command,
            save_llm_config_command,
            import_opencode_providers_command,
            create_provider_command,
            delete_provider_command,
            set_current_provider_model_command,
            preview_session_intake_command
        ])
        .run(context)
        .expect("error while running tauri application");
}
