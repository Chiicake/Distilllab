use crate::app::AppRuntime;
use crate::contracts::{
    MaterializeSourcesResult, RunExecutionOutput, RunInput, RunProgressPhase, RunProgressUpdate,
};
use crate::flows::execute_materialize_sources;
use crate::runs::import_and_distill_step_definitions;
use agent::{SessionActionType, SessionAgentDecision};
use chrono::Utc;
use memory::db::open_database;
use memory::migrations::run_migrations;
use memory::run_store::{insert_run, update_run_status};
use schema::run::RunType;
use schema::{Run, RunState};
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

pub fn create_and_execute_from_decision(
    runtime: &AppRuntime,
    decision: &SessionAgentDecision,
    run_input: RunInput,
) -> Result<DistillRunExecutionOutcome, RuntimeError> {
    create_and_execute_from_decision_with_progress(runtime, decision, run_input, |_| {})
}

pub fn create_and_execute_from_decision_with_progress<F>(
    runtime: &AppRuntime,
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

        let result = execute_materialize_sources(runtime, &run.id, run_input)?;

        on_progress(run_progress_update(
            RunProgressPhase::StepFinished,
            &run,
            Some(90),
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
