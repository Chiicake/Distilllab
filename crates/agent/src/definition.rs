#[derive(Debug, Clone)]
pub struct AgentDefinition {
    pub id: String,
    pub key: String,
    pub name: String,
    pub kind: String,
    pub description: String,
    pub responsibility_summary: String,
    pub status: String,
    pub system_prompt: String,
    pub default_model_profile: String,
    pub allowed_tool_keys: Vec<String>,
    pub input_object_types: Vec<String>,
    pub output_object_types: Vec<String>,
    pub can_create_run_types: Vec<String>,
}
