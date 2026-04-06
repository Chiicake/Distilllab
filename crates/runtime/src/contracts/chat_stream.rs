use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChatStreamPhase {
    Started,
    DecisionReady,
    ToolStarted,
    ToolFinished,
    RunCreated,
    RunStarted,
    RunStepStarted,
    RunStepFinished,
    RunProgress,
    RunFinished,
    AssistantStarted,
    AssistantChunk,
    Stopped,
    Completed,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunProgressPhase {
    Created,
    StateChanged,
    StepStarted,
    StepFinished,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LiveRunState {
    Queued,
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LiveToolStatus {
    Started,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LiveRunStepStatus {
    Started,
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveToolEvent {
    pub tool_call_id: String,
    pub tool_name: String,
    pub status: LiveToolStatus,
    pub arguments_text: Option<String>,
    pub result_text: Option<String>,
    pub summary: String,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveRunEvent {
    pub run_id: String,
    pub run_type: Option<String>,
    pub state: LiveRunState,
    pub progress_percent: Option<u8>,
    pub detail_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunProgressUpdate {
    pub phase: RunProgressPhase,
    pub run_id: String,
    pub run_type: String,
    pub run_state: LiveRunState,
    pub progress_percent: Option<u8>,
    pub step_key: Option<String>,
    pub step_summary: Option<String>,
    pub step_status: Option<LiveRunStepStatus>,
    pub step_index: Option<u32>,
    pub steps_total: Option<u32>,
    pub detail_text: Option<String>,
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
    pub tool_event: Option<LiveToolEvent>,
    pub run_event: Option<LiveRunEvent>,
    pub run_progress: Option<RunProgressUpdate>,
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
