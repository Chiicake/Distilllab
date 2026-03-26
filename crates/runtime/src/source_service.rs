use crate::app::AppRuntime;
use memory::db::open_database;
use memory::migrations::run_migrations;
use memory::run_store::insert_run;
use memory::source_store::insert_source;
use schema::run::RunType;
use schema::{RunRecord, RunState, SourceRecord, SourceType};
use uuid::Uuid;

pub fn create_demo_source(
    runtime: &AppRuntime,
) -> Result<SourceRecord, Box<dyn std::error::Error>> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let source = SourceRecord {
        id: format!("source-{}", Uuid::new_v4()),
        source_type: SourceType::Document,
        title: "Demo Source".to_string(),
        created_at: "2026-03-25T00:00:00Z".to_string(),
    };

    insert_source(&conn, &source)?;

    let run = RunRecord {
        id: format!("demo-source-run-{}", Uuid::new_v4()),
        run_type: RunType::Demo,
        status: RunState::Completed,
        created_at: "2026-03-25T00:00:00Z".to_string(),
    };

    insert_run(&conn, &run)?;

    Ok(source)
}
