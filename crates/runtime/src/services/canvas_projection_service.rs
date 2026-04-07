use crate::app::AppRuntime;
use crate::config::{default_app_config_path, load_app_config_from_path};
use memory::asset_store::list_assets;
use memory::db::open_database;
use memory::migrations::run_migrations;
use memory::project_store::list_projects;
use memory::source_store::list_sources;
use memory::work_item_store::list_work_items;
use schema::{Asset, Chunk, Project, Source, WorkItem};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[cfg(test)]
thread_local! {
    static REMEMBERED_CANVAS_PROJECT_CONFIG_PATH_OVERRIDE: std::cell::RefCell<Option<PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasGraphNode {
    pub id: String,
    pub node_type: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasGraphEdge {
    pub from: String,
    pub to: String,
    pub edge_type: String,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasGraphDto {
    pub nodes: Vec<CanvasGraphNode>,
    pub edges: Vec<CanvasGraphEdge>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasInspectorDto {
    pub node_id: String,
    pub node_type: String,
    pub fields: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasGlobalViewDto {
    pub current_project_id: Option<String>,
    pub graph: CanvasGraphDto,
    pub inspectors_by_node_id: BTreeMap<String, CanvasInspectorDto>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasDetailViewDto {
    pub focus_node_id: String,
    pub focus_node_type: String,
    pub graph: CanvasGraphDto,
    pub inspectors_by_node_id: BTreeMap<String, CanvasInspectorDto>,
}

fn inspector(node_id: &str, node_type: &str) -> CanvasInspectorDto {
    CanvasInspectorDto {
        node_id: node_id.to_string(),
        node_type: node_type.to_string(),
        fields: BTreeMap::new(),
    }
}

fn project_inspector(project: &Project) -> CanvasInspectorDto {
    let mut dto = inspector(&project.id, "project");
    dto.fields.insert("name".to_string(), project.name.clone());
    dto.fields
        .insert("summary".to_string(), project.summary.clone());
    dto
}

fn work_item_inspector(work_item: &WorkItem) -> CanvasInspectorDto {
    let mut dto = inspector(&work_item.id, "work_item");
    dto.fields
        .insert("projectId".to_string(), work_item.project_id.clone());
    dto.fields
        .insert("title".to_string(), work_item.title.clone());
    dto.fields
        .insert("summary".to_string(), work_item.summary.clone());
    dto
}

fn asset_inspector(asset: &Asset) -> CanvasInspectorDto {
    let mut dto = inspector(&asset.id, "asset");
    dto.fields
        .insert("projectId".to_string(), asset.project_id.clone());
    dto.fields.insert("title".to_string(), asset.title.clone());
    dto.fields
        .insert("summary".to_string(), asset.summary.clone());
    dto
}

fn source_inspector(source: &Source) -> CanvasInspectorDto {
    let mut dto = inspector(&source.id, "source");
    dto.fields.insert("title".to_string(), source.title.clone());
    if let Some(run_id) = &source.run_id {
        dto.fields.insert("runId".to_string(), run_id.clone());
    }
    dto
}

fn chunk_inspector(chunk: &Chunk) -> CanvasInspectorDto {
    let mut dto = inspector(&chunk.id, "chunk");
    dto.fields
        .insert("parentSource".to_string(), chunk.source_id.clone());
    dto.fields.insert("title".to_string(), chunk.title.clone());
    dto
}

fn push_node(nodes: &mut Vec<CanvasGraphNode>, id: &str, node_type: &str) {
    if nodes.iter().any(|node| node.id == id) {
        return;
    }

    nodes.push(CanvasGraphNode {
        id: id.to_string(),
        node_type: node_type.to_string(),
    });
}

fn push_edge(edges: &mut Vec<CanvasGraphEdge>, from: &str, to: &str, edge_type: &str) {
    if edges
        .iter()
        .any(|edge| edge.from == from && edge.to == to && edge.edge_type == edge_type)
    {
        return;
    }

    edges.push(CanvasGraphEdge {
        from: from.to_string(),
        to: to.to_string(),
        edge_type: edge_type.to_string(),
    });
}

fn upsert_inspector(
    inspectors_by_node_id: &mut BTreeMap<String, CanvasInspectorDto>,
    inspector_dto: CanvasInspectorDto,
) {
    inspectors_by_node_id.insert(inspector_dto.node_id.clone(), inspector_dto);
}

fn append_project_context(
    project_id: &str,
    projects: &[Project],
    work_items: &[WorkItem],
    assets: &[Asset],
    nodes: &mut Vec<CanvasGraphNode>,
    edges: &mut Vec<CanvasGraphEdge>,
    inspectors_by_node_id: &mut BTreeMap<String, CanvasInspectorDto>,
) {
    let Some(project) = projects.iter().find(|project| project.id == project_id) else {
        return;
    };

    push_node(nodes, &project.id, "project");
    upsert_inspector(inspectors_by_node_id, project_inspector(project));

    for work_item in work_items
        .iter()
        .filter(|work_item| work_item.project_id == project.id)
    {
        push_node(nodes, &work_item.id, "work_item");
        push_edge(edges, &project.id, &work_item.id, "project_has_work_item");
        upsert_inspector(inspectors_by_node_id, work_item_inspector(work_item));
    }

    for asset in assets.iter().filter(|asset| asset.project_id == project.id) {
        push_node(nodes, &asset.id, "asset");
        push_edge(edges, &project.id, &asset.id, "project_has_asset");
        upsert_inspector(inspectors_by_node_id, asset_inspector(asset));
    }
}

fn resolve_current_project_id(
    available_project_ids: &[String],
    explicit_project_id: Option<&str>,
    remembered_project_id: Option<String>,
) -> Option<String> {
    if let Some(project_id) = explicit_project_id.filter(|project_id| {
        available_project_ids
            .iter()
            .any(|candidate| candidate == project_id)
    }) {
        return Some(project_id.to_string());
    }

    if let Some(project_id) = remembered_project_id.filter(|project_id| {
        available_project_ids
            .iter()
            .any(|candidate| candidate == project_id)
    }) {
        return Some(project_id);
    }

    available_project_ids.first().cloned()
}

fn load_remembered_canvas_project_id_from_path(config_path: &std::path::Path) -> Option<String> {
    let config = load_app_config_from_path(&config_path).ok()?;
    config
        .distilllab
        .desktop_ui
        .and_then(|desktop_ui| desktop_ui.last_opened_canvas_project_id)
}

fn remembered_canvas_project_config_path() -> Option<PathBuf> {
    #[cfg(test)]
    {
        if let Some(path) = REMEMBERED_CANVAS_PROJECT_CONFIG_PATH_OVERRIDE
            .with(|override_path| override_path.borrow().clone())
        {
            return Some(path);
        }
    }

    default_app_config_path().ok()
}

fn load_remembered_canvas_project_id() -> Option<String> {
    let config_path = remembered_canvas_project_config_path()?;
    load_remembered_canvas_project_id_from_path(&config_path)
}

#[cfg(test)]
fn set_remembered_canvas_project_config_path_override(config_path: &std::path::Path) {
    REMEMBERED_CANVAS_PROJECT_CONFIG_PATH_OVERRIDE.with(|override_path| {
        *override_path.borrow_mut() = Some(config_path.to_path_buf());
    });
}

#[cfg(test)]
fn clear_remembered_canvas_project_config_path_override() {
    REMEMBERED_CANVAS_PROJECT_CONFIG_PATH_OVERRIDE.with(|override_path| {
        *override_path.borrow_mut() = None;
    });
}

pub fn load_canvas_global_view(
    runtime: &AppRuntime,
    project_id: Option<&str>,
) -> Result<CanvasGlobalViewDto, Box<dyn std::error::Error>> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let projects = list_projects(&conn)?;
    if projects.is_empty() {
        return Ok(CanvasGlobalViewDto::default());
    }

    let work_items = list_work_items(&conn)?;
    let assets = list_assets(&conn)?;
    let remembered_project_id = load_remembered_canvas_project_id();
    let available_project_ids = projects
        .iter()
        .map(|project| project.id.clone())
        .collect::<Vec<_>>();
    let current_project_id =
        resolve_current_project_id(&available_project_ids, project_id, remembered_project_id);

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut inspectors_by_node_id = BTreeMap::new();

    if let Some(current_project_id) = current_project_id.as_deref() {
        append_project_context(
            current_project_id,
            &projects,
            &work_items,
            &assets,
            &mut nodes,
            &mut edges,
            &mut inspectors_by_node_id,
        );
    }

    Ok(CanvasGlobalViewDto {
        current_project_id,
        graph: CanvasGraphDto { nodes, edges },
        inspectors_by_node_id,
    })
}

pub fn load_canvas_detail_view(
    runtime: &AppRuntime,
    node_type: &str,
    node_id: &str,
    project_id: Option<&str>,
) -> Result<CanvasDetailViewDto, Box<dyn std::error::Error>> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let projects = list_projects(&conn)?;
    let work_items = list_work_items(&conn)?;
    let assets = list_assets(&conn)?;
    let sources = list_sources(&conn)?;
    let mut chunks = Vec::new();
    for source in &sources {
        chunks.extend(memory::chunk_store::list_chunks_by_source(
            &conn, &source.id,
        )?);
    }

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut inspectors_by_node_id = BTreeMap::new();

    match node_type {
        "project" => {
            append_project_context(
                node_id,
                &projects,
                &work_items,
                &assets,
                &mut nodes,
                &mut edges,
                &mut inspectors_by_node_id,
            );
        }
        "work_item" => {
            if let Some(work_item) = work_items.iter().find(|work_item| work_item.id == node_id) {
                append_project_context(
                    &work_item.project_id,
                    &projects,
                    &work_items,
                    &assets,
                    &mut nodes,
                    &mut edges,
                    &mut inspectors_by_node_id,
                );
            }
        }
        "asset" => {
            if let Some(asset) = assets.iter().find(|asset| asset.id == node_id) {
                append_project_context(
                    &asset.project_id,
                    &projects,
                    &work_items,
                    &assets,
                    &mut nodes,
                    &mut edges,
                    &mut inspectors_by_node_id,
                );
            }
        }
        "source" => {
            if let Some(project_id) = project_id {
                append_project_context(
                    project_id,
                    &projects,
                    &work_items,
                    &assets,
                    &mut nodes,
                    &mut edges,
                    &mut inspectors_by_node_id,
                );
            }

            if let Some(source) = sources.iter().find(|source| source.id == node_id) {
                push_node(&mut nodes, &source.id, "source");
                upsert_inspector(&mut inspectors_by_node_id, source_inspector(source));

                for chunk in chunks.iter().filter(|chunk| chunk.source_id == source.id) {
                    push_node(&mut nodes, &chunk.id, "chunk");
                    push_edge(&mut edges, &source.id, &chunk.id, "source_has_chunk");
                    upsert_inspector(&mut inspectors_by_node_id, chunk_inspector(chunk));
                }
            }
        }
        "chunk" => {
            if let Some(project_id) = project_id {
                append_project_context(
                    project_id,
                    &projects,
                    &work_items,
                    &assets,
                    &mut nodes,
                    &mut edges,
                    &mut inspectors_by_node_id,
                );
            }

            if let Some(chunk) = chunks.iter().find(|chunk| chunk.id == node_id) {
                if let Some(source) = sources.iter().find(|source| source.id == chunk.source_id) {
                    push_node(&mut nodes, &source.id, "source");
                    upsert_inspector(&mut inspectors_by_node_id, source_inspector(source));
                }

                push_node(&mut nodes, &chunk.id, "chunk");
                push_edge(&mut edges, &chunk.source_id, &chunk.id, "source_has_chunk");
                upsert_inspector(&mut inspectors_by_node_id, chunk_inspector(chunk));
            }
        }
        _ => {}
    }

    Ok(CanvasDetailViewDto {
        focus_node_id: node_id.to_string(),
        focus_node_type: node_type.to_string(),
        graph: CanvasGraphDto { nodes, edges },
        inspectors_by_node_id,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        clear_remembered_canvas_project_config_path_override, load_canvas_detail_view,
        load_canvas_global_view, load_remembered_canvas_project_id_from_path,
        set_remembered_canvas_project_config_path_override,
    };
    use crate::app::AppRuntime;
    use crate::config::{save_app_config_to_path, AppConfig, DesktopUiConfig};
    use chrono::Utc;
    use memory::asset_store::insert_asset;
    use memory::chunk_store::insert_chunk;
    use memory::db::open_database;
    use memory::migrations::run_migrations;
    use memory::project_store::insert_project;
    use memory::source_store::insert_source;
    use memory::work_item_store::insert_work_item;
    use schema::{Asset, AssetType, Chunk, Project, Source, SourceType, WorkItem, WorkItemType};
    use uuid::Uuid;

    fn test_runtime() -> AppRuntime {
        AppRuntime::new(format!(
            "/tmp/distilllab-canvas-projection-{}.db",
            Uuid::new_v4()
        ))
    }

    fn insert_project_record(runtime: &AppRuntime, id: &str, name: &str) -> Project {
        let conn = open_database(&runtime.database_path).expect("database should open");
        run_migrations(&conn).expect("migrations should run");

        let project = Project {
            id: id.to_string(),
            name: name.to_string(),
            summary: format!("Summary for {name}"),
        };
        insert_project(&conn, &project).expect("project should insert");
        project
    }

    fn insert_work_item_record(runtime: &AppRuntime, id: &str, project_id: &str) -> WorkItem {
        let conn = open_database(&runtime.database_path).expect("database should open");
        run_migrations(&conn).expect("migrations should run");

        let work_item = WorkItem {
            id: id.to_string(),
            project_id: project_id.to_string(),
            work_item_type: WorkItemType::Note,
            title: format!("Work Item {id}"),
            summary: format!("Summary for {id}"),
        };
        insert_work_item(&conn, &work_item).expect("work item should insert");
        work_item
    }

    fn insert_asset_record(runtime: &AppRuntime, id: &str, project_id: &str) -> Asset {
        let conn = open_database(&runtime.database_path).expect("database should open");
        run_migrations(&conn).expect("migrations should run");

        let asset = Asset {
            id: id.to_string(),
            project_id: project_id.to_string(),
            asset_type: AssetType::Insight,
            title: format!("Asset {id}"),
            summary: format!("Summary for {id}"),
        };
        insert_asset(&conn, &asset).expect("asset should insert");
        asset
    }

    fn test_config_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "distilllab-canvas-config-test-{}.json",
            Uuid::new_v4()
        ))
    }

    fn write_canvas_project_preference(config_path: &std::path::Path, project_id: Option<&str>) {
        set_remembered_canvas_project_config_path_override(config_path);

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).expect("config parent should exist");
        }

        let mut config = AppConfig::default();
        config.distilllab.desktop_ui = Some(DesktopUiConfig {
            theme: "dark".to_string(),
            locale: "en-US".to_string(),
            show_debug_panel: false,
            last_opened_canvas_project_id: project_id.map(ToString::to_string),
        });

        save_app_config_to_path(&config, config_path).expect("config should save");
    }

    fn clear_canvas_project_preference(config_path: &std::path::Path) {
        let _ = std::fs::remove_file(config_path);
    }

    #[test]
    fn test_canvas_config_override_stays_on_temp_path_when_config_file_is_missing() {
        let config_path = test_config_path();
        clear_canvas_project_preference(&config_path);
        set_remembered_canvas_project_config_path_override(&config_path);

        assert_eq!(
            super::remembered_canvas_project_config_path().as_deref(),
            Some(config_path.as_path())
        );
        assert_eq!(
            load_remembered_canvas_project_id_from_path(&config_path),
            None
        );

        clear_remembered_canvas_project_config_path_override();
        clear_canvas_project_preference(&config_path);
    }

    fn insert_source_record(runtime: &AppRuntime, id: &str) -> Source {
        let conn = open_database(&runtime.database_path).expect("database should open");
        run_migrations(&conn).expect("migrations should run");

        let source = Source {
            id: id.to_string(),
            source_type: SourceType::Document,
            title: format!("Source {id}"),
            run_id: None,
            origin_key: None,
            locator: None,
            content: Some(format!("Content for {id}")),
            metadata_json: "{}".to_string(),
            created_at: Utc::now().to_rfc3339(),
        };
        insert_source(&conn, &source).expect("source should insert");
        source
    }

    fn insert_chunk_record(runtime: &AppRuntime, id: &str, source_id: &str) -> Chunk {
        let conn = open_database(&runtime.database_path).expect("database should open");
        run_migrations(&conn).expect("migrations should run");

        let chunk = Chunk {
            id: id.to_string(),
            source_id: source_id.to_string(),
            sequence: 0,
            title: format!("Chunk {id}"),
            summary: format!("Summary for {id}"),
            content: format!("Content for {id}"),
        };
        insert_chunk(&conn, &chunk).expect("chunk should insert");
        chunk
    }

    fn cleanup(runtime: &AppRuntime) {
        let _ = std::fs::remove_file(&runtime.database_path);
    }

    #[test]
    fn load_canvas_global_view_returns_empty_state_when_no_projects_exist() {
        let runtime = test_runtime();
        let config_path = test_config_path();
        clear_canvas_project_preference(&config_path);

        let projection = load_canvas_global_view(&runtime, None).expect("projection should load");

        assert_eq!(projection.current_project_id, None);
        assert!(projection.graph.nodes.is_empty());
        assert!(projection.graph.edges.is_empty());
        assert!(projection.inspectors_by_node_id.is_empty());

        cleanup(&runtime);
        clear_canvas_project_preference(&config_path);
    }

    #[test]
    fn load_canvas_global_view_resolves_current_project_by_override_then_remembered_then_first_available(
    ) {
        let runtime = test_runtime();
        let config_path = test_config_path();
        clear_canvas_project_preference(&config_path);
        let _alpha = insert_project_record(&runtime, "project-alpha", "Alpha");
        let beta = insert_project_record(&runtime, "project-beta", "Beta");
        let gamma = insert_project_record(&runtime, "project-gamma", "Gamma");

        write_canvas_project_preference(&config_path, Some(&beta.id));
        assert_eq!(
            load_remembered_canvas_project_id_from_path(&config_path).as_deref(),
            Some(beta.id.as_str())
        );

        let remembered_projection =
            load_canvas_global_view(&runtime, None).expect("projection should load");
        assert_eq!(
            remembered_projection.current_project_id.as_deref(),
            Some(beta.id.as_str())
        );

        let override_projection =
            load_canvas_global_view(&runtime, Some(&gamma.id)).expect("projection should load");
        assert_eq!(
            override_projection.current_project_id.as_deref(),
            Some(gamma.id.as_str())
        );

        let invalid_override_projection =
            load_canvas_global_view(&runtime, Some("missing-project"))
                .expect("projection should load");
        assert_eq!(
            invalid_override_projection.current_project_id.as_deref(),
            Some(beta.id.as_str())
        );

        let runtime_without_remembered = test_runtime();
        clear_canvas_project_preference(&config_path);
        let first = insert_project_record(&runtime_without_remembered, "project-a", "A Project");
        insert_project_record(&runtime_without_remembered, "project-z", "Z Project");

        let first_available_projection = load_canvas_global_view(&runtime_without_remembered, None)
            .expect("projection should load");
        assert_eq!(
            first_available_projection.current_project_id.as_deref(),
            Some(first.id.as_str())
        );

        write_canvas_project_preference(&config_path, Some("missing-project"));
        assert_eq!(
            load_remembered_canvas_project_id_from_path(&config_path).as_deref(),
            Some("missing-project")
        );
        let invalid_persisted_projection =
            load_canvas_global_view(&runtime_without_remembered, None)
                .expect("projection should load");
        assert_eq!(
            invalid_persisted_projection.current_project_id.as_deref(),
            Some(first.id.as_str())
        );

        cleanup(&runtime);
        cleanup(&runtime_without_remembered);
        clear_canvas_project_preference(&config_path);
    }

    #[test]
    fn load_canvas_global_view_scopes_graph_to_the_resolved_current_project() {
        let runtime = test_runtime();
        let config_path = test_config_path();
        clear_canvas_project_preference(&config_path);

        let alpha = insert_project_record(&runtime, "project-alpha", "Alpha");
        let alpha_work_item = insert_work_item_record(&runtime, "work-item-alpha", &alpha.id);
        let alpha_asset = insert_asset_record(&runtime, "asset-alpha", &alpha.id);

        let beta = insert_project_record(&runtime, "project-beta", "Beta");
        let beta_work_item = insert_work_item_record(&runtime, "work-item-beta", &beta.id);
        let beta_asset = insert_asset_record(&runtime, "asset-beta", &beta.id);

        write_canvas_project_preference(&config_path, Some(&beta.id));
        assert_eq!(
            load_remembered_canvas_project_id_from_path(&config_path).as_deref(),
            Some(beta.id.as_str())
        );

        let projection = load_canvas_global_view(&runtime, None).expect("projection should load");
        let node_ids = projection
            .graph
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            projection.current_project_id.as_deref(),
            Some(beta.id.as_str())
        );
        assert!(node_ids.contains(&beta.id.as_str()));
        assert!(node_ids.contains(&beta_work_item.id.as_str()));
        assert!(node_ids.contains(&beta_asset.id.as_str()));
        assert!(!node_ids.contains(&alpha.id.as_str()));
        assert!(!node_ids.contains(&alpha_work_item.id.as_str()));
        assert!(!node_ids.contains(&alpha_asset.id.as_str()));

        assert_eq!(projection.graph.edges.len(), 2);
        assert!(projection
            .graph
            .edges
            .iter()
            .all(|edge| edge.from == beta.id));
        assert!(projection.inspectors_by_node_id.contains_key(&beta.id));
        assert!(projection
            .inspectors_by_node_id
            .contains_key(&beta_work_item.id));
        assert!(projection
            .inspectors_by_node_id
            .contains_key(&beta_asset.id));
        assert!(!projection.inspectors_by_node_id.contains_key(&alpha.id));
        assert!(!projection
            .inspectors_by_node_id
            .contains_key(&alpha_work_item.id));
        assert!(!projection
            .inspectors_by_node_id
            .contains_key(&alpha_asset.id));

        cleanup(&runtime);
        clear_canvas_project_preference(&config_path);
    }

    #[test]
    fn load_canvas_global_view_includes_only_project_work_item_and_asset_nodes() {
        let runtime = test_runtime();
        let project = insert_project_record(&runtime, "project-1", "Project One");
        let work_item = insert_work_item_record(&runtime, "work-item-1", &project.id);
        let asset = insert_asset_record(&runtime, "asset-1", &project.id);
        let _source = insert_source_record(&runtime, "source-1");
        let _chunk = insert_chunk_record(&runtime, "chunk-1", "source-1");

        let projection =
            load_canvas_global_view(&runtime, Some(&project.id)).expect("projection should load");

        let node_ids = projection
            .graph
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>();
        assert!(node_ids.contains(&project.id.as_str()));
        assert!(node_ids.contains(&work_item.id.as_str()));
        assert!(node_ids.contains(&asset.id.as_str()));
        assert!(!node_ids.contains(&"source-1"));
        assert!(!node_ids.contains(&"chunk-1"));

        let node_types = projection
            .graph
            .nodes
            .iter()
            .map(|node| node.node_type.as_str())
            .collect::<Vec<_>>();
        assert!(node_types
            .iter()
            .all(|node_type| { matches!(*node_type, "project" | "work_item" | "asset") }));

        cleanup(&runtime);
    }

    #[test]
    fn load_canvas_global_view_builds_inspectors_only_for_project_work_item_and_asset_nodes() {
        let runtime = test_runtime();
        let project = insert_project_record(&runtime, "project-1", "Project One");
        let work_item = insert_work_item_record(&runtime, "work-item-1", &project.id);
        let asset = insert_asset_record(&runtime, "asset-1", &project.id);
        let _source = insert_source_record(&runtime, "source-1");
        let _chunk = insert_chunk_record(&runtime, "chunk-1", "source-1");

        let projection =
            load_canvas_global_view(&runtime, Some(&project.id)).expect("projection should load");

        assert!(projection.inspectors_by_node_id.contains_key(&project.id));
        assert!(projection.inspectors_by_node_id.contains_key(&work_item.id));
        assert!(projection.inspectors_by_node_id.contains_key(&asset.id));
        assert!(!projection.inspectors_by_node_id.contains_key("source-1"));
        assert!(!projection.inspectors_by_node_id.contains_key("chunk-1"));

        let inspector_types = projection
            .inspectors_by_node_id
            .values()
            .map(|inspector| inspector.node_type.as_str())
            .collect::<Vec<_>>();
        assert!(inspector_types
            .iter()
            .all(|node_type| { matches!(*node_type, "project" | "work_item" | "asset") }));

        cleanup(&runtime);
    }

    #[test]
    fn load_canvas_detail_view_assembles_project_detail() {
        let runtime = test_runtime();
        let project = insert_project_record(&runtime, "project-1", "Project One");
        let work_item = insert_work_item_record(&runtime, "work-item-1", &project.id);
        let asset = insert_asset_record(&runtime, "asset-1", &project.id);

        let projection = load_canvas_detail_view(&runtime, "project", &project.id, None)
            .expect("projection should load");

        assert_eq!(projection.focus_node_type, "project");
        assert_eq!(projection.focus_node_id, project.id);
        let node_ids = projection
            .graph
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>();
        assert!(node_ids.contains(&project.id.as_str()));
        assert!(node_ids.contains(&work_item.id.as_str()));
        assert!(node_ids.contains(&asset.id.as_str()));

        cleanup(&runtime);
    }

    #[test]
    fn load_canvas_detail_view_assembles_work_item_detail() {
        let runtime = test_runtime();
        let project = insert_project_record(&runtime, "project-1", "Project One");
        let work_item = insert_work_item_record(&runtime, "work-item-1", &project.id);

        let projection = load_canvas_detail_view(&runtime, "work_item", &work_item.id, None)
            .expect("projection should load");

        assert_eq!(projection.focus_node_type, "work_item");
        assert_eq!(projection.focus_node_id, work_item.id);
        let node_ids = projection
            .graph
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>();
        assert!(node_ids.contains(&project.id.as_str()));
        assert!(node_ids.contains(&work_item.id.as_str()));

        cleanup(&runtime);
    }

    #[test]
    fn load_canvas_detail_view_assembles_asset_detail() {
        let runtime = test_runtime();
        let project = insert_project_record(&runtime, "project-1", "Project One");
        let asset = insert_asset_record(&runtime, "asset-1", &project.id);

        let projection = load_canvas_detail_view(&runtime, "asset", &asset.id, None)
            .expect("projection should load");

        assert_eq!(projection.focus_node_type, "asset");
        assert_eq!(projection.focus_node_id, asset.id);
        let node_ids = projection
            .graph
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>();
        assert!(node_ids.contains(&project.id.as_str()));
        assert!(node_ids.contains(&asset.id.as_str()));

        cleanup(&runtime);
    }

    #[test]
    fn load_canvas_detail_view_assembles_source_detail_with_and_without_project_context() {
        let runtime = test_runtime();
        let project = insert_project_record(&runtime, "project-1", "Project One");
        let work_item = insert_work_item_record(&runtime, "work-item-1", &project.id);
        let asset = insert_asset_record(&runtime, "asset-1", &project.id);
        let source = insert_source_record(&runtime, "source-1");
        let chunk = insert_chunk_record(&runtime, "chunk-1", &source.id);

        let contextual_projection =
            load_canvas_detail_view(&runtime, "source", &source.id, Some(&project.id))
                .expect("projection should load");
        let contextual_node_ids = contextual_projection
            .graph
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>();
        assert!(contextual_node_ids.contains(&project.id.as_str()));
        assert!(contextual_node_ids.contains(&work_item.id.as_str()));
        assert!(contextual_node_ids.contains(&asset.id.as_str()));
        assert!(contextual_node_ids.contains(&source.id.as_str()));
        assert!(contextual_node_ids.contains(&chunk.id.as_str()));

        let plain_projection = load_canvas_detail_view(&runtime, "source", &source.id, None)
            .expect("projection should load");
        let plain_node_ids = plain_projection
            .graph
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>();
        assert!(!plain_node_ids.contains(&project.id.as_str()));
        assert!(plain_node_ids.contains(&source.id.as_str()));
        assert!(plain_node_ids.contains(&chunk.id.as_str()));

        cleanup(&runtime);
    }

    #[test]
    fn load_canvas_detail_view_assembles_chunk_detail_with_and_without_project_context() {
        let runtime = test_runtime();
        let project = insert_project_record(&runtime, "project-1", "Project One");
        let work_item = insert_work_item_record(&runtime, "work-item-1", &project.id);
        let asset = insert_asset_record(&runtime, "asset-1", &project.id);
        let source = insert_source_record(&runtime, "source-1");
        let chunk = insert_chunk_record(&runtime, "chunk-1", &source.id);

        let contextual_projection =
            load_canvas_detail_view(&runtime, "chunk", &chunk.id, Some(&project.id))
                .expect("projection should load");
        let contextual_node_ids = contextual_projection
            .graph
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>();
        assert!(contextual_node_ids.contains(&project.id.as_str()));
        assert!(contextual_node_ids.contains(&work_item.id.as_str()));
        assert!(contextual_node_ids.contains(&asset.id.as_str()));
        assert!(contextual_node_ids.contains(&source.id.as_str()));
        assert!(contextual_node_ids.contains(&chunk.id.as_str()));

        let plain_projection = load_canvas_detail_view(&runtime, "chunk", &chunk.id, None)
            .expect("projection should load");
        let plain_node_ids = plain_projection
            .graph
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>();
        assert!(!plain_node_ids.contains(&project.id.as_str()));
        assert!(plain_node_ids.contains(&source.id.as_str()));
        assert!(plain_node_ids.contains(&chunk.id.as_str()));

        cleanup(&runtime);
    }

    #[test]
    fn load_canvas_detail_view_does_not_add_synthetic_parent_project_to_source_or_chunk() {
        let runtime = test_runtime();
        let project = insert_project_record(&runtime, "project-1", "Project One");
        let source = insert_source_record(&runtime, "source-1");
        let chunk = insert_chunk_record(&runtime, "chunk-1", &source.id);

        let source_projection =
            load_canvas_detail_view(&runtime, "source", &source.id, Some(&project.id))
                .expect("projection should load");
        assert_eq!(
            source_projection.inspectors_by_node_id[&source.id]
                .fields
                .get("parentProject"),
            None
        );

        let chunk_projection =
            load_canvas_detail_view(&runtime, "chunk", &chunk.id, Some(&project.id))
                .expect("projection should load");
        assert_eq!(
            chunk_projection.inspectors_by_node_id[&chunk.id]
                .fields
                .get("parentProject"),
            None
        );

        cleanup(&runtime);
    }
}
