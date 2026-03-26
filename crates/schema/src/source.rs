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

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "document" => Some(SourceType::Document),
            "session" => Some(SourceType::Session),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub source_type: SourceType,
    pub title: String,
    pub created_at: String,
}
