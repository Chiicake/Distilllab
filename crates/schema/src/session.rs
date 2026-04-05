use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Idle,
    Archived,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Active => "active",
            SessionStatus::Idle => "idle",
            SessionStatus::Archived => "archived",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(SessionStatus::Active),
            "idle" => Some(SessionStatus::Idle),
            "archived" => Some(SessionStatus::Archived),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub manual_title: Option<String>,
    pub pinned: bool,
    pub status: SessionStatus,
    pub current_intent: String,
    pub current_object_type: String,
    pub current_object_id: String,
    pub summary: String,
    pub started_at: String,
    pub updated_at: String,
    pub last_user_message_at: String,
    pub last_run_at: String,
    pub last_compacted_at: String,
    pub metadata_json: String,
}
