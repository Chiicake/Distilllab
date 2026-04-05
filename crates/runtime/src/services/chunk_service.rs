use crate::app::AppRuntime;
use chrono::Utc;
use memory::chunk_store::{insert_chunk, list_chunks_by_source};
use memory::db::open_database;
use memory::migrations::run_migrations;
use memory::run_store::insert_run;
use memory::source_store::insert_source;
use schema::run::RunType;
use schema::{Chunk, Run, RunState, Source, SourceType};
use uuid::Uuid;

pub fn chunk_demo_source(
    runtime: &AppRuntime,
) -> Result<(Source, Vec<Chunk>), Box<dyn std::error::Error>> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let source = Source {
        id: format!("source-{}", Uuid::new_v4()),
        source_type: SourceType::Document,
        title: "Demo Chunk Source".to_string(),
        run_id: None,
        origin_key: None,
        locator: None,
        content: Some("Demo chunk source content".to_string()),
        metadata_json: "{}".to_string(),
        created_at: Utc::now().to_string(),
    };

    insert_source(&conn, &source)?;

    let chunks = vec![
        Chunk {
            id: format!("chunk-{}", Uuid::new_v4()),
            source_id: source.id.clone(),
            sequence: 0,
            title: "First chunk".to_string(),
            summary: "First demo summary".to_string(),
            content: "First chunk".to_string(),
        },
        Chunk {
            id: format!("chunk-{}", Uuid::new_v4()),
            source_id: source.id.clone(),
            sequence: 1,
            title: "Second chunk".to_string(),
            summary: "Second demo summary".to_string(),
            content: "Second chunk".to_string(),
        },
    ];

    for chunk in &chunks {
        insert_chunk(&conn, chunk)?;
    }

    let run = Run {
        id: format!("demo-chunk-run-{}", Uuid::new_v4()),
        run_type: RunType::Demo,
        status: RunState::Completed,
        primary_object_type: "source".to_string(),
        primary_object_id: source.id.clone(),
        created_at: Utc::now().to_string(),
    };

    insert_run(&conn, &run)?;

    let persisted_chunks = list_chunks_by_source(&conn, &source.id)?;
    Ok((source, persisted_chunks))
}

pub fn list_chunks_for_source(
    runtime: &AppRuntime,
    source_id: &str,
) -> Result<Vec<Chunk>, Box<dyn std::error::Error>> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let chunks = list_chunks_by_source(&conn, source_id)?;
    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn chunks_demo_source_into_two_chunks() {
        let db_path = format!("/tmp/distilllab-runtime-test-{}.db", Uuid::new_v4());
        let runtime = AppRuntime::new(db_path.clone());

        let (_source, chunks) = chunk_demo_source(&runtime).expect("failed to chunk demo source");

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].sequence, 0);
        assert_eq!(chunks[1].sequence, 1);

        let _ = std::fs::remove_file(db_path);
    }
}
