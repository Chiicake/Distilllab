use crate::app::AppRuntime;
use memory::db::open_database;
use memory::migrations::run_migrations;
use memory::run_store::insert_run;
use schema::{RunRecord, RunState};
use schema::run::RunType;

pub fn create_demo_run(runtime: &AppRuntime) -> Result<RunRecord, Box<dyn std::error::Error>> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let run = RunRecord {
        id: "demo-run-1".to_string(),
        run_type: RunType::Demo,
        status: RunState::Completed,
        created_at: "2026-03-25T00:00:00Z".to_string(),
    };
    insert_run(&conn, &run)?;
    Ok(run)
}
