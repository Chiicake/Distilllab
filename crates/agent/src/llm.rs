use serde::{Deserialize, Serialize};
use crate::AgentError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    pub provider_kind: String,
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
}

impl LlmProviderConfig {
    pub fn chat_completions_url(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenAiCompatibleChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiCompatibleChatRequest {
    pub model: String,
    pub messages: Vec<OpenAiCompatibleChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiCompatibleChatChoice {
    pub message: OpenAiCompatibleChatMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiCompatibleChatResponse {
    pub choices: Vec<OpenAiCompatibleChatChoice>,
}

impl OpenAiCompatibleChatResponse {
    pub fn first_message_content(&self) -> Option<&str> {
        self.choices
            .first()
            .map(|choice| choice.message.content.as_str())
    }
}

pub async fn send_chat_completion_request(
    client: &reqwest::Client,
    config: &LlmProviderConfig,
    request: &OpenAiCompatibleChatRequest,
) -> Result<OpenAiCompatibleChatResponse, AgentError> {
    let url = config.chat_completions_url();
    let mut request_builder = client.post(url).json(request);
    if let Some(api_key) = &config.api_key {
        request_builder = request_builder.bearer_auth(api_key);
    }
    let response = request_builder
        .send()
        .await
        .map_err(|error| AgentError::Invocation(error.to_string()))?;
    let parsed_response = response
        .json::<OpenAiCompatibleChatResponse>()
        .await
        .map_err(|error| AgentError::Response(error.to_string()))?;
    Ok(parsed_response)
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use super::{
        send_chat_completion_request, LlmProviderConfig, OpenAiCompatibleChatChoice,
        OpenAiCompatibleChatMessage, OpenAiCompatibleChatRequest, OpenAiCompatibleChatResponse,
    };

    #[test]
    fn provider_config_builds_chat_completions_url_without_double_slash() {
        let config = LlmProviderConfig {
            provider_kind: "openai_compatible".to_string(),
            base_url: "https://api.example.com/v1/".to_string(),
            model: "gpt-test".to_string(),
            api_key: Some("test-key".to_string()),
        };

        assert_eq!(
            config.chat_completions_url(),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn openai_compatible_chat_message_preserves_role_and_content() {
        let message = OpenAiCompatibleChatMessage {
            role: "user".to_string(),
            content: "Import these notes".to_string(),
        };

        assert_eq!(message.role, "user");
        assert_eq!(message.content, "Import these notes");
    }

    #[test]
    fn chat_request_preserves_model_and_message_count() {
        let request = OpenAiCompatibleChatRequest {
            model: "gpt-test".to_string(),
            messages: vec![
                OpenAiCompatibleChatMessage {
                    role: "system".to_string(),
                    content: "You are a routing agent".to_string(),
                },
                OpenAiCompatibleChatMessage {
                    role: "user".to_string(),
                    content: "Import these notes".to_string(),
                },
            ],
        };

        assert_eq!(request.model, "gpt-test");
        assert_eq!(request.messages.len(), 2);
    }

    #[test]
    fn chat_response_returns_first_message_content() {
        let response = OpenAiCompatibleChatResponse {
            choices: vec![OpenAiCompatibleChatChoice {
                message: OpenAiCompatibleChatMessage {
                    role: "assistant".to_string(),
                    content: "{\"intent\":\"import_material\"}".to_string(),
                },
            }],
        };

        assert_eq!(
            response.first_message_content(),
            Some("{\"intent\":\"import_material\"}")
        );
    }

    #[tokio::test]
    async fn send_chat_completion_request_parses_successful_response() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let address = listener
            .local_addr()
            .expect("listener should have local addr");
        tokio::spawn(async move {
            let (mut stream, _) = listener
                .accept()
                .await
                .expect("server should accept connection");
            let mut buffer = [0_u8; 4096];
            let _ = stream
                .read(&mut buffer)
                .await
                .expect("server should read request");
            let response_body = r#"{
            "choices": [
                {
                    "message": {
                        "role": "assistant",
                        "content": "Hello from fake llm"
                    }
                }
            ]
        }"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("server should write response");
        });
        let base_url = format!("http://{}", address);
        let client = reqwest::Client::new();
        let config = LlmProviderConfig {
            provider_kind: "openai_compatible".to_string(),
            base_url,
            model: "gpt-test".to_string(),
            api_key: None,
        };
        let request = OpenAiCompatibleChatRequest {
            model: "gpt-test".to_string(),
            messages: vec![OpenAiCompatibleChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
        };
        let response = send_chat_completion_request(&client, &config, &request).await.expect("send failed");
        assert_eq!(
            response.first_message_content(),
            Some("Hello from fake llm")
        );
    }
}
