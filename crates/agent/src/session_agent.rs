use crate::AgentError;

#[derive(Debug, Clone)]
pub struct SessionAgentDecision {
    pub intent: String,
    pub primary_object_type: Option<String>,
    pub primary_object_id: Option<String>,
    pub action_type: String,
    pub reply_text: String,
    pub suggested_run_type: Option<String>,
    pub session_summary: Option<String>,
}

pub trait SessionAgent {
    fn decide(&self, user_message: &str) -> Result<SessionAgentDecision, AgentError>;
}
