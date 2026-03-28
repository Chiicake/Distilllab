use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionMessageRole {
    User,
    Assistant,
    System,
}

impl SessionMessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionMessageRole::User => "user",
            SessionMessageRole::Assistant => "assistant",
            SessionMessageRole::System => "system",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "user" => Some(SessionMessageRole::User),
            "assistant" => Some(SessionMessageRole::Assistant),
            "system" => Some(SessionMessageRole::System),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub id: String,
    pub session_id: String,
    pub run_id: Option<String>,
    pub message_type: String,
    pub role: SessionMessageRole,
    pub content: String,
    pub data_json: String,
    pub created_at: String,
}
