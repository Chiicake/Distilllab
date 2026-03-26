use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkItemType {
    Note,
}

impl WorkItemType {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkItemType::Note => "note",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "note" => Some(WorkItemType::Note),
            _ => None,
        }
    }
}

// WorkItem 表示从 Chunk 中抽取出的最小结构化项。
// Phase 1 先只做最小版，用于证明 Chunk -> WorkItem 这条链路成立。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItem {
    pub id: String,
    pub project_id: String,
    pub work_item_type: WorkItemType,
    pub title: String,
    pub summary: String,
}
