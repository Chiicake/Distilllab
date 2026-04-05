use agent::{LlmProviderConfig, SessionActionType, SessionAgentDecision};
use runtime::{
    AppConfig, AppRuntime, ChatStreamEvent, ChatStreamPhase, DesktopUiConfig,
    LlmSessionDebugRequest, ModelConfigEntry, ProviderConfigEntry, ProviderOptions,
    RunProgressPhase, RunProgressUpdate, SessionIntakePreview,
    SessionMessageExecutionResult, SessionMessageRequest,
    default_app_config_path,
    create_session,
    delete_failed_first_send_session,
    delete_provider_entry, import_providers_from_opencode_path, load_app_config_from_path,
    resolve_current_provider_model, save_app_config_to_path, set_current_provider_model,
    upsert_provider_entry,
};
use runtime::flows::attachment_storage::{remove_session_attachment_storage, store_attachment_copy};
use schema::{SessionIntake, SessionMessage, SessionMessageRole};
use tauri::Emitter;

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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopUiPreferences {
    theme: String,
    locale: String,
    show_debug_panel: bool,
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

fn desktop_ui_config_from_preferences(preferences: &DesktopUiPreferences) -> DesktopUiConfig {
    DesktopUiConfig {
        theme: preferences.theme.clone(),
        locale: preferences.locale.clone(),
        show_debug_panel: preferences.show_debug_panel,
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

    config.distilllab.desktop_ui = Some(desktop_ui_config_from_preferences(&preferences));
    save_app_config_to_path(&config, config_path).map_err(|e| e.to_string())?;
    load_desktop_ui_preferences_from_path(config_path)
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

                format!("{}\n{}", role_header, indent_block(&message.content))
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn indent_block(text: &str) -> String {
    text.lines()
        .map(|line| format!("  {}", line))
        .collect::<Vec<_>>()
        .join("\n")
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
    run_progress: RunProgressUpdate,
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
            update.run_id,
            update.run_type
        ),
        RunProgressPhase::StateChanged => format!(
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
            None,
            None,
            None,
            result.created_run_id.clone(),
        ),
    )?;

    if let Some(tool_name) = &result.tool_name {
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
            ),
        )?;

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
            ),
        )?;
    }

    if let Some(run_id) = &result.created_run_id {
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
            ),
        )?;

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
    let phase = stream_phase_from_progress(update);
    let status_text = progress_status_text(update);

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
            Some(update.run_id.clone()),
            update.clone(),
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
    let chunks =
        runtime::list_chunks_for_source(&runtime, &source_id).map_err(|e| e.to_string())?;

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
    let resolved = resolve_current_provider_model(&config, &config_path).map_err(|e| e.to_string())?;

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
    let resolved = resolve_current_provider_model(&config, &config_path).map_err(|e| e.to_string())?;
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

    let messages = runtime::list_session_messages(&runtime, &session_id).map_err(|e| e.to_string())?;

    Ok(format_session_messages_text(&messages))
}

#[tauri::command]
async fn stream_session_message_command(
    app: tauri::AppHandle,
    payload: StreamSessionMessageForm,
) -> Result<(), String> {
    let request_id = payload.request_id.clone();
    let form = payload.form.clone();
    let app_handle = app.clone();

    tauri::async_runtime::spawn(async move {
        let task = async {
            let runtime = AppRuntime::new("distilllab-dev.db".to_string());
            let (config_path, config) = load_or_create_app_config()?;
            let resolved = resolve_current_provider_model(&config, &config_path).map_err(|e| e.to_string())?;
            let storage_root = config_path
                .parent()
                .ok_or_else(|| "failed to resolve distilllab storage root".to_string())?
                .to_path_buf();

            emit_chat_stream_event(
                &app_handle,
                &build_chat_stream_event(
                    &request_id,
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
                            &request_id,
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
                                &request_id,
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
                            ),
                        );
                    }

                    let _ = emit_chat_stream_event(
                        &app_handle,
                        &build_chat_stream_event(
                            &request_id,
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
                        ),
                    );
                },
                |update| {
                    let _ = emit_run_progress_stream(
                        &app_handle,
                        &request_id,
                        &form.session_id,
                        &update,
                    );
                },
            )
            .await;

            match result {
                Ok(execution) => emit_execution_result_stream(&app_handle, &request_id, &execution),
                Err(error) => {
                    let error_text = error.to_string();
                    emit_chat_stream_event(
                        &app_handle,
                        &build_chat_stream_event(
                            &request_id,
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
    });

    Ok(())
}

#[tauri::command]
async fn create_session_and_send_first_message_command(
    form: SessionMessageForm,
) -> Result<FirstSendCommandResponse, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let (config_path, config) = load_or_create_app_config()?;
    let resolved = resolve_current_provider_model(&config, &config_path).map_err(|e| e.to_string())?;
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

    let messages = runtime::list_session_messages(&runtime, &session_id).map_err(|e| e.to_string())?;

    Ok(FirstSendCommandResponse {
        session_id,
        timeline_text: format_session_messages_text(&messages),
    })
}

#[tauri::command]
async fn stream_first_session_message_command(
    app: tauri::AppHandle,
    payload: StreamSessionMessageForm,
) -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let (config_path, config) = load_or_create_app_config()?;
    let resolved = resolve_current_provider_model(&config, &config_path).map_err(|e| e.to_string())?;
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

    tauri::async_runtime::spawn(async move {
        let task = async {
            emit_chat_stream_event(
                &app_handle,
                &build_chat_stream_event(
                    &request_id,
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
                            &request_id,
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
                                &request_id,
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
                            ),
                        );
                    }

                    let _ = emit_chat_stream_event(
                        &app_handle,
                        &build_chat_stream_event(
                            &request_id,
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
                        ),
                    );
                },
                |update| {
                    let _ = emit_run_progress_stream(
                        &app_handle,
                        &request_id,
                        &first_session_id,
                        &update,
                    );
                },
            )
            .await;

            match result {
                Ok(execution) => emit_execution_result_stream(&app_handle, &request_id, &execution),
                Err(error) => {
                    delete_failed_first_send_session(&runtime, &first_session_id).map_err(|e| e.to_string())?;
                    remove_session_attachment_storage(&storage_root, &first_session_id).map_err(|e| e.to_string())?;
                    let error_text = error.to_string();
                    emit_chat_stream_event(
                        &app_handle,
                        &build_chat_stream_event(
                            &request_id,
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
    });

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
    let resolved = resolve_current_provider_model(&config, &config_path).map_err(|e| e.to_string())?;

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
    let config = set_current_provider_model(&config_path, &provider_id, &model_id)
        .map_err(|e| e.to_string())?;
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
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let messages = runtime::list_session_messages(&runtime, &session_id).map_err(|e| e.to_string())?;

    Ok(format_session_messages_text(&messages))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_demo_run,
            create_demo_session,
            create_session_command,
            create_session_and_send_first_message_command,
            stream_first_session_message_command,
            create_demo_source,
            list_runs,
            list_sessions,
            list_session_selector_options,
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
            build_demo_assets,
            list_assets,
            test_current_provider_command,
            send_session_message_command,
            stream_session_message_command,
            list_session_messages_command,
            load_llm_config_command,
            load_llm_config_json_command,
            load_desktop_ui_preferences_command,
            save_desktop_ui_preferences_command,
            save_llm_config_command,
            import_opencode_providers_command,
            create_provider_command,
            delete_provider_command,
            set_current_provider_model_command,
            preview_session_intake_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{
        DesktopUiPreferences, desktop_ui_preferences_from_config,
        format_app_config_text, format_intake_preview_text, format_llm_debug_comparison_text,
        format_provider_test_text, format_session_agent_decision_text, format_session_messages_text,
        format_session_selector_label, load_desktop_ui_preferences_from_path,
        save_desktop_ui_preferences_to_path,
    };
    use agent::{
        RunCreationRequest, SessionActionType, SessionAgentDecision, SessionIntent,
        SessionNextAction,
    };
    use runtime::{AppConfig, DesktopUiConfig, DistillRunStepPreview, RunHandoffPreview, SessionIntakePreview, upsert_provider_entry};
    use schema::{SessionMessage, SessionMessageRole};

    fn test_config_path(test_name: &str) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        let path = std::env::temp_dir()
            .join("distilllab-desktop-tests")
            .join(format!("{}-{}-{}.json", test_name, std::process::id(), unique));

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("temp test directory should exist");
        }

        path
    }

    #[test]
    fn formats_structured_session_agent_decision_as_plain_text() {
        let text = format_session_agent_decision_text(&SessionAgentDecision {
            intent: SessionIntent::GeneralReply,
            primary_object_type: None,
            primary_object_id: None,
            action_type: SessionActionType::DirectReply,
            next_action: SessionNextAction::DirectReply,
            tool_invocation: None,
            skill_selection: None,
            run_creation: None,
            reply_text: "Hello from debug panel".to_string(),
            suggested_run_type: None,
            session_summary: Some("LLM replied to the current session message".to_string()),
            should_continue_planning: false,
            failure_hint: None,
        });

        assert!(text.contains("intent: general_reply"));
        assert!(text.contains("action_type: direct_reply"));
        assert!(text.contains("reply_text: Hello from debug panel"));
        assert!(text.contains("suggested_run_type: none"));
        assert!(text.contains("session_summary: LLM replied to the current session message"));
    }

    #[test]
    fn formats_create_run_decision_with_debug_readability() {
        let text = format_session_agent_decision_text(&SessionAgentDecision {
            intent: SessionIntent::DistillMaterial,
            primary_object_type: Some("source".to_string()),
            primary_object_id: Some("source-1".to_string()),
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
            session_summary: Some("Preparing to import material".to_string()),
            should_continue_planning: true,
            failure_hint: Some("clarify_or_stop".to_string()),
        });

        assert!(text.contains("intent: distill_material"));
        assert!(text.contains("action_type: create_run"));
        assert!(text.contains("primary_object_type: source"));
        assert!(text.contains("primary_object_id: source-1"));
        assert!(text.contains("suggested_run_type: import_and_distill"));
    }

    #[test]
    fn formats_session_messages_as_timeline_text() {
        let text = format_session_messages_text(&[
            SessionMessage {
                id: "message-1".to_string(),
                session_id: "session-1".to_string(),
                run_id: None,
                message_type: "user_message".to_string(),
                role: SessionMessageRole::User,
                content: "Hello timeline".to_string(),
                data_json: "{}".to_string(),
                created_at: "2026-03-29T00:00:00Z".to_string(),
            },
            SessionMessage {
                id: "message-2".to_string(),
                session_id: "session-1".to_string(),
                run_id: None,
                message_type: "assistant_message".to_string(),
                role: SessionMessageRole::Assistant,
                content: "Hello back".to_string(),
                data_json: "{}".to_string(),
                created_at: "2026-03-29T00:00:01Z".to_string(),
            },
        ]);

        assert!(text.contains("[User]\n  Hello timeline"));
        assert!(text.contains("[Assistant]\n  Hello back"));
    }

    #[test]
    fn formats_tool_result_messages_with_tool_header_style() {
        let text = format_session_messages_text(&[SessionMessage {
            id: "message-1".to_string(),
            session_id: "session-1".to_string(),
            run_id: None,
            message_type: "tool_result_message".to_string(),
            role: SessionMessageRole::System,
            content: "Attachment excerpt: hello".to_string(),
            data_json: r#"{"tool_name":"read_attachment_excerpt","arguments":{"attachment_index":0,"max_chars":400}}"#.to_string(),
            created_at: "2026-03-29T00:00:00Z".to_string(),
        }]);

        assert!(text.contains("[Tool] read_attachment_excerpt({\"attachment_index\":0,\"max_chars\":400})"));
        assert!(text.contains("  Attachment excerpt: hello"));
    }

    #[test]
    fn formats_session_selector_label_with_title_and_session_id() {
        let label = format_session_selector_label(&schema::Session {
            id: "session-123".to_string(),
            title: "Attachment Tooling Debug".to_string(),
            manual_title: None,
            pinned: false,
            status: schema::SessionStatus::Active,
            current_intent: "idle".to_string(),
            current_object_type: "none".to_string(),
            current_object_id: "none".to_string(),
            summary: "debugging attachment tools".to_string(),
            started_at: "2026-03-31T00:00:00Z".to_string(),
            updated_at: "2026-03-31T00:00:00Z".to_string(),
            last_user_message_at: "2026-03-31T00:00:00Z".to_string(),
            last_run_at: "2026-03-31T00:00:00Z".to_string(),
            last_compacted_at: "2026-03-31T00:00:00Z".to_string(),
            metadata_json: "{}".to_string(),
        });

        assert_eq!(label, "Attachment Tooling Debug (session-123)");
    }

    #[test]
    fn formats_app_config_as_readable_text() {
        let text = format_app_config_text(
            r#"{
                "provider": {
                    "ice": {
                        "name": "Ice",
                        "models": {
                            "gpt-5.4": { "name": "GPT-5.4" }
                        }
                    },
                    "openai": {
                        "name": "OpenAI",
                        "models": {
                            "gpt-5": { "name": "GPT-5" }
                        }
                    },
                    "copilot": {
                        "name": "GitHub Copilot",
                        "models": {
                            "gpt-4.1": { "name": "GPT-4.1" }
                        }
                    }
                },
                "distilllab": {
                    "currentProvider": "ice",
                    "currentModel": "gpt-5.4"
                }
            }"#,
        )
        .expect("config text should format");

        assert!(text.contains("current provider: ice"));
        assert!(text.contains("current model: gpt-5.4"));
        assert!(text.contains("providers: copilot, ice, openai"));
    }

    #[test]
    fn formats_current_provider_test_result_as_readable_text() {
        let text = format_provider_test_text("ice", "gpt-5.4", "ok", "connected successfully");

        assert!(text.contains("provider: ice"));
        assert!(text.contains("model: gpt-5.4"));
        assert!(text.contains("status: ok"));
        assert!(text.contains("message: connected successfully"));
    }

    #[test]
    fn load_desktop_ui_preferences_command_returns_defaults_when_missing() {
        let config_path = test_config_path("load-desktop-ui-defaults");
        let text = load_desktop_ui_preferences_from_path(&config_path).expect("preferences should load");
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid json");
        assert_eq!(value.get("theme").and_then(|v| v.as_str()), Some("system"));
        assert_eq!(value.get("locale").and_then(|v| v.as_str()), Some("en"));
        assert_eq!(
            value.get("showDebugPanel").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn load_desktop_ui_preferences_command_falls_back_to_system_for_invalid_saved_theme() {
        let config_path = test_config_path("load-desktop-ui-invalid-theme");
        std::fs::write(
            &config_path,
            r#"{
                "$schema": "https://opencode.ai/config.json",
                "distilllab": {
                    "desktopUi": {
                        "theme": "sepia",
                        "locale": "zh-CN",
                        "showDebugPanel": false
                    }
                }
            }"#,
        )
        .expect("seed config should save");

        let text = load_desktop_ui_preferences_from_path(&config_path).expect("preferences should load");
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid json");
        assert_eq!(value.get("theme").and_then(|v| v.as_str()), Some("system"));
        assert_eq!(value.get("locale").and_then(|v| v.as_str()), Some("zh-CN"));
        assert_eq!(
            value.get("showDebugPanel").and_then(|v| v.as_bool()),
            Some(false)
        );
    }

    #[test]
    fn save_desktop_ui_preferences_command_writes_distilllab_desktop_ui_and_preserves_other_config() {
        let config_path = test_config_path("save-desktop-ui-preferences");
        std::fs::write(
            &config_path,
            r#"{
                "$schema": "https://opencode.ai/config.json",
                "provider": {
                    "ice": {
                        "name": "Ice",
                        "models": {
                            "gpt-5.4": { "name": "GPT-5.4" }
                        }
                    }
                },
                "distilllab": {
                    "currentProvider": "ice",
                    "currentModel": "gpt-5.4"
                }
            }"#,
        )
        .expect("seed config should save");

        let text = save_desktop_ui_preferences_to_path(
            &config_path,
            DesktopUiPreferences {
                theme: "dark".to_string(),
                locale: "zh-CN".to_string(),
                show_debug_panel: false,
            },
        )
        .expect("preferences should save");

        let saved_preferences: serde_json::Value =
            serde_json::from_str(&text).expect("saved response should be valid json");
        assert_eq!(saved_preferences.get("theme").and_then(|v| v.as_str()), Some("dark"));
        assert_eq!(saved_preferences.get("locale").and_then(|v| v.as_str()), Some("zh-CN"));
        assert_eq!(
            saved_preferences.get("showDebugPanel").and_then(|v| v.as_bool()),
            Some(false)
        );

        let saved_config: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&config_path).expect("saved config should be readable"),
        )
        .expect("saved config should contain valid json");
        assert_eq!(
            saved_config
                .get("distilllab")
                .and_then(|v| v.get("desktopUi"))
                .and_then(|v| v.get("theme"))
                .and_then(|v| v.as_str()),
            Some("dark")
        );
        assert_eq!(
            saved_config
                .get("distilllab")
                .and_then(|v| v.get("currentProvider"))
                .and_then(|v| v.as_str()),
            Some("ice")
        );
        assert_eq!(
            saved_config
                .get("provider")
                .and_then(|v| v.get("ice"))
                .and_then(|v| v.get("models"))
                .and_then(|v| v.get("gpt-5.4"))
                .and_then(|v| v.get("name"))
                .and_then(|v| v.as_str()),
            Some("GPT-5.4")
        );
    }

    #[test]
    fn save_desktop_ui_preferences_command_rejects_invalid_theme_and_locale() {
        let config_path = test_config_path("save-desktop-ui-invalid");

        let invalid_theme_error = save_desktop_ui_preferences_to_path(
            &config_path,
            DesktopUiPreferences {
                theme: "sepia".to_string(),
                locale: "en".to_string(),
                show_debug_panel: true,
            },
        )
        .expect_err("invalid theme should be rejected");
        assert!(invalid_theme_error.contains("theme must be one of"));

        let invalid_locale_error = save_desktop_ui_preferences_to_path(
            &config_path,
            DesktopUiPreferences {
                theme: "system".to_string(),
                locale: "fr".to_string(),
                show_debug_panel: true,
            },
        )
        .expect_err("invalid locale should be rejected");
        assert!(invalid_locale_error.contains("locale must be one of"));
    }

    #[test]
    fn desktop_ui_preferences_round_trip_through_typed_config() {
        let preferences = desktop_ui_preferences_from_config(&AppConfig {
            distilllab: runtime::DistilllabConfigSection {
                desktop_ui: Some(DesktopUiConfig {
                    theme: "dark".to_string(),
                    locale: "zh-CN".to_string(),
                    show_debug_panel: false,
                }),
                ..Default::default()
            },
            ..Default::default()
        });

        assert_eq!(preferences.theme, "dark");
        assert_eq!(preferences.locale, "zh-CN");
        assert!(!preferences.show_debug_panel);
    }

    #[test]
    fn desktop_ui_preferences_survive_provider_save_mutation() {
        let config_path = test_config_path("desktop-ui-survives-provider-save");

        save_desktop_ui_preferences_to_path(
            &config_path,
            DesktopUiPreferences {
                theme: "dark".to_string(),
                locale: "zh-CN".to_string(),
                show_debug_panel: false,
            },
        )
        .expect("preferences should save");

        upsert_provider_entry(
            &config_path,
            "ice",
            runtime::ProviderConfigEntry {
                npm: Some("@ai-sdk/openai-compatible".to_string()),
                name: "Ice".to_string(),
                options: runtime::ProviderOptions {
                    base_url: Some("https://ice.v.ua/v1".to_string()),
                    api_key: Some("token".to_string()),
                },
                models: std::collections::BTreeMap::from([(
                    "gpt-5.4".to_string(),
                    runtime::ModelConfigEntry {
                        name: "GPT-5.4".to_string(),
                        ..Default::default()
                    },
                )]),
            },
            Some("gpt-5.4".to_string()),
        )
        .expect("provider save should succeed");

        let saved_config: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&config_path).expect("saved config should be readable"),
        )
        .expect("saved config should be valid json");

        assert_eq!(
            saved_config
                .get("distilllab")
                .and_then(|value| value.get("desktopUi"))
                .and_then(|value| value.get("theme"))
                .and_then(|value| value.as_str()),
            Some("dark")
        );
        assert_eq!(
            saved_config
                .get("distilllab")
                .and_then(|value| value.get("desktopUi"))
                .and_then(|value| value.get("locale"))
                .and_then(|value| value.as_str()),
            Some("zh-CN")
        );
        assert_eq!(
            saved_config
                .get("distilllab")
                .and_then(|value| value.get("desktopUi"))
                .and_then(|value| value.get("showDebugPanel"))
                .and_then(|value| value.as_bool()),
            Some(false)
        );
    }

    #[test]
    fn formats_llm_debug_comparison_text_with_raw_and_parsed_sections() {
        let text = format_llm_debug_comparison_text(
            "{\"intent\":\"distill_material\"}",
            &SessionAgentDecision {
                intent: SessionIntent::DistillMaterial,
                primary_object_type: None,
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
                session_summary: Some("Preparing to import material".to_string()),
                should_continue_planning: true,
                failure_hint: Some("clarify_or_stop".to_string()),
            },
        );

        assert!(text.contains("Raw LLM Output"));
        assert!(text.contains("Parsed Decision"));
        assert!(text.contains("distill_material"));
        assert!(text.contains("import_and_distill"));
    }

    #[test]
    fn formats_session_intake_preview_with_decision_and_handoff_sections() {
        let text = format_intake_preview_text(&SessionIntakePreview {
            decision: SessionAgentDecision {
                intent: SessionIntent::DistillMaterial,
                primary_object_type: None,
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
                reply_text: "I will start a distillation workflow for this work material.".to_string(),
                suggested_run_type: Some("import_and_distill".to_string()),
                session_summary: Some("Preparing to distill work material".to_string()),
                should_continue_planning: true,
                failure_hint: Some("clarify_or_stop".to_string()),
            },
            run_handoff_preview: Some(RunHandoffPreview {
                run_type: "import_and_distill".to_string(),
                primary_object_type: Some("material".to_string()),
                primary_object_id: None,
                summary: "Previewing the import-and-distill workflow for this work material.".to_string(),
                planned_steps: vec![
                    DistillRunStepPreview {
                        step_key: "materialize_sources".to_string(),
                        summary: "Materialize the current work material into one or more sources.".to_string(),
                    },
                    DistillRunStepPreview {
                        step_key: "chunk_sources".to_string(),
                        summary: "Chunk the source material into retrieval and extraction units.".to_string(),
                    },
                ],
            }),
        });

        assert!(text.contains("SessionAgent Decision"));
        assert!(text.contains("Run Handoff Preview"));
        assert!(text.contains("run_type: import_and_distill"));
        assert!(text.contains("- materialize_sources"));
        assert!(text.contains("- chunk_sources"));
    }

    #[test]
    fn formats_session_intake_preview_with_attachment_hint() {
        let text = format_intake_preview_text(&SessionIntakePreview {
            decision: SessionAgentDecision {
                intent: SessionIntent::DistillMaterial,
                primary_object_type: Some("material".to_string()),
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
                reply_text: "I will start a distillation workflow for this work material.".to_string(),
                suggested_run_type: Some("import_and_distill".to_string()),
                session_summary: Some("Preparing to distill work material".to_string()),
                should_continue_planning: true,
                failure_hint: Some("clarify_or_stop".to_string()),
            },
            run_handoff_preview: Some(RunHandoffPreview {
                run_type: "import_and_distill".to_string(),
                primary_object_type: Some("material".to_string()),
                primary_object_id: None,
                summary: "Previewing the import-and-distill workflow for this work material.".to_string(),
                planned_steps: vec![DistillRunStepPreview {
                    step_key: "materialize_sources".to_string(),
                    summary: "Materialize the current work material into one or more sources.".to_string(),
                }],
            }),
        });

        assert!(text.contains("primary_object_type: material"));
        assert!(text.contains("materialize_sources"));
    }
}
