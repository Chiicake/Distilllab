use crate::app::AppRuntime;
use chrono::Utc;
use memory::chunk_store::{insert_chunk, list_chunks_by_source};
use memory::db::open_database;
use memory::migrations::run_migrations;
use memory::run_store::insert_run;
use memory::source_store::insert_source;
use memory::work_item_store::{insert_work_item, list_work_items as memory_list_work_items};
use schema::run::RunType;
use schema::{Chunk, Run, RunState, Source, SourceType, WorkItem, WorkItemType};
use uuid::Uuid;

pub fn extract_demo_work_items(
    runtime: &AppRuntime,
) -> Result<(Source, Vec<Chunk>, Vec<WorkItem>), Box<dyn std::error::Error>> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let source = Source {
        id: format!("source-{}", Uuid::new_v4()),
        source_type: SourceType::Document,
        title: "Demo Work Item Source".to_string(),
        created_at: Utc::now().to_string(),
    };
    insert_source(&conn, &source)?;

    let chunks = vec![
        Chunk {
            id: format!("chunk-{}", Uuid::new_v4()),
            source_id: source.id.clone(),
            sequence: 0,
            content: "Runtime should be explicit and traceable.".to_string(),
        },
        Chunk {
            id: format!("chunk-{}", Uuid::new_v4()),
            source_id: source.id.clone(),
            sequence: 1,
            content: "Source materials should become structured assets.".to_string(),
        },
    ];

    for chunk in &chunks {
        insert_chunk(&conn, chunk)?;
    }

    let persisted_chunks = list_chunks_by_source(&conn, &source.id)?;

    let work_items: Vec<WorkItem> = persisted_chunks
        .iter()
        .enumerate()
        .map(|(index, chunk)| WorkItem {
            id: format!("work-item-{}", Uuid::new_v4()),
            project_id: "unassigned".to_string(),
            work_item_type: WorkItemType::Note,
            title: format!("Work Item {}", index + 1),
            summary: chunk.content.clone(),
        })
        .collect();

    for work_item in &work_items {
        insert_work_item(&conn, work_item)?;
    }

    let run = Run {
        id: format!("demo-work-item-run-{}", Uuid::new_v4()),
        run_type: RunType::Demo,
        status: RunState::Completed,
        primary_object_type: "source".to_string(),
        primary_object_id: source.id.clone(),
        created_at: Utc::now().to_string(),
    };
    insert_run(&conn, &run)?;

    let persisted_work_items = memory_list_work_items(&conn)?;
    Ok((source, persisted_chunks, persisted_work_items))
}

pub fn list_work_items(runtime: &AppRuntime) -> Result<Vec<WorkItem>, Box<dyn std::error::Error>> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let work_items = memory_list_work_items(&conn)?;
    Ok(work_items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn extracts_demo_work_items_from_chunks() {
        let db_path = format!("/tmp/distilllab-work-item-test-{}.db", Uuid::new_v4());
        let runtime = AppRuntime::new(db_path.clone());

        let (_source, chunks, work_items) =
            extract_demo_work_items(&runtime).expect("failed to extract demo work items");

        assert_eq!(chunks.len(), 2);
        assert_eq!(work_items.len(), 2);
        assert_eq!(work_items[0].work_item_type.as_str(), "note");

        let _ = std::fs::remove_file(db_path);
    }
}
