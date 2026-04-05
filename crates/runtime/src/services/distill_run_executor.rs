use crate::agents::{run_chunk_agent, ChunkAgentInput, ChunkDraft};
use crate::app::AppRuntime;
use crate::contracts::{
    MaterializeSourcesResult, RunExecutionOutput, RunInput, RunProgressPhase, RunProgressUpdate,
};
use crate::flows::execute_materialize_sources;
use crate::runs::import_and_distill_step_definitions;
use crate::services::{list_sources_for_run, read_source_text};
use agent::{
    run_work_item_extraction_agent, LlmProviderConfig, SessionActionType, SessionAgentDecision,
    WorkItemExtractionChunkInput, WorkItemExtractionInput,
};
use chrono::Utc;
use memory::chunk_store::insert_chunk;
use memory::db::open_database;
use memory::migrations::run_migrations;
use memory::run_store::{insert_run, update_run_status};
use schema::run::RunType;
use schema::{Chunk, Run, RunState};
use uuid::Uuid;

type RuntimeError = Box<dyn std::error::Error + Send + Sync>;

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
        run_state: run.status.as_str().to_string(),
        progress_percent,
        step_key: step_key.map(str::to_string),
        step_summary: step_summary.map(str::to_string),
        step_status: step_status.map(str::to_string),
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
            let Some(llm_provider_config) = _llm_provider_config else {
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
                    output: None,
                });
            };

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

                tasks.spawn(async move {
                    let output = run_chunk_agent(
                        &client_clone,
                        &config,
                        &ChunkAgentInput {
                            run_id,
                            source_id: source_id.clone(),
                            source_type,
                            source_title,
                            source_text,
                            distill_goal,
                        },
                    )
                    .await?;

                    Ok::<(String, Vec<ChunkDraft>), agent::AgentError>((source_id, output.chunks))
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
                    primary_asset_id: None,
                    asset_ids: vec![],
                    work_item_ids: vec![],
                    execution_summary: format!(
                        "materialized sources, created {} chunks, extracted {} work item drafts",
                        total_chunks,
                        work_item_output.work_items.len()
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
