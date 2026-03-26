use runtime::AppRuntime;

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
            create_demo_source,
            list_runs,
            list_sources,
            chunk_demo_source,
            list_chunks_for_source,
            extract_demo_work_items,
            list_work_items,
            group_demo_project,
            list_projects
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
