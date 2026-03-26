use crate::app::AppRuntime;
use chrono::Utc;
use memory::db::open_database;
use memory::migrations::run_migrations;
use memory::run_store::{insert_run, list_runs as memory_list_runs};
use schema::run::RunType;
use schema::{RunRecord, RunState};
use uuid::Uuid;

pub fn create_demo_run(runtime: &AppRuntime) -> Result<RunRecord, Box<dyn std::error::Error>> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let run_id = format!("demo-run-{}", Uuid::new_v4());
    let run = RunRecord {
        id: run_id.clone(),
        run_type: RunType::Demo,
        status: RunState::Completed,
        primary_object_type: "run".to_string(),
        primary_object_id: run_id,
        created_at: Utc::now().to_string(),
    };
    insert_run(&conn, &run)?;
    Ok(run)
}

pub fn list_runs(runtime: &AppRuntime) -> Result<Vec<RunRecord>, Box<dyn std::error::Error>> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let runs = memory_list_runs(&conn)?;
    Ok(runs)
}
