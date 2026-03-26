use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetType {
    Insight,
}

impl AssetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetType::Insight => "insight",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "insight" => Some(AssetType::Insight),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub project_id: String,
    pub asset_type: AssetType,
    pub title: String,
    pub summary: String,
}
