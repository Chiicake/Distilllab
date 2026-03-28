use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::{
        LlmProviderConfig, OpenAiCompatibleChatChoice, OpenAiCompatibleChatMessage,
        OpenAiCompatibleChatRequest, OpenAiCompatibleChatResponse,
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
}
