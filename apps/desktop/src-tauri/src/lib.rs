use agent::{SessionActionType, SessionAgentDecision};
use runtime::{AppRuntime, LlmSessionDebugRequest};

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LlmDebugForm {
    provider_kind: String,
    base_url: String,
    model: String,
    api_key: Option<String>,
    user_message: String,
}

fn format_action_type(action_type: &SessionActionType) -> &'static str {
    match action_type {
        SessionActionType::DirectReply => "direct_reply",
        SessionActionType::RequestClarification => "request_clarification",
        SessionActionType::CreateRun => "create_run",
    }
}

fn format_optional_text(value: Option<&str>) -> &str {
    value.unwrap_or("none")
}

fn format_session_agent_decision_text(decision: &SessionAgentDecision) -> String {
    [
        format!("intent: {}", decision.intent),
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
async fn decide_llm_session_message_debug(form: LlmDebugForm) -> Result<String, String> {
    let runtime = AppRuntime::new("distilllab-dev.db".to_string());
    let request = LlmSessionDebugRequest {
        provider_kind: form.provider_kind,
        base_url: form.base_url,
        model: form.model,
        api_key: form.api_key,
        user_message: form.user_message,
    };

    let decision = runtime::decide_llm_session_message_with_config(&runtime, request)
        .await
        .map_err(|e| e.to_string())?;

    Ok(format_session_agent_decision_text(&decision))
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
            decide_llm_session_message_debug
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::format_session_agent_decision_text;
    use agent::{SessionActionType, SessionAgentDecision};

    #[test]
    fn formats_structured_session_agent_decision_as_plain_text() {
        let text = format_session_agent_decision_text(&SessionAgentDecision {
            intent: "llm_direct_reply".to_string(),
            primary_object_type: None,
            primary_object_id: None,
            action_type: SessionActionType::DirectReply,
            reply_text: "Hello from debug panel".to_string(),
            suggested_run_type: None,
            session_summary: Some("LLM replied to the current session message".to_string()),
        });

        assert!(text.contains("intent: llm_direct_reply"));
        assert!(text.contains("action_type: direct_reply"));
        assert!(text.contains("reply_text: Hello from debug panel"));
        assert!(text.contains("suggested_run_type: none"));
        assert!(text.contains("session_summary: LLM replied to the current session message"));
    }
}
