use crate::app::AppRuntime;
use crate::contracts::{
    LiveRunState, MaterializeSourcesResult, RunExecutionOutput, RunInput, RunProgressPhase,
    RunProgressUpdate,
};
use crate::flows::execute_materialize_sources;
use crate::runs::import_and_distill_step_definitions;
use crate::services::{list_sources_for_run, read_source_text};
use agent::{
    run_asset_extraction_agent,
    run_chunk_extraction_agent,
    run_project_resolution_agent, run_work_item_extraction_agent, LlmProviderConfig,
    AssetExtractionChunkInput, AssetExtractionInput, AssetExtractionWorkItemInput,
    ChunkDraft, ChunkExtractionInput,
    ProjectResolutionChunkInput, ProjectResolutionInput, ProjectResolutionWorkItemInput,
    ProjectSummaryInput, SessionActionType, SessionAgentDecision, WorkItemExtractionChunkInput,
    WorkItemExtractionInput,
};
use chrono::Utc;
use memory::asset_store::insert_asset;
use memory::chunk_store::insert_chunk;
use memory::db::open_database;
use memory::migrations::run_migrations;
use memory::project_store::insert_project;
use memory::run_store::{insert_run, update_run, update_run_status};
use schema::run::RunType;
use schema::{Asset, AssetType, Chunk, Project, Run, RunState};
use uuid::Uuid;

type RuntimeError = Box<dyn std::error::Error + Send + Sync>;

fn live_run_state_from_run_state(state: &RunState) -> LiveRunState {
    match state {
        RunState::Pending => LiveRunState::Pending,
        RunState::Running => LiveRunState::Running,
        RunState::Completed => LiveRunState::Completed,
        RunState::Failed => LiveRunState::Failed,
    }
}

#[derive(Debug, Clone)]
pub struct DistillRunExecutionOutcome {
    pub run: Run,
    pub materialize_result: Option<MaterializeSourcesResult>,
    pub output: Option<RunExecutionOutput>,
}

fn run_progress_update(
    phase: RunProgressPhase,
    run: &Run,
    progress_percent: Option<u8>,
    step_key: Option<&str>,
    step_summary: Option<&str>,
    step_status: Option<&str>,
    step_index: Option<u32>,
    steps_total: Option<u32>,
    detail_text: Option<&str>,
) -> RunProgressUpdate {
    RunProgressUpdate {
        phase,
        run_id: run.id.clone(),
        run_type: run.run_type.as_str().to_string(),
        run_state: live_run_state_from_run_state(&run.status),
        progress_percent,
        step_key: step_key.map(str::to_string),
        step_summary: step_summary.map(str::to_string),
        step_status: step_status.and_then(|status| match status {
            "started" => Some(crate::contracts::LiveRunStepStatus::Started),
            "running" => Some(crate::contracts::LiveRunStepStatus::Running),
            "completed" => Some(crate::contracts::LiveRunStepStatus::Completed),
            "failed" => Some(crate::contracts::LiveRunStepStatus::Failed),
            _ => None,
        }),
        step_index,
        steps_total,
        detail_text: detail_text.map(str::to_string),
    }
}

fn persist_chunk_drafts(
    conn: &rusqlite::Connection,
    source_id: &str,
    drafts: Vec<ChunkDraft>,
) -> Result<Vec<Chunk>, RuntimeError> {
    let mut persisted = Vec::new();
    for (index, draft) in drafts.into_iter().enumerate() {
        let chunk = Chunk {
            id: format!("chunk-{}", Uuid::new_v4()),
            source_id: source_id.to_string(),
            sequence: index as i64,
            title: draft.title,
            summary: draft.summary,
            content: draft.content,
        };
        insert_chunk(conn, &chunk)?;
        persisted.push(chunk);
    }
    Ok(persisted)
}

pub fn create_and_execute_from_decision<'a>(
    runtime: &'a AppRuntime,
    llm_provider_config: Option<&'a LlmProviderConfig>,
    decision: &'a SessionAgentDecision,
    run_input: RunInput,
) -> impl std::future::Future<Output = Result<DistillRunExecutionOutcome, RuntimeError>> + 'a {
    create_and_execute_from_decision_with_progress(
        runtime,
        llm_provider_config,
        decision,
        run_input,
        |_| {},
    )
}

pub async fn create_and_execute_from_decision_with_progress<F>(
    runtime: &AppRuntime,
    _llm_provider_config: Option<&LlmProviderConfig>,
    decision: &SessionAgentDecision,
    run_input: RunInput,
    mut on_progress: F,
) -> Result<DistillRunExecutionOutcome, RuntimeError>
where
    F: FnMut(RunProgressUpdate),
{
    if decision.action_type != SessionActionType::CreateRun {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "decision is not create_run",
        )));
    }

    let run_type = match decision.suggested_run_type.as_deref() {
        Some("import_and_distill") => RunType::ImportAndDistill,
        Some("deepening") => RunType::Deepening,
        Some("compose_and_verify") => RunType::ComposeAndVerify,
        _ => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid suggested_run_type for create_run decision",
            )));
        }
    };

    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let mut run = Run {
        id: format!("run-{}", Uuid::new_v4()),
        run_type,
        status: RunState::Pending,
        primary_object_type: decision
            .primary_object_type
            .clone()
            .unwrap_or_else(|| "material".to_string()),
        primary_object_id: decision
            .primary_object_id
            .clone()
            .unwrap_or_else(|| "pending".to_string()),
        created_at: Utc::now().to_string(),
    };

    insert_run(&conn, &run)?;

    on_progress(run_progress_update(
        RunProgressPhase::Created,
        &run,
        Some(0),
        None,
        None,
        None,
        None,
        None,
        Some("run created"),
    ));

    run.status = RunState::Running;
    update_run_status(&conn, &run.id, &run.status)?;
    on_progress(run_progress_update(
        RunProgressPhase::StateChanged,
        &run,
        Some(5),
        None,
        None,
        None,
        None,
        None,
        Some("run state changed to running"),
    ));

    let materialize_result = if run.run_type.as_str() == "import_and_distill" {
        let steps = import_and_distill_step_definitions();
        let materialize_step = steps
            .iter()
            .find(|step| step.step_key == "materialize_sources")
            .ok_or_else(|| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "missing materialize_sources step definition",
                )) as RuntimeError
            })?;

        on_progress(run_progress_update(
            RunProgressPhase::StepStarted,
            &run,
            Some(10),
            Some(materialize_step.step_key),
            Some(materialize_step.summary),
            Some("running"),
            Some(1),
            Some(steps.len() as u32),
            Some("materialize step started"),
        ));

        let result = execute_materialize_sources(runtime, &run.id, run_input.clone())?;

        on_progress(run_progress_update(
            RunProgressPhase::StepFinished,
            &run,
            Some(25),
            Some(materialize_step.step_key),
            Some(materialize_step.summary),
            Some(if result.can_continue {
                "completed"
            } else {
                "failed"
            }),
            Some(1),
            Some(steps.len() as u32),
            Some(result.summary.as_str()),
        ));

        if result.can_continue {
            let llm_provider_config = _llm_provider_config.ok_or_else(|| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "missing provider config for import_and_distill pipeline",
                )) as RuntimeError
            })?;

            let chunk_step = steps
                .iter()
                .find(|step| step.step_key == "chunk_sources")
                .ok_or_else(|| {
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "missing chunk_sources step definition",
                    )) as RuntimeError
                })?;

            on_progress(run_progress_update(
                RunProgressPhase::StepStarted,
                &run,
                Some(35),
                Some(chunk_step.step_key),
                Some(chunk_step.summary),
                Some("running"),
                Some(2),
                Some(steps.len() as u32),
                Some("chunk step started"),
            ));

            let sources = list_sources_for_run(runtime, &run.id)?;
            let client = reqwest::Client::new();
            let mut tasks = tokio::task::JoinSet::new();

            for (index, source) in sources.iter().enumerate() {
                let source_progress_detail = format!(
                    "processing source {}/{}: {}",
                    index + 1,
                    sources.len(),
                    source.title
                );
                on_progress(run_progress_update(
                    RunProgressPhase::StateChanged,
                    &run,
                    Some(35),
                    Some(chunk_step.step_key),
                    Some(chunk_step.summary),
                    Some("running"),
                    Some(2),
                    Some(steps.len() as u32),
                    Some(source_progress_detail.as_str()),
                ));

                let source_id = source.id.clone();
                let source_title = source.title.clone();
                let source_type = source.source_type.as_str().to_string();
                let distill_goal = run_input.decision_summary.clone();
                let source_text = read_source_text(runtime, &source_id)?;
                let config = llm_provider_config.clone();
                let client_clone = client.clone();
                let run_id = run.id.clone();
                let runtime = runtime.clone();

                tasks.spawn(async move {
                    runtime
                        .with_agent_dispatch_permit(|| async move {
                            let output = run_chunk_extraction_agent(
                                &client_clone,
                                &config,
                                &ChunkExtractionInput {
                                    run_id,
                                    source_id: source_id.clone(),
                                    source_type,
                                    source_title,
                                    source_text,
                                    distill_goal,
                                },
                            )
                            .await?;

                            Ok::<(String, Vec<ChunkDraft>), agent::AgentError>((
                                source_id,
                                output.chunks,
                            ))
                        })
                        .await
                });
            }

            let mut total_chunks = 0usize;
            while let Some(task_result) = tasks.join_next().await {
                let chunk_result = task_result.map_err(|error| {
                    Box::new(std::io::Error::other(format!("chunk task join failed: {}", error)))
                        as RuntimeError
                })?;
                let (source_id, drafts) = chunk_result.map_err(RuntimeError::from)?;
                let persisted = persist_chunk_drafts(&conn, &source_id, drafts)?;
                total_chunks += persisted.len();
            }

            let chunk_finish_detail = format!("created {} chunks", total_chunks);
            on_progress(run_progress_update(
                RunProgressPhase::StepFinished,
                &run,
                Some(50),
                Some(chunk_step.step_key),
                Some(chunk_step.summary),
                Some("completed"),
                Some(2),
                Some(steps.len() as u32),
                Some(chunk_finish_detail.as_str()),
            ));

            let work_item_step = steps
                .iter()
                .find(|step| step.step_key == "extract_work_items")
                .ok_or_else(|| {
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "missing extract_work_items step definition",
                    )) as RuntimeError
                })?;

            on_progress(run_progress_update(
                RunProgressPhase::StepStarted,
                &run,
                Some(60),
                Some(work_item_step.step_key),
                Some(work_item_step.summary),
                Some("running"),
                Some(3),
                Some(steps.len() as u32),
                Some("work item extraction started"),
            ));

            let chunk_inputs = sources
                .iter()
                .flat_map(|source| {
                    memory::chunk_store::list_chunks_by_source(&conn, &source.id)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|chunk| WorkItemExtractionChunkInput {
                            chunk_id: chunk.id,
                            title: chunk.title,
                            summary: chunk.summary,
                            content: chunk.content,
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();

            let work_item_output = run_work_item_extraction_agent(
                &client,
                llm_provider_config,
                &WorkItemExtractionInput {
                    run_id: run.id.clone(),
                    distill_goal: run_input.decision_summary.clone(),
                    chunks: chunk_inputs,
                },
            )
            .await
            .map_err(RuntimeError::from)?;

            let work_item_finish_detail =
                format!("extracted {} work item drafts", work_item_output.work_items.len());
            on_progress(run_progress_update(
                RunProgressPhase::StepFinished,
                &run,
                Some(75),
                Some(work_item_step.step_key),
                Some(work_item_step.summary),
                Some("completed"),
                Some(3),
                Some(steps.len() as u32),
                Some(work_item_finish_detail.as_str()),
            ));

            let project_step = steps
                .iter()
                .find(|step| step.step_key == "resolve_project_context")
                .ok_or_else(|| {
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "missing resolve_project_context step definition",
                    )) as RuntimeError
                })?;

            on_progress(run_progress_update(
                RunProgressPhase::StepStarted,
                &run,
                Some(85),
                Some(project_step.step_key),
                Some(project_step.summary),
                Some("running"),
                Some(4),
                Some(steps.len() as u32),
                Some("project resolution started"),
            ));

            let existing_projects = memory::project_store::list_projects(&conn)?
                .into_iter()
                .map(|project| ProjectSummaryInput {
                    project_id: project.id,
                    name: project.name,
                    summary: project.summary,
                })
                .collect::<Vec<_>>();

            let project_decision = run_project_resolution_agent(
                &client,
                llm_provider_config,
                &ProjectResolutionInput {
                    run_id: run.id.clone(),
                    distill_goal: run_input.decision_summary.clone(),
                    chunks: sources
                        .iter()
                        .map(|source| memory::chunk_store::list_chunks_by_source(&conn, &source.id))
                        .collect::<Result<Vec<_>, _>>()?
                        .into_iter()
                        .flatten()
                        .map(|chunk| ProjectResolutionChunkInput {
                            chunk_id: chunk.id,
                            title: chunk.title,
                            summary: chunk.summary,
                        })
                        .collect(),
                    work_items: work_item_output
                        .work_items
                        .iter()
                        .map(|item| ProjectResolutionWorkItemInput {
                            title: item.title.clone(),
                            summary: item.summary.clone(),
                        })
                        .collect(),
                    existing_projects,
                },
            )
            .await
            .map_err(RuntimeError::from)?;

            let resolved_project = match project_decision {
                agent::ProjectResolutionDecision::UseExistingProject { project_id, .. } => {
                    memory::project_store::get_project_by_id(&conn, &project_id)?
                        .ok_or_else(|| {
                            Box::new(std::io::Error::new(
                                std::io::ErrorKind::NotFound,
                                format!("resolved project not found: {project_id}"),
                            )) as RuntimeError
                        })?
                }
                agent::ProjectResolutionDecision::CreateNewProject {
                    title,
                    summary,
                    ..
                } => {
                    let project = Project {
                        id: format!("project-{}", Uuid::new_v4()),
                        name: title,
                        summary,
                    };
                    insert_project(&conn, &project)?;
                    project
                }
            };

            run.primary_object_type = "project".to_string();
            run.primary_object_id = resolved_project.id.clone();
            update_run(&conn, &run)?;

            let project_finish_detail = format!("resolved project: {}", resolved_project.name);
            on_progress(run_progress_update(
                RunProgressPhase::StepFinished,
                &run,
                Some(90),
                Some(project_step.step_key),
                Some(project_step.summary),
                Some("completed"),
                Some(4),
                Some(steps.len() as u32),
                Some(project_finish_detail.as_str()),
            ));

            let asset_step = steps
                .iter()
                .find(|step| step.step_key == "extract_assets")
                .ok_or_else(|| {
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "missing extract_assets step definition",
                    )) as RuntimeError
                })?;

            on_progress(run_progress_update(
                RunProgressPhase::StepStarted,
                &run,
                Some(95),
                Some(asset_step.step_key),
                Some(asset_step.summary),
                Some("running"),
                Some(5),
                Some(steps.len() as u32),
                Some("asset extraction started"),
            ));

            let chunk_inputs = sources
                .iter()
                .map(|source| memory::chunk_store::list_chunks_by_source(&conn, &source.id))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .map(|chunk| AssetExtractionChunkInput {
                    chunk_id: chunk.id,
                    title: chunk.title,
                    summary: chunk.summary,
                    content: chunk.content,
                })
                .collect::<Vec<_>>();

            let asset_output = run_asset_extraction_agent(
                &client,
                llm_provider_config,
                &AssetExtractionInput {
                    run_id: run.id.clone(),
                    distill_goal: run_input.decision_summary.clone(),
                    project_id: resolved_project.id.clone(),
                    project_name: resolved_project.name.clone(),
                    project_summary: resolved_project.summary.clone(),
                    chunks: chunk_inputs,
                    work_items: work_item_output
                        .work_items
                        .iter()
                        .map(|item| AssetExtractionWorkItemInput {
                            title: item.title.clone(),
                            summary: item.summary.clone(),
                        })
                        .collect(),
                },
            )
            .await
            .map_err(RuntimeError::from)?;

            let mut persisted_assets = Vec::new();
            for draft in asset_output.assets {
                let asset = Asset {
                    id: format!("asset-{}", Uuid::new_v4()),
                    project_id: resolved_project.id.clone(),
                    asset_type: AssetType::Insight,
                    title: draft.title,
                    summary: draft.summary,
                };
                insert_asset(&conn, &asset)?;
                persisted_assets.push(asset);
            }

            let primary_asset_id = persisted_assets.first().map(|asset| asset.id.clone());
            if let Some(primary_asset_id) = primary_asset_id.clone() {
                run.primary_object_type = "asset".to_string();
                run.primary_object_id = primary_asset_id;
                update_run(&conn, &run)?;
            }

            let asset_finish_detail = format!("created {} assets", persisted_assets.len());
            on_progress(run_progress_update(
                RunProgressPhase::StepFinished,
                &run,
                Some(98),
                Some(asset_step.step_key),
                Some(asset_step.summary),
                Some("completed"),
                Some(5),
                Some(steps.len() as u32),
                Some(asset_finish_detail.as_str()),
            ));

            let next_status = RunState::Completed;
            update_run_status(&conn, &run.id, &next_status)?;
            run.status = next_status;
            on_progress(run_progress_update(
                RunProgressPhase::StateChanged,
                &run,
                Some(100),
                None,
                None,
                None,
                None,
                None,
                Some("run completed"),
            ));

            return Ok(DistillRunExecutionOutcome {
                run,
                materialize_result: Some(result),
                output: Some(RunExecutionOutput {
                    primary_asset_id: primary_asset_id.clone(),
                    asset_ids: persisted_assets.iter().map(|asset| asset.id.clone()).collect(),
                    work_item_ids: vec![],
                    execution_summary: format!(
                        "materialized sources, created {} chunks, extracted {} work item drafts, resolved project {}, created {} assets",
                        total_chunks,
                        work_item_output.work_items.len(),
                        resolved_project.name,
                        persisted_assets.len(),
                    ),
                }),
            });
        }

        let next_status = if result.can_continue {
            RunState::Completed
        } else {
            RunState::Failed
        };
        update_run_status(&conn, &run.id, &next_status)?;
        run.status = next_status;
        on_progress(run_progress_update(
            RunProgressPhase::StateChanged,
            &run,
            Some(if matches!(run.status, RunState::Completed) {
                100
            } else {
                100
            }),
            None,
            None,
            None,
            None,
            None,
            Some(match run.status {
                RunState::Completed => "run completed",
                RunState::Failed => "run failed",
                _ => "run state changed",
            }),
        ));
        Some(result)
    } else {
        let detail = format!(
            "run type {} has no execution pipeline yet",
            run.run_type.as_str()
        );
        on_progress(run_progress_update(
            RunProgressPhase::StepStarted,
            &run,
            Some(10),
            Some("run_pipeline"),
            Some("Execute run pipeline"),
            Some("running"),
            Some(1),
            Some(1),
            Some(detail.as_str()),
        ));

        run.status = RunState::Completed;
        update_run_status(&conn, &run.id, &run.status)?;

        on_progress(run_progress_update(
            RunProgressPhase::StepFinished,
            &run,
            Some(100),
            Some("run_pipeline"),
            Some("Execute run pipeline"),
            Some("completed"),
            Some(1),
            Some(1),
            Some(detail.as_str()),
        ));
        on_progress(run_progress_update(
            RunProgressPhase::StateChanged,
            &run,
            Some(100),
            None,
            None,
            None,
            None,
            None,
            Some("run completed"),
        ));
        None
    };

    Ok(DistillRunExecutionOutcome {
        run,
        materialize_result,
        output: None,
    })
}

#[cfg(test)]
mod tests {
    use super::create_and_execute_from_decision_with_progress;
    use crate::app::AppRuntime;
    use crate::contracts::RunInput;
    use agent::{
        LlmProviderConfig, RunCreationRequest, SessionActionType, SessionAgentDecision,
        SessionIntent, SessionNextAction,
    };
    use schema::AttachmentRef;
    use std::fs;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::net::TcpListener;
    use tokio::sync::Semaphore;
    use tokio::task::JoinHandle;
    use tokio::time::{Duration, Instant};
    use uuid::Uuid;

    struct ChunkRequestGate {
        started: AtomicUsize,
        in_flight: AtomicUsize,
        max_in_flight: AtomicUsize,
        release_permits: Semaphore,
    }

    impl ChunkRequestGate {
        fn new() -> Self {
            Self {
                started: AtomicUsize::new(0),
                in_flight: AtomicUsize::new(0),
                max_in_flight: AtomicUsize::new(0),
                release_permits: Semaphore::new(0),
            }
        }

        fn started(&self) -> usize {
            self.started.load(Ordering::SeqCst)
        }

        fn in_flight(&self) -> usize {
            self.in_flight.load(Ordering::SeqCst)
        }

        fn max_in_flight(&self) -> usize {
            self.max_in_flight.load(Ordering::SeqCst)
        }

        fn release(&self, permits: usize) {
            self.release_permits.add_permits(permits);
        }
    }

    fn mock_distill_decision() -> SessionAgentDecision {
        SessionAgentDecision {
            intent: SessionIntent::DistillMaterial,
            primary_object_type: Some("material".to_string()),
            primary_object_id: None,
            action_type: SessionActionType::CreateRun,
            next_action: SessionNextAction::CreateRun(RunCreationRequest {
                run_type: "import_and_distill".to_string(),
                reasoning_summary: None,
            }),
            run_creation: Some(RunCreationRequest {
                run_type: "import_and_distill".to_string(),
                reasoning_summary: None,
            }),
            reply_text: "I will start a distill run for this work material.".to_string(),
            suggested_run_type: Some("import_and_distill".to_string()),
            session_summary: Some("Preparing to distill work material".to_string()),
            tool_invocation: None,
            skill_selection: None,
            should_continue_planning: true,
            failure_hint: Some("clarify_or_stop".to_string()),
        }
    }

    fn build_run_input(attachment_paths: &[String]) -> RunInput {
        RunInput {
            session_id: format!("session-{}", Uuid::new_v4()),
            trigger_message: "Please distill these work notes".to_string(),
            attachment_refs: attachment_paths
                .iter()
                .enumerate()
                .map(|(index, path)| AttachmentRef {
                    attachment_id: format!("attachment-{}", index + 1),
                    kind: "file_path".to_string(),
                    name: format!("notes-{}.md", index + 1),
                    mime_type: "text/markdown".to_string(),
                    path_or_locator: path.clone(),
                    size: 64,
                    metadata_json: "{}".to_string(),
                })
                .collect(),
            current_object_type: None,
            current_object_id: None,
            decision_summary: "Distill work material via import_and_distill".to_string(),
        }
    }

    fn create_test_attachments(count: usize) -> Vec<String> {
        (0..count)
            .map(|index| {
                let path = format!(
                    "/tmp/distilllab-run-executor-concurrency-{}-{}.md",
                    Uuid::new_v4(),
                    index
                );
                fs::write(
                    &path,
                    format!("# Notes {}\nship the first prototype next week", index + 1),
                )
                .expect("attachment fixture should write");
                path
            })
            .collect()
    }

    fn cleanup_files(paths: &[String]) {
        for path in paths {
            let _ = fs::remove_file(path);
        }
    }

    fn mock_response_for_request(request_text: &str) -> String {
        if request_text.contains("AssetExtractionAgent") {
            return r#"{
                "choices": [
                    {
                        "message": {
                            "role": "assistant",
                            "content": "{\"assets\":[{\"title\":\"Prototype launch readiness\",\"summary\":\"The launch is gated by scope finalization and clear coordination before next week.\"}]}"
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
            return r#"{
                "choices": [
                    {
                        "message": {
                            "role": "assistant",
                            "content": "{\"chunks\":[{\"title\":\"Progress update\",\"summary\":\"A concrete work update was captured.\",\"content\":\"Captured chunk content\"}]}"
                        }
                    }
                ]
            }"#
            .to_string();
        }

        panic!("unexpected request: {request_text}");
    }

    async fn spawn_chunk_gated_provider(gate: Arc<ChunkRequestGate>) -> LlmProviderConfig {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let address = listener
            .local_addr()
            .expect("listener should have local addr");

        tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener
                    .accept()
                    .await
                    .expect("server should accept connection");
                let gate = gate.clone();

                tokio::spawn(async move {
                    let mut buffer = vec![0_u8; 8192];
                    let bytes_read = tokio::io::AsyncReadExt::read(&mut stream, &mut buffer)
                        .await
                        .expect("server should read request");
                    let request_text = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();

                    if request_text.contains("ChunkExtractionAgent") {
                        gate.started.fetch_add(1, Ordering::SeqCst);
                        let current = gate.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                        gate.max_in_flight.fetch_max(current, Ordering::SeqCst);
                        let permit = gate
                            .release_permits
                            .acquire()
                            .await
                            .expect("chunk gate should acquire release permit");
                        drop(permit);
                        gate.in_flight.fetch_sub(1, Ordering::SeqCst);
                    }

                    let response_body = mock_response_for_request(&request_text);
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response_body.len(),
                        response_body
                    );

                    tokio::io::AsyncWriteExt::write_all(&mut stream, response.as_bytes())
                        .await
                        .expect("server should write response");
                });
            }
        });

        LlmProviderConfig {
            provider_kind: "openai_compatible".to_string(),
            base_url: format!("http://{}", address),
            model: "gpt-test".to_string(),
            api_key: None,
        }
    }

    async fn wait_until(label: &str, mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if condition() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("timed out waiting for condition: {label}");
    }

    async fn start_distill_run(
        runtime: AppRuntime,
        provider_config: LlmProviderConfig,
        run_input: RunInput,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            create_and_execute_from_decision_with_progress(
                &runtime,
                Some(&provider_config),
                &mock_distill_decision(),
                run_input,
                |_| {},
            )
            .await
            .expect("distill run should complete");
        })
    }

    #[tokio::test]
    async fn runtime_path_limit_one_forces_serial_execution() {
        let db_path = format!("/tmp/distilllab-run-executor-serial-{}.db", Uuid::new_v4());
        let runtime = AppRuntime::new(db_path.clone());
        runtime.set_max_agent_concurrency(1);
        let gate = Arc::new(ChunkRequestGate::new());
        let provider_config = spawn_chunk_gated_provider(gate.clone()).await;
        let attachments = create_test_attachments(2);
        let handle = start_distill_run(runtime, provider_config, build_run_input(&attachments)).await;

        wait_until("first chunk request starts", || gate.started() >= 1).await;
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert_eq!(gate.started(), 1);
        assert_eq!(gate.max_in_flight(), 1);

        gate.release(1);
        wait_until("second chunk request starts", || gate.started() >= 2).await;
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert_eq!(gate.max_in_flight(), 1);

        gate.release(1);
        wait_until("third chunk request starts", || gate.started() >= 3).await;
        gate.release(1);
        handle.await.expect("run task should finish");

        cleanup_files(&attachments);
        let _ = fs::remove_file(db_path);
    }

    #[tokio::test]
    async fn runtime_path_limit_two_never_exceeds_two_in_flight_launches() {
        let db_path = format!("/tmp/distilllab-run-executor-limit-two-{}.db", Uuid::new_v4());
        let runtime = AppRuntime::new(db_path.clone());
        runtime.set_max_agent_concurrency(2);
        let gate = Arc::new(ChunkRequestGate::new());
        let provider_config = spawn_chunk_gated_provider(gate.clone()).await;
        let attachments = create_test_attachments(2);
        let handle = start_distill_run(runtime, provider_config, build_run_input(&attachments)).await;

        wait_until("two chunk requests start", || gate.started() >= 2).await;
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert_eq!(gate.in_flight(), 2);
        assert_eq!(gate.max_in_flight(), 2);
        assert_eq!(gate.started(), 2);

        gate.release(2);
        wait_until("third chunk request starts", || gate.started() >= 3).await;
        assert_eq!(gate.max_in_flight(), 2);

        gate.release(1);
        handle.await.expect("run task should finish");

        cleanup_files(&attachments);
        let _ = fs::remove_file(db_path);
    }

    #[tokio::test]
    async fn runtime_path_queued_work_waits_and_resumes_after_release() {
        let db_path = format!("/tmp/distilllab-run-executor-queued-{}.db", Uuid::new_v4());
        let runtime = AppRuntime::new(db_path.clone());
        runtime.set_max_agent_concurrency(1);
        let gate = Arc::new(ChunkRequestGate::new());
        let provider_config = spawn_chunk_gated_provider(gate.clone()).await;
        let attachments = create_test_attachments(2);
        let handle = start_distill_run(runtime, provider_config, build_run_input(&attachments)).await;

        wait_until("first chunk request starts", || gate.started() >= 1).await;
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert_eq!(gate.started(), 1);

        gate.release(1);
        wait_until("second chunk request starts after release", || gate.started() >= 2).await;
        gate.release(1);
        wait_until("third chunk request starts after second release", || gate.started() >= 3).await;
        gate.release(1);
        handle.await.expect("run task should finish");

        cleanup_files(&attachments);
        let _ = fs::remove_file(db_path);
    }

    #[tokio::test]
    async fn runtime_path_limit_changes_only_affect_subsequently_started_work() {
        let db_path = format!("/tmp/distilllab-run-executor-change-{}.db", Uuid::new_v4());
        let runtime = AppRuntime::new(db_path.clone());
        runtime.set_max_agent_concurrency(1);
        let gate = Arc::new(ChunkRequestGate::new());
        let provider_config = spawn_chunk_gated_provider(gate.clone()).await;
        let attachments = create_test_attachments(2);
        let handle = start_distill_run(runtime.clone(), provider_config, build_run_input(&attachments)).await;

        wait_until("first chunk request starts", || gate.started() >= 1).await;
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert_eq!(gate.started(), 1);

        runtime.set_max_agent_concurrency(2);
        wait_until("second chunk request starts after increasing limit", || gate.started() >= 2)
            .await;
        assert_eq!(gate.max_in_flight(), 2);

        gate.release(2);
        wait_until("third chunk request starts after prior work releases", || gate.started() >= 3)
            .await;
        gate.release(1);
        handle.await.expect("run task should finish");

        cleanup_files(&attachments);
        let _ = fs::remove_file(db_path);
    }

    #[tokio::test]
    async fn runtime_path_lowering_limit_does_not_interrupt_already_running_work() {
        let db_path = format!("/tmp/distilllab-run-executor-lower-{}.db", Uuid::new_v4());
        let runtime = AppRuntime::new(db_path.clone());
        runtime.set_max_agent_concurrency(2);
        let gate = Arc::new(ChunkRequestGate::new());
        let provider_config = spawn_chunk_gated_provider(gate.clone()).await;
        let attachments = create_test_attachments(2);
        let handle = start_distill_run(runtime.clone(), provider_config, build_run_input(&attachments)).await;

        wait_until("two chunk requests start", || gate.started() >= 2).await;
        assert_eq!(gate.in_flight(), 2);

        runtime.set_max_agent_concurrency(1);
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert!(gate.started() >= 2);
        assert_eq!(gate.in_flight(), 2);

        gate.release(1);
        wait_until(
            "third chunk request starts after one already-queued request completes",
            || gate.started() >= 3,
        )
        .await;
        assert_eq!(gate.max_in_flight(), 2);

        gate.release(1);
        gate.release(1);
        handle.await.expect("run task should finish");

        cleanup_files(&attachments);
        let _ = fs::remove_file(db_path);
    }
}
