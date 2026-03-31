//! Agent-common tool invocation contracts.
//!
//! This module defines the shared contracts for tool definitions, invocations,
//! and execution results that can be used across all agents in Distilllab.
//!
//! Design principles (A-now / B-ready):
//! - A-now: one planner turn emits one tool call, runtime executes it, next planner turn consumes result
//! - B-ready: contracts are shaped for future richer multi-tool / permission / scheduling model

use serde::{Deserialize, Serialize};

/// A tool definition describes a callable capability that agents can use.
///
/// Tool definitions are first-class objects, not string branches.
/// They include schema, policy, and execution shape metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Unique tool name used for invocation lookup.
    pub name: String,

    /// Human-readable description of what the tool does.
    pub description: String,

    /// JSON schema describing the expected input arguments.
    /// This is stored as a string for flexibility.
    pub input_schema_json: String,

    /// Whether the tool only reads data and has no side effects.
    pub is_read_only: bool,

    /// Whether the tool is safe to run concurrently with other tools.
    pub is_concurrency_safe: bool,

    /// Whether the tool requires user confirmation before execution.
    pub needs_confirmation: bool,
}

impl ToolDefinition {
    /// Create a new read-only tool definition with default settings.
    pub fn read_only(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            input_schema_json: "{}".to_string(),
            is_read_only: true,
            is_concurrency_safe: true,
            needs_confirmation: false,
        }
    }

    /// Create a new tool definition that may have side effects.
    pub fn with_side_effects(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            input_schema_json: "{}".to_string(),
            is_read_only: false,
            is_concurrency_safe: false,
            needs_confirmation: false,
        }
    }

    /// Set the input schema for this tool definition.
    pub fn with_input_schema(mut self, schema_json: &str) -> Self {
        self.input_schema_json = schema_json.to_string();
        self
    }

    /// Mark this tool as requiring user confirmation.
    pub fn requiring_confirmation(mut self) -> Self {
        self.needs_confirmation = true;
        self
    }
}

/// A tool invocation represents a request to execute a specific tool.
///
/// This is what the planner emits when it decides to call a tool.
/// In the A-now model, only a single ToolInvocation is attached to an action decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInvocation {
    /// The name of the tool to invoke.
    pub tool_name: String,

    /// JSON-encoded arguments to pass to the tool.
    pub arguments_json: String,

    /// Optional summary of why this tool is being called.
    pub reasoning_summary: Option<String>,

    /// Optional hint about what the planner expects to do with the result.
    pub expected_follow_up: Option<String>,
}

impl ToolInvocation {
    /// Create a new tool invocation with just the tool name.
    pub fn new(tool_name: &str) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            arguments_json: "{}".to_string(),
            reasoning_summary: None,
            expected_follow_up: None,
        }
    }

    /// Create a tool invocation with arguments.
    pub fn with_args(tool_name: &str, arguments_json: &str) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            arguments_json: arguments_json.to_string(),
            reasoning_summary: None,
            expected_follow_up: None,
        }
    }

    /// Add reasoning summary to the invocation.
    pub fn with_reasoning(mut self, reasoning: &str) -> Self {
        self.reasoning_summary = Some(reasoning.to_string());
        self
    }

    /// Add expected follow-up hint to the invocation.
    pub fn with_expected_follow_up(mut self, follow_up: &str) -> Self {
        self.expected_follow_up = Some(follow_up.to_string());
        self
    }
}

/// The result of executing a tool invocation.
///
/// This is what Runtime returns after executing a tool.
/// The next SessionAgent planning turn should consume this result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    /// The name of the tool that was executed.
    pub tool_name: String,

    /// Whether the tool execution succeeded.
    pub ok: bool,

    /// JSON-encoded output from the tool (if successful).
    pub output_json: Option<String>,

    /// Human-readable summary of the result for the planner.
    pub rendered_summary: Option<String>,

    /// Error message if the tool execution failed.
    pub error_message: Option<String>,

    /// Whether the planner should continue planning after this result.
    pub should_continue_planning: bool,
}

impl ToolExecutionResult {
    /// Create a successful tool execution result.
    pub fn success(tool_name: &str, output_json: &str, rendered_summary: &str) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            ok: true,
            output_json: Some(output_json.to_string()),
            rendered_summary: Some(rendered_summary.to_string()),
            error_message: None,
            should_continue_planning: true,
        }
    }

    /// Create a failed tool execution result.
    pub fn failure(tool_name: &str, error_message: &str) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            ok: false,
            output_json: None,
            rendered_summary: None,
            error_message: Some(error_message.to_string()),
            should_continue_planning: true,
        }
    }

    /// Create a successful result that signals the planner should stop.
    pub fn success_and_stop(tool_name: &str, output_json: &str, rendered_summary: &str) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            ok: true,
            output_json: Some(output_json.to_string()),
            rendered_summary: Some(rendered_summary.to_string()),
            error_message: None,
            should_continue_planning: false,
        }
    }

    /// Create a failed result that signals the planner should stop.
    pub fn failure_and_stop(tool_name: &str, error_message: &str) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            ok: false,
            output_json: None,
            rendered_summary: None,
            error_message: Some(error_message.to_string()),
            should_continue_planning: false,
        }
    }
}

/// Error type for tool registry operations.
#[derive(Debug, Clone)]
pub enum ToolRegistryError {
    /// The requested tool was not found in the registry.
    ToolNotFound(String),
    /// The tool is already registered.
    ToolAlreadyRegistered(String),
}

impl std::fmt::Display for ToolRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolRegistryError::ToolNotFound(name) => write!(f, "tool not found: {}", name),
            ToolRegistryError::ToolAlreadyRegistered(name) => {
                write!(f, "tool already registered: {}", name)
            }
        }
    }
}

impl std::error::Error for ToolRegistryError {}

/// A registry of available tool definitions.
///
/// The registry stays intentionally light in the first version:
/// - register built-in tool definitions
/// - lookup by tool_name
/// - expose allowed tools for the current agent
///
/// It does NOT build a scheduler into the registry.
#[derive(Debug, Default)]
pub struct ToolRegistry {
    tools: std::collections::HashMap<String, ToolDefinition>,
}

impl ToolRegistry {
    /// Create a new empty tool registry.
    pub fn new() -> Self {
        Self {
            tools: std::collections::HashMap::new(),
        }
    }

    /// Register a tool definition.
    pub fn register(&mut self, definition: ToolDefinition) -> Result<(), ToolRegistryError> {
        if self.tools.contains_key(&definition.name) {
            return Err(ToolRegistryError::ToolAlreadyRegistered(
                definition.name.clone(),
            ));
        }
        self.tools.insert(definition.name.clone(), definition);
        Ok(())
    }

    /// Look up a tool by name.
    pub fn get(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name)
    }

    /// Check if a tool exists in the registry.
    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get all registered tool names.
    pub fn tool_names(&self) -> Vec<&str> {
        self.tools.keys().map(String::as_str).collect()
    }

    /// Get tool definitions that are allowed for a given set of allowed keys.
    pub fn allowed_tools(&self, allowed_keys: &[String]) -> Vec<&ToolDefinition> {
        allowed_keys
            .iter()
            .filter_map(|key| self.tools.get(key))
            .collect()
    }
}

/// Create a registry with Distilllab's built-in tools.
///
/// These are the tools that Session Agent is allowed to use by default.
pub fn builtin_tool_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    // Read-only lookup tools
    registry
        .register(ToolDefinition::read_only(
            "list_sources",
            "List all sources in the current session or project",
        ))
        .expect("builtin tool registration should not fail");

    registry
        .register(ToolDefinition::read_only(
            "list_projects",
            "List all projects in the workspace",
        ))
        .expect("builtin tool registration should not fail");

    registry
        .register(ToolDefinition::read_only(
            "list_runs",
            "List recent runs in the current session",
        ))
        .expect("builtin tool registration should not fail");

    registry
        .register(ToolDefinition::read_only(
            "get_session",
            "Get details about the current session",
        ))
        .expect("builtin tool registration should not fail");

    registry
        .register(ToolDefinition::read_only(
            "get_project",
            "Get details about a specific project",
        ))
        .expect("builtin tool registration should not fail");

    registry
        .register(ToolDefinition::read_only(
            "get_asset",
            "Get details about a specific asset",
        ))
        .expect("builtin tool registration should not fail");

    registry
        .register(
            ToolDefinition::read_only(
                "search_memory",
                "Search the knowledge base for related notes and assets",
            )
            .with_input_schema(r#"{"query": "string", "limit": "number?"}"#),
        )
        .expect("builtin tool registration should not fail");

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_definition_can_be_created_as_read_only() {
        let tool = ToolDefinition::read_only("list_sources", "List all sources");

        assert_eq!(tool.name, "list_sources");
        assert!(tool.is_read_only);
        assert!(tool.is_concurrency_safe);
        assert!(!tool.needs_confirmation);
    }

    #[test]
    fn tool_definition_can_be_created_with_side_effects() {
        let tool = ToolDefinition::with_side_effects("create_source", "Create a new source");

        assert_eq!(tool.name, "create_source");
        assert!(!tool.is_read_only);
        assert!(!tool.is_concurrency_safe);
        assert!(!tool.needs_confirmation);
    }

    #[test]
    fn tool_definition_can_have_input_schema() {
        let tool = ToolDefinition::read_only("search_memory", "Search memory")
            .with_input_schema(r#"{"query": "string"}"#);

        assert_eq!(tool.input_schema_json, r#"{"query": "string"}"#);
    }

    #[test]
    fn tool_definition_can_require_confirmation() {
        let tool = ToolDefinition::with_side_effects("delete_source", "Delete a source")
            .requiring_confirmation();

        assert!(tool.needs_confirmation);
    }

    #[test]
    fn tool_invocation_can_be_created_with_name_only() {
        let invocation = ToolInvocation::new("list_sources");

        assert_eq!(invocation.tool_name, "list_sources");
        assert_eq!(invocation.arguments_json, "{}");
        assert!(invocation.reasoning_summary.is_none());
        assert!(invocation.expected_follow_up.is_none());
    }

    #[test]
    fn tool_invocation_can_be_created_with_arguments() {
        let invocation =
            ToolInvocation::with_args("search_memory", r#"{"query": "session notes"}"#);

        assert_eq!(invocation.tool_name, "search_memory");
        assert_eq!(invocation.arguments_json, r#"{"query": "session notes"}"#);
    }

    #[test]
    fn tool_invocation_can_include_reasoning_and_follow_up() {
        let invocation = ToolInvocation::new("list_sources")
            .with_reasoning("Need to see what sources are available")
            .with_expected_follow_up("Will select a source to distill");

        assert_eq!(
            invocation.reasoning_summary.as_deref(),
            Some("Need to see what sources are available")
        );
        assert_eq!(
            invocation.expected_follow_up.as_deref(),
            Some("Will select a source to distill")
        );
    }

    #[test]
    fn tool_execution_result_success_has_expected_fields() {
        let result = ToolExecutionResult::success(
            "list_sources",
            r#"[{"id": "source-1", "name": "Notes"}]"#,
            "Found 1 source: Notes",
        );

        assert_eq!(result.tool_name, "list_sources");
        assert!(result.ok);
        assert!(result.output_json.is_some());
        assert!(result.rendered_summary.is_some());
        assert!(result.error_message.is_none());
        assert!(result.should_continue_planning);
    }

    #[test]
    fn tool_execution_result_failure_has_expected_fields() {
        let result = ToolExecutionResult::failure("list_sources", "database connection failed");

        assert_eq!(result.tool_name, "list_sources");
        assert!(!result.ok);
        assert!(result.output_json.is_none());
        assert!(result.rendered_summary.is_none());
        assert_eq!(
            result.error_message.as_deref(),
            Some("database connection failed")
        );
        assert!(result.should_continue_planning);
    }

    #[test]
    fn tool_execution_result_can_signal_stop_planning() {
        let success_stop =
            ToolExecutionResult::success_and_stop("final_action", "{}", "Action completed");
        let failure_stop =
            ToolExecutionResult::failure_and_stop("critical_action", "unrecoverable error");

        assert!(!success_stop.should_continue_planning);
        assert!(!failure_stop.should_continue_planning);
    }

    #[test]
    fn tool_registry_can_register_and_lookup_tools() {
        let mut registry = ToolRegistry::new();
        let tool = ToolDefinition::read_only("list_sources", "List all sources");

        registry.register(tool).expect("should register");

        assert!(registry.contains("list_sources"));
        assert!(!registry.contains("unknown_tool"));

        let found = registry.get("list_sources").expect("should find tool");
        assert_eq!(found.name, "list_sources");
    }

    #[test]
    fn tool_registry_prevents_duplicate_registration() {
        let mut registry = ToolRegistry::new();
        let tool1 = ToolDefinition::read_only("list_sources", "List all sources");
        let tool2 = ToolDefinition::read_only("list_sources", "Duplicate tool");

        registry.register(tool1).expect("first should succeed");
        let result = registry.register(tool2);

        assert!(matches!(
            result,
            Err(ToolRegistryError::ToolAlreadyRegistered(_))
        ));
    }

    #[test]
    fn tool_registry_returns_allowed_tools_for_agent() {
        let registry = builtin_tool_registry();
        let allowed_keys = vec![
            "list_sources".to_string(),
            "list_projects".to_string(),
            "nonexistent".to_string(),
        ];

        let allowed = registry.allowed_tools(&allowed_keys);

        assert_eq!(allowed.len(), 2);
        assert!(allowed.iter().any(|t| t.name == "list_sources"));
        assert!(allowed.iter().any(|t| t.name == "list_projects"));
    }

    #[test]
    fn builtin_tool_registry_contains_session_agent_allowed_tools() {
        let registry = builtin_tool_registry();

        // These are the tools SessionAgent is allowed to use
        assert!(registry.contains("list_sources"));
        assert!(registry.contains("list_projects"));
        assert!(registry.contains("list_runs"));
        assert!(registry.contains("get_session"));
        assert!(registry.contains("get_project"));
        assert!(registry.contains("get_asset"));
        assert!(registry.contains("search_memory"));
    }

    #[test]
    fn tool_invocation_can_be_serialized_and_deserialized() {
        let invocation = ToolInvocation::with_args("search_memory", r#"{"query": "test"}"#)
            .with_reasoning("Looking for related notes");

        let json = serde_json::to_string(&invocation).expect("should serialize");
        let restored: ToolInvocation = serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(restored.tool_name, invocation.tool_name);
        assert_eq!(restored.arguments_json, invocation.arguments_json);
        assert_eq!(restored.reasoning_summary, invocation.reasoning_summary);
    }

    #[test]
    fn tool_execution_result_can_be_serialized_and_deserialized() {
        let result =
            ToolExecutionResult::success("list_sources", r#"[{"id": "s1"}]"#, "Found 1 source");

        let json = serde_json::to_string(&result).expect("should serialize");
        let restored: ToolExecutionResult =
            serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(restored.tool_name, result.tool_name);
        assert_eq!(restored.ok, result.ok);
        assert_eq!(restored.output_json, result.output_json);
    }

    #[test]
    fn tool_definition_can_be_serialized_and_deserialized() {
        let definition = ToolDefinition::read_only("list_sources", "List all sources")
            .with_input_schema(r#"{"filter": "string?"}"#);

        let json = serde_json::to_string(&definition).expect("should serialize");
        let restored: ToolDefinition = serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(restored.name, definition.name);
        assert_eq!(restored.is_read_only, definition.is_read_only);
        assert_eq!(restored.input_schema_json, definition.input_schema_json);
    }
}
