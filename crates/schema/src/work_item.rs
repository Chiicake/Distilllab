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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItem {
    pub id: String,
    pub project_id: String,
    pub work_item_type: WorkItemType,
    pub title: String,
    pub summary: String,
}
