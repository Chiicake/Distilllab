use crate::{
    send_chat_completion_request, AgentError, LlmProviderConfig, OpenAiCompatibleChatMessage,
    OpenAiCompatibleChatRequest,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetExtractionChunkInput {
    pub chunk_id: String,
    pub title: String,
    pub summary: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetExtractionWorkItemInput {
    pub title: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetExtractionInput {
    pub run_id: String,
    pub distill_goal: String,
    pub project_id: String,
    pub project_name: String,
    pub project_summary: String,
    pub chunks: Vec<AssetExtractionChunkInput>,
    pub work_items: Vec<AssetExtractionWorkItemInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetDraft {
    pub title: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetExtractionOutput {
    pub assets: Vec<AssetDraft>,
}

pub fn asset_extraction_system_prompt() -> &'static str {
    r#"You are AssetExtractionAgent for Distilllab.

Your job is to convert distilled chunks and supporting work items into reusable recap assets.

Rules:
1. Return valid JSON only.
2. Do not include markdown fences.
3. Output must match this exact schema:

{
  "assets": [
    {
      "title": "string",
      "summary": "string"
    }
  ]
}

4. Assets should be durable recap artifacts, not raw notes or task lists.
5. Use chunks as the primary evidence.
6. Use work items as auxiliary context only.
7. Do not invent facts.
8. Prefer a small number of high-value assets.
9. Every asset must have a concise title and a clear summary.
10. The output will be stored as `AssetType::Insight`, so focus on insight-style outputs."#
}

pub fn build_asset_extraction_messages(
    input: &AssetExtractionInput,
) -> Vec<OpenAiCompatibleChatMessage> {
    let chunk_lines = input
        .chunks
        .iter()
        .map(|chunk| {
            format!(
                "- chunk_id: {}\n  title: {}\n  summary: {}\n  content: {}",
                chunk.chunk_id, chunk.title, chunk.summary, chunk.content
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let work_item_lines = input
        .work_items
        .iter()
        .map(|item| format!("- title: {}\n  summary: {}", item.title, item.summary))
        .collect::<Vec<_>>()
        .join("\n");

    vec![
        OpenAiCompatibleChatMessage {
            role: "system".to_string(),
            content: asset_extraction_system_prompt().to_string(),
        },
        OpenAiCompatibleChatMessage {
            role: "user".to_string(),
            content: format!(
                "run_id: {}\ndistill_goal: {}\nproject_id: {}\nproject_name: {}\nproject_summary: {}\n\nChunks:\n{}\n\nWork Items:\n{}",
                input.run_id,
                input.distill_goal,
                input.project_id,
                input.project_name,
                input.project_summary,
                chunk_lines,
                work_item_lines,
            ),
        },
    ]
}

pub fn validate_asset_extraction_output(
    output: AssetExtractionOutput,
) -> Result<AssetExtractionOutput, AgentError> {
    if output.assets.is_empty() {
        return Err(AgentError::Response(
            "asset extraction output must contain at least one asset".to_string(),
        ));
    }

    for asset in &output.assets {
        if asset.title.trim().is_empty() {
            return Err(AgentError::Response("asset title cannot be empty".to_string()));
        }
        if asset.summary.trim().is_empty() {
            return Err(AgentError::Response("asset summary cannot be empty".to_string()));
        }
    }

    Ok(output)
}

pub async fn run_asset_extraction_agent(
    client: &reqwest::Client,
    config: &LlmProviderConfig,
    input: &AssetExtractionInput,
) -> Result<AssetExtractionOutput, AgentError> {
    let request = OpenAiCompatibleChatRequest {
        model: config.model.clone(),
        messages: build_asset_extraction_messages(input),
        stream: None,
    };

    let response = send_chat_completion_request(client, config, &request).await?;
    let body = response.first_message_content().ok_or_else(|| {
        AgentError::Response("asset extraction response missing assistant content".to_string())
    })?;

    let parsed = serde_json::from_str::<AssetExtractionOutput>(body)
        .map_err(|error| AgentError::Response(format!("invalid asset extraction json: {}", error)))?;

    validate_asset_extraction_output(parsed)
}

#[cfg(test)]
mod tests {
    use super::{
        build_asset_extraction_messages, validate_asset_extraction_output, AssetDraft,
        AssetExtractionChunkInput, AssetExtractionInput, AssetExtractionOutput,
        AssetExtractionWorkItemInput,
    };

    fn fixture_input() -> AssetExtractionInput {
        AssetExtractionInput {
            run_id: "run-1".to_string(),
            distill_goal: "Distill these notes into reusable recap assets".to_string(),
            project_id: "project-1".to_string(),
            project_name: "Prototype Program".to_string(),
            project_summary: "Prototype planning, scope, and delivery work.".to_string(),
            chunks: vec![AssetExtractionChunkInput {
                chunk_id: "chunk-1".to_string(),
                title: "Prototype launch".to_string(),
                summary: "The prototype will ship next week.".to_string(),
                content: "Decision: ship next week. Action: finalize scope before launch.".to_string(),
            }],
            work_items: vec![AssetExtractionWorkItemInput {
                title: "Finalize scope".to_string(),
                summary: "Finalize the scope before launch.".to_string(),
            }],
        }
    }

    #[test]
    fn asset_extraction_messages_include_project_chunks_and_work_items() {
        let messages = build_asset_extraction_messages(&fixture_input());

        assert_eq!(messages.len(), 2);
        assert!(messages[0].content.contains("AssetExtractionAgent"));
        assert!(messages[1].content.contains("project_name: Prototype Program"));
        assert!(messages[1].content.contains("Prototype launch"));
        assert!(messages[1].content.contains("Finalize scope"));
    }

    #[test]
    fn validate_asset_extraction_output_accepts_non_empty_assets() {
        let output = AssetExtractionOutput {
            assets: vec![AssetDraft {
                title: "Prototype launch readiness".to_string(),
                summary: "The launch is gated by scope finalization and timeline coordination.".to_string(),
            }],
        };

        let validated = validate_asset_extraction_output(output.clone())
            .expect("asset output should validate");
        assert_eq!(validated, output);
    }

    #[test]
    fn validate_asset_extraction_output_rejects_empty_title() {
        let output = AssetExtractionOutput {
            assets: vec![AssetDraft {
                title: "   ".to_string(),
                summary: "A valid summary.".to_string(),
            }],
        };

        let error = validate_asset_extraction_output(output).expect_err("empty title should fail");
        assert!(error.to_string().contains("title cannot be empty"));
    }

    #[test]
    fn validate_asset_extraction_output_rejects_empty_summary() {
        let output = AssetExtractionOutput {
            assets: vec![AssetDraft {
                title: "Prototype launch readiness".to_string(),
                summary: " ".to_string(),
            }],
        };

        let error = validate_asset_extraction_output(output).expect_err("empty summary should fail");
        assert!(error.to_string().contains("summary cannot be empty"));
    }
}
