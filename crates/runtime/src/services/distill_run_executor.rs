use crate::app::AppRuntime;
use crate::contracts::{MaterializeSourcesResult, RunInput};
use crate::flows::execute_materialize_sources;
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
}

pub fn create_and_execute_from_decision(
    runtime: &AppRuntime,
    decision: &SessionAgentDecision,
    run_input: RunInput,
) -> Result<DistillRunExecutionOutcome, RuntimeError> {
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

    let materialize_result = if run.run_type.as_str() == "import_and_distill" {
        let result = execute_materialize_sources(runtime, &run.id, run_input)?;
        let next_status = if result.can_continue {
            RunState::Completed
        } else {
            RunState::Failed
        };
        update_run_status(&conn, &run.id, &next_status)?;
        run.status = next_status;
        Some(result)
    } else {
        None
    };

    Ok(DistillRunExecutionOutcome {
        run,
        materialize_result,
    })
}
