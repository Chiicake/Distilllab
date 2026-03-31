use schema::AttachmentRef;

#[derive(Debug, Clone)]
pub struct LlmSessionDebugRequest {
    pub provider_kind: String,
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    pub user_message: String,
}

#[derive(Debug, Clone)]
pub struct SessionMessageRequest {
    pub session_id: String,
    pub user_message: String,
    pub attachments: Vec<AttachmentRef>,
    pub provider_kind: String,
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
}
