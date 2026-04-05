use crate::{
    send_chat_completion_request, AgentError, LlmProviderConfig, OpenAiCompatibleChatMessage,
    OpenAiCompatibleChatRequest,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkExtractionInput {
    pub run_id: String,
    pub source_id: String,
    pub source_type: String,
    pub source_title: String,
    pub source_text: String,
    pub distill_goal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChunkDraft {
    pub title: String,
    pub summary: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChunkExtractionOutput {
    pub chunks: Vec<ChunkDraft>,
}

pub fn chunk_extraction_system_prompt() -> &'static str {
    r#"You are ChunkExtractionAgent for Distilllab.

Your job is to transform one source document into a small set of high-quality semantic chunks for downstream distillation.

These chunks are not generic text splits.
They must be useful for:
- extracting structured work items
- extracting final recap assets
- preserving meaningful units of work, decisions, updates, blockers, and insights

Rules:
1. Return valid JSON only.
2. Do not include markdown fences.
3. Output must match this exact schema:

{
  \"chunks\": [
    {
      \"title\": \"string\",
      \"summary\": \"string\",
      \"content\": \"string\"
    }
  ]
}

4. Create chunks based on meaning, not arbitrary size.
5. Each chunk should represent one coherent unit of information.
6. Preserve important operational details:
   - decisions
   - action items
   - blockers
   - progress updates
   - findings
   - discussion themes
7. Do not invent facts that are not in the source.
8. Do not omit critical details just to shorten output.
9. Keep chunk titles concise and descriptive.
10. Keep summaries short, 1 sentence if possible.
11. Chunk content should be the cleaned, relevant source excerpt or rewrite of that unit, preserving the original meaning.
12. Prefer fewer, higher-quality chunks over many shallow chunks.
13. If the source is very short but meaningful, one chunk is acceptable.
14. If the source contains multiple distinct themes, split them into separate chunks.
15. You may discard source content that has no meaningful distillation value.
16. Distillation value means information that can contribute to:
   - decisions
   - work items
   - blockers
   - progress updates
   - findings
   - reusable recap assets
17. Do not preserve filler, repetition, or purely social text unless it changes the meaning of the work context.
18. The goal is not complete coverage. The goal is high-value semantic preservation for distillation."#
}

pub fn build_chunk_extraction_messages(input: &ChunkExtractionInput) -> Vec<OpenAiCompatibleChatMessage> {
    vec![
        OpenAiCompatibleChatMessage {
            role: "system".to_string(),
            content: chunk_extraction_system_prompt().to_string(),
        },
        OpenAiCompatibleChatMessage {
            role: "user".to_string(),
            content: format!(
                "Distill goal:\n{}\n\nSource metadata:\n- run_id: {}\n- source_id: {}\n- source_type: {}\n- source_title: {}\n\nSource text:\n{}",
                input.distill_goal,
                input.run_id,
                input.source_id,
                input.source_type,
                input.source_title,
                input.source_text,
            ),
        },
    ]
}

pub fn validate_chunk_extraction_output(
    output: ChunkExtractionOutput,
) -> Result<ChunkExtractionOutput, AgentError> {
    if output.chunks.is_empty() {
        return Err(AgentError::Response("chunk agent returned no chunks".to_string()));
    }

    let mut seen_content = std::collections::HashSet::new();
    for chunk in &output.chunks {
        if chunk.title.trim().is_empty() {
            return Err(AgentError::Response("chunk draft title cannot be empty".to_string()));
        }
        if chunk.summary.trim().is_empty() {
            return Err(AgentError::Response("chunk draft summary cannot be empty".to_string()));
        }
        if chunk.content.trim().is_empty() {
            return Err(AgentError::Response("chunk draft content cannot be empty".to_string()));
        }
        if !seen_content.insert(chunk.content.trim().to_string()) {
            return Err(AgentError::Response("chunk draft content must be unique per source".to_string()));
        }
    }

    Ok(output)
}

pub async fn run_chunk_extraction_agent(
    client: &reqwest::Client,
    config: &LlmProviderConfig,
    input: &ChunkExtractionInput,
) -> Result<ChunkExtractionOutput, AgentError> {
    let request = OpenAiCompatibleChatRequest {
        model: config.model.clone(),
        messages: build_chunk_extraction_messages(input),
        stream: None,
    };

    let response = send_chat_completion_request(client, config, &request).await?;
    let body = response
        .first_message_content()
        .ok_or_else(|| AgentError::Response("chunk agent response missing assistant content".to_string()))?;

    let parsed = serde_json::from_str::<ChunkExtractionOutput>(body)
        .map_err(|error| AgentError::Response(format!("invalid chunk agent json: {}", error)))?;

    validate_chunk_extraction_output(parsed)
}

#[cfg(test)]
mod tests {
    use super::{
        build_chunk_extraction_messages, validate_chunk_extraction_output, ChunkDraft,
        ChunkExtractionInput, ChunkExtractionOutput,
    };

    fn fixture_input() -> ChunkExtractionInput {
        ChunkExtractionInput {
            run_id: "run-1".to_string(),
            source_id: "source-1".to_string(),
            source_type: "document".to_string(),
            source_title: "notes.md".to_string(),
            source_text: "Decision: ship the first prototype next week.".to_string(),
            distill_goal: "Create recap-ready chunks".to_string(),
        }
    }

    #[test]
    fn chunk_extraction_messages_include_source_context() {
        let messages = build_chunk_extraction_messages(&fixture_input());

        assert_eq!(messages.len(), 2);
        assert!(messages[1].content.contains("run_id: run-1"));
        assert!(messages[1].content.contains("source_title: notes.md"));
        assert!(messages[1].content.contains("ship the first prototype next week"));
    }

    #[test]
    fn validate_chunk_extraction_output_accepts_non_empty_unique_chunks() {
        let output = ChunkExtractionOutput {
            chunks: vec![ChunkDraft {
                title: "Prototype decision".to_string(),
                summary: "A launch decision was made.".to_string(),
                content: "Ship the first prototype next week.".to_string(),
            }],
        };

        let validated = validate_chunk_extraction_output(output.clone()).expect("output should validate");
        assert_eq!(validated, output);
    }

    #[test]
    fn validate_chunk_extraction_output_rejects_empty_chunk_content() {
        let output = ChunkExtractionOutput {
            chunks: vec![ChunkDraft {
                title: "Prototype decision".to_string(),
                summary: "A launch decision was made.".to_string(),
                content: "   ".to_string(),
            }],
        };

        let error = validate_chunk_extraction_output(output).expect_err("empty chunk content should fail");
        assert!(error.to_string().contains("content cannot be empty"));
    }

    #[test]
    fn validate_chunk_extraction_output_rejects_duplicate_content() {
        let output = ChunkExtractionOutput {
            chunks: vec![
                ChunkDraft {
                    title: "Chunk A".to_string(),
                    summary: "A".to_string(),
                    content: "duplicate".to_string(),
                },
                ChunkDraft {
                    title: "Chunk B".to_string(),
                    summary: "B".to_string(),
                    content: "duplicate".to_string(),
                },
            ],
        };

        let error = validate_chunk_extraction_output(output).expect_err("duplicate chunk content should fail");
        assert!(error.to_string().contains("content must be unique"));
    }
}
