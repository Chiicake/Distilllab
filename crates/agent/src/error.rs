use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("agent configuration error: {0}")]
    Configuration(String),

    #[error("agent invocation error: {0}")]
    Invocation(String),

    #[error("agent response error: {0}")]
    Response(String),
}
