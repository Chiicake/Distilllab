use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentRef {
    pub attachment_id: String,
    pub kind: String,
    pub name: String,
    pub mime_type: String,
    pub path_or_locator: String,
    pub size: u64,
    pub metadata_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionIntake {
    pub session_id: String,
    pub user_message: String,
    pub attachments: Vec<AttachmentRef>,
    pub current_object_type: Option<String>,
    pub current_object_id: Option<String>,
}
