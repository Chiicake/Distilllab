use crate::app::AppRuntime;
use agent::{
    ToolDefinition, ToolExecutionResult, ToolInvocation, ToolRegistry, builtin_tool_registry,
};
use memory::asset_store::get_asset_by_id;
use memory::db::open_database;
use memory::migrations::run_migrations;
use memory::project_store::get_project_by_id;
use memory::session_store::get_session_by_id;

type RuntimeError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone)]
pub enum ToolExecutionError {
    ToolNotFound(String),
    InvalidArguments(String),
    ExecutionFailed(String),
    ConfirmationRequired(String),
}

impl std::fmt::Display for ToolExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolExecutionError::ToolNotFound(name) => write!(f, "tool not found: {name}"),
            ToolExecutionError::InvalidArguments(msg) => write!(f, "invalid tool arguments: {msg}"),
            ToolExecutionError::ExecutionFailed(msg) => write!(f, "tool execution failed: {msg}"),
            ToolExecutionError::ConfirmationRequired(name) => {
                write!(f, "tool requires confirmation: {name}")
            }
        }
    }
}

impl std::error::Error for ToolExecutionError {}

pub struct ToolExecutor {
    registry: ToolRegistry,
}

impl ToolExecutor {
    pub fn new() -> Self {
        Self {
            registry: builtin_tool_registry(),
        }
    }

    pub fn with_registry(registry: ToolRegistry) -> Self {
        Self { registry }
    }

    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    pub fn has_tool(&self, name: &str) -> bool {
        self.registry.contains(name)
    }

    pub fn get_tool_definition(&self, name: &str) -> Option<&ToolDefinition> {
        self.registry.get(name)
    }

    pub async fn execute(
        &self,
        runtime: &AppRuntime,
        invocation: &ToolInvocation,
    ) -> ToolExecutionResult {
        let definition = match self.registry.get(&invocation.tool_name) {
            Some(definition) => definition,
            None => {
                return ToolExecutionResult::failure(
                    &invocation.tool_name,
                    &ToolExecutionError::ToolNotFound(invocation.tool_name.clone()).to_string(),
                );
            }
        };

        if definition.needs_confirmation {
            return ToolExecutionResult::failure(
                &invocation.tool_name,
                &ToolExecutionError::ConfirmationRequired(invocation.tool_name.clone()).to_string(),
            );
        }

        self.dispatch_tool(runtime, definition, invocation).await
    }

    async fn dispatch_tool(
        &self,
        runtime: &AppRuntime,
        definition: &ToolDefinition,
        invocation: &ToolInvocation,
    ) -> ToolExecutionResult {
        match definition.name.as_str() {
            "list_sources" => self.execute_list_sources(runtime, invocation),
            "list_projects" => self.execute_list_projects(runtime, invocation),
            "list_runs" => self.execute_list_runs(runtime, invocation),
            "get_session" => self.execute_get_session(runtime, invocation),
            "get_project" => self.execute_get_project(runtime, invocation),
            "get_asset" => self.execute_get_asset(runtime, invocation),
            "search_memory" => self.execute_search_memory(invocation),
            _ => ToolExecutionResult::failure(
                &invocation.tool_name,
                &ToolExecutionError::ExecutionFailed(format!(
                    "no backing implementation for tool: {}",
                    invocation.tool_name
                ))
                .to_string(),
            ),
        }
    }

    fn execute_list_sources(
        &self,
        runtime: &AppRuntime,
        invocation: &ToolInvocation,
    ) -> ToolExecutionResult {
        match crate::services::list_sources(runtime) {
            Ok(sources) => ToolExecutionResult::success(
                &invocation.tool_name,
                &serde_json::to_string(&sources).unwrap_or_else(|_| "[]".to_string()),
                &format!("Found {} source(s)", sources.len()),
            ),
            Err(error) => ToolExecutionResult::failure(
                &invocation.tool_name,
                &ToolExecutionError::ExecutionFailed(format!("failed to list sources: {error}"))
                    .to_string(),
            ),
        }
    }

    fn execute_list_projects(
        &self,
        runtime: &AppRuntime,
        invocation: &ToolInvocation,
    ) -> ToolExecutionResult {
        match crate::services::list_projects(runtime) {
            Ok(projects) => ToolExecutionResult::success(
                &invocation.tool_name,
                &serde_json::to_string(&projects).unwrap_or_else(|_| "[]".to_string()),
                &format!("Found {} project(s)", projects.len()),
            ),
            Err(error) => ToolExecutionResult::failure(
                &invocation.tool_name,
                &ToolExecutionError::ExecutionFailed(format!("failed to list projects: {error}"))
                    .to_string(),
            ),
        }
    }

    fn execute_list_runs(
        &self,
        runtime: &AppRuntime,
        invocation: &ToolInvocation,
    ) -> ToolExecutionResult {
        match crate::services::list_runs(runtime) {
            Ok(runs) => ToolExecutionResult::success(
                &invocation.tool_name,
                &serde_json::to_string(&runs).unwrap_or_else(|_| "[]".to_string()),
                &format!("Found {} run(s)", runs.len()),
            ),
            Err(error) => ToolExecutionResult::failure(
                &invocation.tool_name,
                &ToolExecutionError::ExecutionFailed(format!("failed to list runs: {error}"))
                    .to_string(),
            ),
        }
    }

    fn execute_get_session(
        &self,
        runtime: &AppRuntime,
        invocation: &ToolInvocation,
    ) -> ToolExecutionResult {
        let session_id = match parse_required_string_arg(&invocation.arguments_json, "session_id") {
            Ok(value) => value,
            Err(error) => return ToolExecutionResult::failure(&invocation.tool_name, &error.to_string()),
        };

        match with_conn(runtime, |conn| Ok(get_session_by_id(conn, &session_id)?)) {
            Ok(Some(session)) => ToolExecutionResult::success(
                &invocation.tool_name,
                &serde_json::to_string(&session).unwrap_or_else(|_| "{}".to_string()),
                &format!("Session: {}", session.title),
            ),
            Ok(None) => ToolExecutionResult::failure(
                &invocation.tool_name,
                &ToolExecutionError::ExecutionFailed(format!("session not found: {session_id}"))
                    .to_string(),
            ),
            Err(error) => ToolExecutionResult::failure(&invocation.tool_name, &error.to_string()),
        }
    }

    fn execute_get_project(
        &self,
        runtime: &AppRuntime,
        invocation: &ToolInvocation,
    ) -> ToolExecutionResult {
        let project_id = match parse_required_string_arg(&invocation.arguments_json, "project_id") {
            Ok(value) => value,
            Err(error) => return ToolExecutionResult::failure(&invocation.tool_name, &error.to_string()),
        };

        match with_conn(runtime, |conn| Ok(get_project_by_id(conn, &project_id)?)) {
            Ok(Some(project)) => ToolExecutionResult::success(
                &invocation.tool_name,
                &serde_json::to_string(&project).unwrap_or_else(|_| "{}".to_string()),
                &format!("Project: {}", project.name),
            ),
            Ok(None) => ToolExecutionResult::failure(
                &invocation.tool_name,
                &ToolExecutionError::ExecutionFailed(format!("project not found: {project_id}"))
                    .to_string(),
            ),
            Err(error) => ToolExecutionResult::failure(&invocation.tool_name, &error.to_string()),
        }
    }

    fn execute_get_asset(
        &self,
        runtime: &AppRuntime,
        invocation: &ToolInvocation,
    ) -> ToolExecutionResult {
        let asset_id = match parse_required_string_arg(&invocation.arguments_json, "asset_id") {
            Ok(value) => value,
            Err(error) => return ToolExecutionResult::failure(&invocation.tool_name, &error.to_string()),
        };

        match with_conn(runtime, |conn| Ok(get_asset_by_id(conn, &asset_id)?)) {
            Ok(Some(asset)) => ToolExecutionResult::success(
                &invocation.tool_name,
                &serde_json::to_string(&asset).unwrap_or_else(|_| "{}".to_string()),
                &format!("Asset: {}", asset.title),
            ),
            Ok(None) => ToolExecutionResult::failure(
                &invocation.tool_name,
                &ToolExecutionError::ExecutionFailed(format!("asset not found: {asset_id}"))
                    .to_string(),
            ),
            Err(error) => ToolExecutionResult::failure(&invocation.tool_name, &error.to_string()),
        }
    }

    fn execute_search_memory(&self, invocation: &ToolInvocation) -> ToolExecutionResult {
        let query = match parse_required_string_arg(&invocation.arguments_json, "query") {
            Ok(value) => value,
            Err(error) => return ToolExecutionResult::failure(&invocation.tool_name, &error.to_string()),
        };

        ToolExecutionResult::success(
            &invocation.tool_name,
            &serde_json::json!({
                "query": query,
                "results": [],
                "implemented": false,
            })
            .to_string(),
            "Memory search is not yet implemented",
        )
    }
}

fn parse_required_string_arg(
    arguments_json: &str,
    key: &str,
) -> Result<String, ToolExecutionError> {
    let value = serde_json::from_str::<serde_json::Value>(arguments_json)
        .map_err(|error| ToolExecutionError::InvalidArguments(error.to_string()))?;

    value
        .get(key)
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
        .ok_or_else(|| ToolExecutionError::InvalidArguments(format!("missing required argument: {key}")))
}

fn with_conn<T>(
    runtime: &AppRuntime,
    f: impl FnOnce(&rusqlite::Connection) -> Result<T, RuntimeError>,
) -> Result<T, RuntimeError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;
    f(&conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::{build_demo_assets, create_demo_session, group_demo_project};
    use uuid::Uuid;

    fn create_test_runtime() -> AppRuntime {
        AppRuntime::new(format!("/tmp/distilllab-tool-executor-test-{}.db", Uuid::new_v4()))
    }

    #[tokio::test]
    async fn tool_executor_has_builtin_registry() {
        let executor = ToolExecutor::new();

        assert!(executor.has_tool("list_sources"));
        assert!(executor.has_tool("list_projects"));
        assert!(executor.has_tool("list_runs"));
        assert!(executor.has_tool("get_session"));
        assert!(executor.has_tool("get_project"));
        assert!(executor.has_tool("get_asset"));
        assert!(executor.has_tool("search_memory"));
    }

    #[tokio::test]
    async fn tool_executor_returns_failure_for_unknown_tool() {
        let runtime = create_test_runtime();
        let executor = ToolExecutor::new();
        let result = executor.execute(&runtime, &ToolInvocation::new("unknown_tool")).await;

        assert!(!result.ok);
        assert!(result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("tool not found"));

        let _ = std::fs::remove_file(&runtime.database_path);
    }

    #[tokio::test]
    async fn tool_executor_can_execute_list_tools() {
        let runtime = create_test_runtime();
        let executor = ToolExecutor::new();

        let sources = executor.execute(&runtime, &ToolInvocation::new("list_sources")).await;
        let projects = executor.execute(&runtime, &ToolInvocation::new("list_projects")).await;
        let runs = executor.execute(&runtime, &ToolInvocation::new("list_runs")).await;

        assert!(sources.ok);
        assert!(projects.ok);
        assert!(runs.ok);

        let _ = std::fs::remove_file(&runtime.database_path);
    }

    #[tokio::test]
    async fn tool_executor_can_get_session_by_id() {
        let runtime = create_test_runtime();
        let executor = ToolExecutor::new();
        let session = create_demo_session(&runtime).expect("session created");

        let invocation = ToolInvocation::with_args(
            "get_session",
            &serde_json::json!({ "session_id": session.id }).to_string(),
        );
        let result = executor.execute(&runtime, &invocation).await;

        assert!(result.ok);
        assert!(result
            .rendered_summary
            .as_deref()
            .unwrap_or_default()
            .contains("Demo Session"));

        let _ = std::fs::remove_file(&runtime.database_path);
    }

    #[tokio::test]
    async fn tool_executor_can_get_project_by_id() {
        let runtime = create_test_runtime();
        let executor = ToolExecutor::new();
        let (_, _, _, project) = group_demo_project(&runtime).expect("project grouped");

        let invocation = ToolInvocation::with_args(
            "get_project",
            &serde_json::json!({ "project_id": project.id }).to_string(),
        );
        let result = executor.execute(&runtime, &invocation).await;

        assert!(result.ok);
        assert!(result
            .rendered_summary
            .as_deref()
            .unwrap_or_default()
            .contains("Distilllab"));

        let _ = std::fs::remove_file(&runtime.database_path);
    }

    #[tokio::test]
    async fn tool_executor_can_get_asset_by_id() {
        let runtime = create_test_runtime();
        let executor = ToolExecutor::new();
        let (_, _, _, _, assets) = build_demo_assets(&runtime).expect("assets built");

        let invocation = ToolInvocation::with_args(
            "get_asset",
            &serde_json::json!({ "asset_id": assets[0].id }).to_string(),
        );
        let result = executor.execute(&runtime, &invocation).await;

        assert!(result.ok);
        assert!(result
            .rendered_summary
            .as_deref()
            .unwrap_or_default()
            .contains("Insight"));

        let _ = std::fs::remove_file(&runtime.database_path);
    }

    #[tokio::test]
    async fn tool_executor_rejects_missing_required_args() {
        let runtime = create_test_runtime();
        let executor = ToolExecutor::new();

        let session_result = executor.execute(&runtime, &ToolInvocation::new("get_session")).await;
        let project_result = executor.execute(&runtime, &ToolInvocation::new("get_project")).await;
        let asset_result = executor.execute(&runtime, &ToolInvocation::new("get_asset")).await;
        let search_result = executor.execute(&runtime, &ToolInvocation::new("search_memory")).await;

        assert!(!session_result.ok);
        assert!(!project_result.ok);
        assert!(!asset_result.ok);
        assert!(!search_result.ok);

        let _ = std::fs::remove_file(&runtime.database_path);
    }

    #[tokio::test]
    async fn tool_executor_returns_placeholder_search_result() {
        let runtime = create_test_runtime();
        let executor = ToolExecutor::new();
        let invocation = ToolInvocation::with_args(
            "search_memory",
            &serde_json::json!({ "query": "test" }).to_string(),
        );
        let result = executor.execute(&runtime, &invocation).await;

        assert!(result.ok);
        assert_eq!(result.tool_name, "search_memory");
        assert!(result.should_continue_planning);
        assert!(result
            .rendered_summary
            .as_deref()
            .unwrap_or_default()
            .contains("not yet implemented"));

        let _ = std::fs::remove_file(&runtime.database_path);
    }
}
