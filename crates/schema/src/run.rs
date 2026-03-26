use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RunState {
    Pending,
    Running,
    Completed,
    Failed,
}

impl RunState {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunState::Pending => "pending",
            RunState::Running => "running",
            RunState::Completed => "completed",
            RunState::Failed => "failed",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RunType {
    Demo,
    ImportAndDistill,
    Deepening,
    ComposeAndVerify,
}

impl RunType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunType::Demo => "demo",
            RunType::ImportAndDistill => "import_and_distill",
            RunType::Deepening => "deepening",
            RunType::ComposeAndVerify => "compose_and_verify",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RunRecord {
    pub id: String,
    pub run_type: RunType,
    pub status: RunState,
    pub created_at: String,
}
