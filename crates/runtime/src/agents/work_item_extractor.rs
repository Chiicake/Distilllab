use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkItemDraft {
    pub title: String,
    pub summary: String,
    pub work_item_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkItemExtractorOutput {
    pub work_items: Vec<WorkItemDraft>,
}
