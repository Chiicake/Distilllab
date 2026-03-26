use crate::app::AppRuntime;
use chrono::Utc;
use memory::asset_store::{insert_asset, list_assets as memory_list_assets};
use memory::db::open_database;
use memory::migrations::run_migrations;
use memory::run_store::insert_run;
use schema::run::RunType;
use schema::{Asset, AssetType, Chunk, Project, Run, RunState, Source, WorkItem};
use uuid::Uuid;

use crate::project_service::group_demo_project;

pub fn build_demo_assets(
    runtime: &AppRuntime,
) -> Result<(Source, Vec<Chunk>, Vec<WorkItem>, Project, Vec<Asset>), Box<dyn std::error::Error>> {
    let (source, chunks, work_items, project) = group_demo_project(runtime)?;

    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let asset = Asset {
        id: format!("asset-{}", Uuid::new_v4()),
        project_id: project.id.clone(),
        asset_type: AssetType::Insight,
        title: format!("{} Insight", project.name),
        summary: format!("Built from {} work items", work_items.len()),
    };

    insert_asset(&conn, &asset)?;

    let run = Run {
        id: format!("demo-asset-run-{}", Uuid::new_v4()),
        run_type: RunType::Demo,
        status: RunState::Completed,
        primary_object_type: "asset".to_string(),
        primary_object_id: asset.id.clone(),
        created_at: Utc::now().to_string(),
    };
    insert_run(&conn, &run)?;

    let assets = memory_list_assets(&conn)?;
    Ok((source, chunks, work_items, project, assets))
}

pub fn list_assets(runtime: &AppRuntime) -> Result<Vec<Asset>, Box<dyn std::error::Error>> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let assets = memory_list_assets(&conn)?;
    Ok(assets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn builds_demo_assets_from_project() {
        let db_path = format!("/tmp/distilllab-asset-test-{}.db", Uuid::new_v4());
        let runtime = AppRuntime::new(db_path.clone());

        let (_source, _chunks, _work_items, project, assets) =
            build_demo_assets(&runtime).expect("failed to build demo assets");

        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].project_id, project.id);
        assert_eq!(assets[0].asset_type.as_str(), "insight");

        let _ = std::fs::remove_file(db_path);
    }
}
