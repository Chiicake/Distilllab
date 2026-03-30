use agent::{SessionActionType, SessionAgentDecision};
use runtime::{
    AppConfig, AppRuntime, LlmSessionDebugRequest, ModelConfigEntry, ProviderConfigEntry,
    ProviderOptions, SessionMessageRequest, default_app_config_path,
    delete_provider_entry, import_providers_from_opencode_path, load_app_config_from_path,
    resolve_current_provider_model, set_current_provider_model, upsert_provider_entry,
};
use schema::SessionMessage;

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
}

fn format_action_type(action_type: &SessionActionType) -> &'static str {
    match action_type {
        SessionActionType::DirectReply => "direct_reply",
        SessionActionType::RequestClarification => "request_clarification",
        SessionActionType::ToolCall => "tool_call",
        SessionActionType::CreateRun => "create_run",
    }
}

fn format_optional_text(value: Option<&str>) -> &str {
    value.unwrap_or("none")
}

fn format_session_agent_decision_text(decision: &SessionAgentDecision) -> String {
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
    ]
    .join("\n")
}

fn format_session_messages_text(messages: &[SessionMessage]) -> String {
    if messages.is_empty() {
        return "no session messages found".to_string();
    }

    messages
        .iter()
        .map(|message| format!("[{}] {}", message.role.as_str(), message.content))
        .collect::<Vec<_>>()
        .join("\n\n")
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

    let decision = runtime::send_session_message_with_config(
        &runtime,
        SessionMessageRequest {
            session_id: form.session_id,
            user_message: form.user_message,
            provider_kind: resolved.provider_type.replace('-', "_"),
            base_url: resolved.base_url,
            model: resolved.model_id,
            api_key: resolved.api_key,
        },
    )
        .await
        .map_err(|e| e.to_string())?;

    Ok(format_session_agent_decision_text(&decision))
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
            create_demo_source,
            list_runs,
            list_sessions,
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
            list_session_messages_command,
            load_llm_config_command,
            load_llm_config_json_command,
            save_llm_config_command,
            import_opencode_providers_command,
            create_provider_command,
            delete_provider_command,
            set_current_provider_model_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{
        format_app_config_text, format_llm_debug_comparison_text, format_provider_test_text,
        format_session_agent_decision_text, format_session_messages_text,
    };
    use agent::{SessionActionType, SessionAgentDecision, SessionIntent};
    use schema::{SessionMessage, SessionMessageRole};

    #[test]
    fn formats_structured_session_agent_decision_as_plain_text() {
        let text = format_session_agent_decision_text(&SessionAgentDecision {
            intent: SessionIntent::GeneralReply,
            primary_object_type: None,
            primary_object_id: None,
            action_type: SessionActionType::DirectReply,
            tool_call_key: None,
            reply_text: "Hello from debug panel".to_string(),
            suggested_run_type: None,
            session_summary: Some("LLM replied to the current session message".to_string()),
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
            intent: SessionIntent::ImportMaterial,
            primary_object_type: Some("source".to_string()),
            primary_object_id: Some("source-1".to_string()),
            action_type: SessionActionType::CreateRun,
            tool_call_key: None,
            reply_text: "I will start an import and distill run.".to_string(),
            suggested_run_type: Some("import_and_distill".to_string()),
            session_summary: Some("Preparing to import material".to_string()),
        });

        assert!(text.contains("intent: import_material"));
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

        assert!(text.contains("[user] Hello timeline"));
        assert!(text.contains("[assistant] Hello back"));
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
    fn formats_llm_debug_comparison_text_with_raw_and_parsed_sections() {
        let text = format_llm_debug_comparison_text(
            "{\"intent\":\"import_material\"}",
            &SessionAgentDecision {
                intent: SessionIntent::ImportMaterial,
                primary_object_type: None,
                primary_object_id: None,
                action_type: SessionActionType::CreateRun,
                tool_call_key: None,
                reply_text: "I will start an import and distill run.".to_string(),
                suggested_run_type: Some("import_and_distill".to_string()),
                session_summary: Some("Preparing to import material".to_string()),
            },
        );

        assert!(text.contains("Raw LLM Output"));
        assert!(text.contains("Parsed Decision"));
        assert!(text.contains("import_material"));
        assert!(text.contains("import_and_distill"));
    }
}
