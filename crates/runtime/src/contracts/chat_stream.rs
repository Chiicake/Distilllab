use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChatStreamPhase {
    Started,
    DecisionReady,
    ToolStarted,
    ToolFinished,
    RunStarted,
    RunFinished,
    AssistantStarted,
    AssistantChunk,
    Completed,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatStreamEvent {
    pub request_id: String,
    pub session_id: String,
    pub phase: ChatStreamPhase,
    pub action_type: Option<String>,
    pub intent: Option<String>,
    pub chunk_text: Option<String>,
    pub status_text: Option<String>,
    pub assistant_text: Option<String>,
    pub timeline_text: Option<String>,
    pub error_text: Option<String>,
    pub created_run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMessageExecutionResult {
    pub session_id: String,
    pub action_type: String,
    pub intent: String,
    pub tool_name: Option<String>,
    pub tool_ok: Option<bool>,
    pub tool_summary: Option<String>,
    pub assistant_text: String,
    pub timeline_text: String,
    pub created_run_id: Option<String>,
    pub run_status: Option<String>,
}
