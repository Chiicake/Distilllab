use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunExecutionOutput {
    pub primary_asset_id: Option<String>,
    pub asset_ids: Vec<String>,
    pub work_item_ids: Vec<String>,
    pub execution_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunResultContext {
    pub run_id: String,
    pub run_type: String,
    pub status: String,
    pub asset_count: usize,
    pub work_item_count: usize,
    pub primary_asset_title: Option<String>,
    pub asset_summaries: Vec<String>,
    pub execution_summary: String,
}
