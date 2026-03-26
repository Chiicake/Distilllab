use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    Document,
    Session,
}

impl SourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceType::Document => "document",
            SourceType::Session => "session",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRecord {
    pub id: String,
    pub source_type: SourceType,
    pub title: String,
    pub created_at: String,
}
