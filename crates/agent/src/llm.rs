use crate::AgentError;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

fn truncate_error_text(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return "<empty body>".to_string();
    }

    let mut result = String::new();
    for (index, ch) in trimmed.chars().enumerate() {
        if index >= max_chars {
            result.push_str("...");
            return result;
        }
        result.push(ch);
    }

    result
}

fn extract_provider_error_detail(body: &str) -> Option<String> {
    let value = serde_json::from_str::<serde_json::Value>(body).ok()?;

    let error_value = value.get("error");
    if let Some(message) = error_value
        .and_then(|error| error.get("message"))
        .and_then(|message| message.as_str())
    {
        return Some(message.to_string());
    }

    if let Some(error_text) = error_value.and_then(|error| error.as_str()) {
        return Some(error_text.to_string());
    }

    if let Some(message) = value.get("message").and_then(|message| message.as_str()) {
        return Some(message.to_string());
    }

    if let Some(detail) = value.get("detail") {
        if let Some(detail_text) = detail.as_str() {
            return Some(detail_text.to_string());
        }

        if let Ok(detail_json) = serde_json::to_string(detail) {
            return Some(truncate_error_text(&detail_json, 400));
        }
    }

    None
}

fn format_http_error(status: reqwest::StatusCode, body: &str) -> String {
    let detail = extract_provider_error_detail(body)
        .unwrap_or_else(|| truncate_error_text(body, 400));
    format!("llm request failed with status {}: {}", status, detail)
}

fn format_decode_error(error: &serde_json::Error, body: &str) -> String {
    format!(
        "error decoding response body: {}; body: {}",
        error,
        truncate_error_text(body, 400)
    )
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiCompatibleChatChoice {
    pub message: OpenAiCompatibleChatMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiCompatibleChatResponse {
    pub choices: Vec<OpenAiCompatibleChatChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiCompatibleChatDelta {
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiCompatibleChatStreamChoice {
    pub delta: OpenAiCompatibleChatDelta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiCompatibleChatStreamChunk {
    pub choices: Vec<OpenAiCompatibleChatStreamChoice>,
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
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| AgentError::Response(error.to_string()))?;

    if !status.is_success() {
        return Err(AgentError::Response(format_http_error(status, &body)));
    }

    let parsed_response = serde_json::from_str::<OpenAiCompatibleChatResponse>(&body)
        .map_err(|error| AgentError::Response(format_decode_error(&error, &body)))?;
    Ok(parsed_response)
}

pub async fn stream_chat_completion_request<F>(
    client: &reqwest::Client,
    config: &LlmProviderConfig,
    request: &OpenAiCompatibleChatRequest,
    mut on_chunk: F,
) -> Result<String, AgentError>
where
    F: FnMut(&str),
{
    let url = config.chat_completions_url();
    let mut request_builder = client.post(url).json(request);
    if let Some(api_key) = &config.api_key {
        request_builder = request_builder.bearer_auth(api_key);
    }

    let response = request_builder
        .send()
        .await
        .map_err(|error| AgentError::Invocation(error.to_string()))?;

    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .map_err(|error| AgentError::Response(error.to_string()))?;
        return Err(AgentError::Response(format_http_error(status, &body)));
    }

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut aggregated = String::new();

    while let Some(next_chunk) = stream.next().await {
        let chunk = next_chunk.map_err(|error| AgentError::Response(error.to_string()))?;
        let chunk_text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&chunk_text);

        while let Some(line_end) = buffer.find('\n') {
            let line = buffer[..line_end].trim().to_string();
            buffer.drain(..=line_end);

            if !line.starts_with("data:") {
                continue;
            }

            let payload = line.trim_start_matches("data:").trim();
            if payload == "[DONE]" {
                return Ok(aggregated);
            }

            let parsed = serde_json::from_str::<OpenAiCompatibleChatStreamChunk>(payload)
                .map_err(|error| {
                    AgentError::Response(format!(
                        "stream chunk decode error: {}; payload: {}",
                        error,
                        truncate_error_text(payload, 200)
                    ))
                })?;

            let next_delta = parsed
                .choices
                .first()
                .and_then(|choice| choice.delta.content.as_deref())
                .unwrap_or("");

            if next_delta.is_empty() {
                continue;
            }

            on_chunk(next_delta);
            aggregated.push_str(next_delta);
        }
    }

    Ok(aggregated)
}

#[cfg(test)]
mod tests {
    use super::{
        LlmProviderConfig, OpenAiCompatibleChatChoice, OpenAiCompatibleChatMessage,
        OpenAiCompatibleChatRequest, OpenAiCompatibleChatResponse,
        send_chat_completion_request, stream_chat_completion_request,
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

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
            stream: None,
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
            model: config.model.clone(),
            messages: vec![OpenAiCompatibleChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            stream: None,
        };
        let response = send_chat_completion_request(&client, &config, &request)
            .await
            .expect("send failed");
        assert_eq!(
            response.first_message_content(),
            Some("Hello from fake llm")
        );
    }

    #[tokio::test]
    async fn stream_chat_completion_request_accumulates_sse_deltas() {
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

            let response_body = concat!(
                "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n",
                "data: {\"choices\":[{\"delta\":{\"content\":\" world\"}}]}\n",
                "data: [DONE]\n",
                "\n"
            );

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
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
            model: config.model.clone(),
            messages: vec![OpenAiCompatibleChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            stream: Some(true),
        };

        let mut observed_chunks = Vec::new();
        let full_text = stream_chat_completion_request(&client, &config, &request, |chunk| {
            observed_chunks.push(chunk.to_string());
        })
        .await
        .expect("stream request should succeed");

        assert_eq!(observed_chunks, vec!["Hello".to_string(), " world".to_string()]);
        assert_eq!(full_text, "Hello world");
    }

    #[tokio::test]
    async fn send_chat_completion_request_surfaces_http_error_message() {
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

            let response_body = r#"{"error":{"message":"model not found"}}"#;
            let response = format!(
                "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
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
            model: config.model.clone(),
            messages: vec![OpenAiCompatibleChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            stream: None,
        };

        let error = send_chat_completion_request(&client, &config, &request)
            .await
            .expect_err("request should fail");

        let message = format!("{}", error);
        assert!(message.contains("status 404"));
        assert!(message.contains("model not found"));
    }

    #[tokio::test]
    async fn send_chat_completion_request_surfaces_decode_body_snippet() {
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

            let response_body = "this-is-not-json";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
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
            model: config.model.clone(),
            messages: vec![OpenAiCompatibleChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            stream: None,
        };

        let error = send_chat_completion_request(&client, &config, &request)
            .await
            .expect_err("request should fail");

        let message = format!("{}", error);
        assert!(message.contains("error decoding response body"));
        assert!(message.contains("this-is-not-json"));
    }

    #[tokio::test]
    async fn stream_chat_completion_request_surfaces_http_error_message() {
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

            let response_body = r#"{"error":{"message":"forbidden"}}"#;
            let response = format!(
                "HTTP/1.1 403 Forbidden\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
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
            model: config.model.clone(),
            messages: vec![OpenAiCompatibleChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            stream: Some(true),
        };

        let error = stream_chat_completion_request(&client, &config, &request, |_| {})
            .await
            .expect_err("stream request should fail");

        let message = format!("{}", error);
        assert!(message.contains("status 403"));
        assert!(message.contains("forbidden"));
    }
}
