use crate::app::AppRuntime;
use chrono::Utc;
use memory::db::open_database;
use memory::migrations::run_migrations;
use memory::project_store::{insert_project, list_projects as memory_list_projects};
use memory::run_store::insert_run;
use schema::run::RunType;
use schema::{Chunk, Project, Run, RunState, Source, WorkItem};
use uuid::Uuid;

use crate::work_item_service::extract_demo_work_items;

pub fn group_demo_project(
    runtime: &AppRuntime,
) -> Result<(Source, Vec<Chunk>, Vec<WorkItem>, Project), Box<dyn std::error::Error>> {
    let (source, chunks, mut work_items) = extract_demo_work_items(runtime)?;

    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let project = Project {
        id: format!("project-{}", Uuid::new_v4()),
        name: "Distilllab".to_string(),
        summary: "Demo grouped project".to_string(),
    };

    insert_project(&conn, &project)?;

    for item in &mut work_items {
        item.project_id = project.id.clone();
    }

    let run = Run {
        id: format!("demo-project-run-{}", Uuid::new_v4()),
        run_type: RunType::Demo,
        status: RunState::Completed,
        primary_object_type: "project".to_string(),
        primary_object_id: project.id.clone(),
        created_at: Utc::now().to_string(),
    };
    insert_run(&conn, &run)?;

    Ok((source, chunks, work_items, project))
}

pub fn list_projects(runtime: &AppRuntime) -> Result<Vec<Project>, Box<dyn std::error::Error>> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let projects = memory_list_projects(&conn)?;
    Ok(projects)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn groups_demo_project_from_work_items() {
        let db_path = format!("/tmp/distilllab-project-test-{}.db", Uuid::new_v4());
        let runtime = AppRuntime::new(db_path.clone());

        let (_source, _chunks, work_items, project) =
            group_demo_project(&runtime).expect("failed to group demo project");

        assert_eq!(project.name, "Distilllab");
        assert_eq!(work_items.len(), 2);
        assert!(work_items.iter().all(|item| item.project_id == project.id));

        let _ = std::fs::remove_file(db_path);
    }
}
