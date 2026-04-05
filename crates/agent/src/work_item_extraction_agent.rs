use crate::{
    send_chat_completion_request, AgentError, LlmProviderConfig, OpenAiCompatibleChatMessage,
    OpenAiCompatibleChatRequest,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkItemExtractionChunkInput {
    pub chunk_id: String,
    pub title: String,
    pub summary: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkItemExtractionInput {
    pub run_id: String,
    pub distill_goal: String,
    pub chunks: Vec<WorkItemExtractionChunkInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkItemDraft {
    pub title: String,
    pub summary: String,
    pub work_item_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkItemExtractionOutput {
    pub work_items: Vec<WorkItemDraft>,
}

pub fn work_item_extraction_system_prompt() -> &'static str {
    r#"You are WorkItemExtractionAgent for Distilllab.

Your job is to extract structured work items from previously distilled chunks.

These work items are not project assignments and they are not final recap assets.
They are stable units of work context that can support later project resolution and asset extraction.

Rules:
1. Return valid JSON only.
2. Do not include markdown fences.
3. Output must match this exact schema:

{
  "work_items": [
    {
      "title": "string",
      "summary": "string",
      "work_item_type": "note"
    }
  ]
}

4. Only extract work items supported by the evidence in the chunks.
5. Do not invent facts.
6. Keep titles concise and summaries clear.
7. Every work item must use work_item_type = \"note\".
8. Prefer fewer, clearer work items over many overlapping ones.
9. Ignore chunks that provide no durable work-item value.
10. Extract only durable work-relevant items such as:
   - decisions
   - action directions
   - blockers
   - progress milestones
   - findings worth carrying forward"#
}

pub fn build_work_item_extraction_messages(
    input: &WorkItemExtractionInput,
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

    vec![
        OpenAiCompatibleChatMessage {
            role: "system".to_string(),
            content: work_item_extraction_system_prompt().to_string(),
        },
        OpenAiCompatibleChatMessage {
            role: "user".to_string(),
            content: format!(
                "run_id: {}\ndistill_goal: {}\n\nChunks:\n{}",
                input.run_id, input.distill_goal, chunk_lines
            ),
        },
    ]
}

pub fn validate_work_item_extraction_output(
    output: WorkItemExtractionOutput,
) -> Result<WorkItemExtractionOutput, AgentError> {
    let mut seen_pairs = std::collections::HashSet::new();

    for work_item in &output.work_items {
        if work_item.title.trim().is_empty() {
            return Err(AgentError::Response(
                "work item draft title cannot be empty".to_string(),
            ));
        }

        if work_item.summary.trim().is_empty() {
            return Err(AgentError::Response(
                "work item draft summary cannot be empty".to_string(),
            ));
        }

        if work_item.work_item_type.trim() != "note" {
            return Err(AgentError::Response(
                "work_item_type must be note".to_string(),
            ));
        }

        let dedupe_key = format!("{}::{}", work_item.title.trim(), work_item.summary.trim());
        if !seen_pairs.insert(dedupe_key) {
            return Err(AgentError::Response(
                "work item drafts must be unique".to_string(),
            ));
        }
    }

    Ok(output)
}

pub async fn run_work_item_extraction_agent(
    client: &reqwest::Client,
    config: &LlmProviderConfig,
    input: &WorkItemExtractionInput,
) -> Result<WorkItemExtractionOutput, AgentError> {
    let request = OpenAiCompatibleChatRequest {
        model: config.model.clone(),
        messages: build_work_item_extraction_messages(input),
        stream: None,
    };

    let response = send_chat_completion_request(client, config, &request).await?;
    let body = response.first_message_content().ok_or_else(|| {
        AgentError::Response("work item extraction response missing assistant content".to_string())
    })?;

    let parsed = serde_json::from_str::<WorkItemExtractionOutput>(body)
        .map_err(|error| AgentError::Response(format!("invalid work item extraction json: {}", error)))?;

    validate_work_item_extraction_output(parsed)
}

#[cfg(test)]
mod tests {
    use super::{
        build_work_item_extraction_messages, validate_work_item_extraction_output,
        WorkItemDraft, WorkItemExtractionChunkInput, WorkItemExtractionInput,
        WorkItemExtractionOutput,
    };

    fn fixture_input() -> WorkItemExtractionInput {
        WorkItemExtractionInput {
            run_id: "run-1".to_string(),
            distill_goal: "Extract durable work items from the chunks".to_string(),
            chunks: vec![WorkItemExtractionChunkInput {
                chunk_id: "chunk-1".to_string(),
                title: "Prototype launch decision".to_string(),
                summary: "The team decided to ship a prototype next week.".to_string(),
                content: "Decision: ship the first prototype next week. Action: finalize scope by Friday."
                    .to_string(),
            }],
        }
    }

    #[test]
    fn work_item_extraction_messages_include_goal_and_chunks() {
        let messages = build_work_item_extraction_messages(&fixture_input());

        assert_eq!(messages.len(), 2);
        assert!(messages[0].content.contains("WorkItemExtractionAgent"));
        assert!(messages[1].content.contains("run_id: run-1"));
        assert!(messages[1].content.contains("Prototype launch decision"));
        assert!(messages[1].content.contains("ship the first prototype next week"));
    }

    #[test]
    fn validate_work_item_extraction_output_accepts_note_items() {
        let output = WorkItemExtractionOutput {
            work_items: vec![WorkItemDraft {
                title: "Finalize prototype scope".to_string(),
                summary: "Scope needs to be finalized before the prototype ships next week."
                    .to_string(),
                work_item_type: "note".to_string(),
            }],
        };

        let validated =
            validate_work_item_extraction_output(output.clone()).expect("output should validate");
        assert_eq!(validated, output);
    }

    #[test]
    fn validate_work_item_extraction_output_rejects_empty_title() {
        let output = WorkItemExtractionOutput {
            work_items: vec![WorkItemDraft {
                title: "   ".to_string(),
                summary: "A valid summary.".to_string(),
                work_item_type: "note".to_string(),
            }],
        };

        let error = validate_work_item_extraction_output(output).expect_err("empty title should fail");
        assert!(error.to_string().contains("title cannot be empty"));
    }

    #[test]
    fn validate_work_item_extraction_output_rejects_invalid_type() {
        let output = WorkItemExtractionOutput {
            work_items: vec![WorkItemDraft {
                title: "Finalize prototype scope".to_string(),
                summary: "A valid summary.".to_string(),
                work_item_type: "task".to_string(),
            }],
        };

        let error = validate_work_item_extraction_output(output).expect_err("invalid work item type should fail");
        assert!(error.to_string().contains("work_item_type must be note"));
    }

    #[test]
    fn validate_work_item_extraction_output_rejects_duplicate_items() {
        let output = WorkItemExtractionOutput {
            work_items: vec![
                WorkItemDraft {
                    title: "Finalize prototype scope".to_string(),
                    summary: "A valid summary.".to_string(),
                    work_item_type: "note".to_string(),
                },
                WorkItemDraft {
                    title: "Finalize prototype scope".to_string(),
                    summary: "A valid summary.".to_string(),
                    work_item_type: "note".to_string(),
                },
            ],
        };

        let error = validate_work_item_extraction_output(output).expect_err("duplicate work items should fail");
        assert!(error.to_string().contains("must be unique"));
    }
}
