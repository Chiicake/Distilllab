use crate::{
    send_chat_completion_request, AgentError, LlmProviderConfig, OpenAiCompatibleChatMessage,
    OpenAiCompatibleChatRequest,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCompletionResultContext {
    pub run_id: String,
    pub run_type: String,
    pub status: String,
    pub asset_count: usize,
    pub work_item_count: usize,
    pub primary_asset_title: Option<String>,
    pub asset_summaries: Vec<String>,
    pub execution_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCompletionSummaryInput {
    pub session_id: String,
    pub user_message: String,
    pub run_result: RunCompletionResultContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunCompletionSummaryOutput {
    pub reply_text: String,
    pub session_summary: Option<String>,
}

pub fn run_completion_summarizer_system_prompt() -> &'static str {
    r#"You are RunCompletionSummarizer for Distilllab.

Your job is to turn a completed run result into a concise, user-facing assistant response.

Rules:
1. Return valid JSON only.
2. Do not include markdown fences.
3. Output must match this exact schema:

{
  "reply_text": "string",
  "session_summary": "string or null"
}

4. The reply must explain what completed, what outputs were produced, and what the user can do next.
5. Base the response only on the provided run result context.
6. Do not invent facts.
7. Keep the reply concise but informative.
8. `reply_text` must not be empty."#
}

pub fn build_run_completion_summary_messages(
    input: &RunCompletionSummaryInput,
) -> Vec<OpenAiCompatibleChatMessage> {
    let asset_summaries = input.run_result.asset_summaries.join("\n- ");

    vec![
        OpenAiCompatibleChatMessage {
            role: "system".to_string(),
            content: run_completion_summarizer_system_prompt().to_string(),
        },
        OpenAiCompatibleChatMessage {
            role: "user".to_string(),
            content: format!(
                "session_id: {}\nuser_message: {}\n\nRun Result:\n- run_id: {}\n- run_type: {}\n- status: {}\n- asset_count: {}\n- work_item_count: {}\n- primary_asset_title: {}\n- execution_summary: {}\n- asset_summaries:\n- {}",
                input.session_id,
                input.user_message,
                input.run_result.run_id,
                input.run_result.run_type,
                input.run_result.status,
                input.run_result.asset_count,
                input.run_result.work_item_count,
                input
                    .run_result
                    .primary_asset_title
                    .as_deref()
                    .unwrap_or("none"),
                input.run_result.execution_summary,
                asset_summaries,
            ),
        },
    ]
}

pub fn validate_run_completion_summary_output(
    output: RunCompletionSummaryOutput,
) -> Result<RunCompletionSummaryOutput, AgentError> {
    if output.reply_text.trim().is_empty() {
        return Err(AgentError::Response(
            "reply_text cannot be empty".to_string(),
        ));
    }

    Ok(output)
}

pub async fn run_run_completion_summarizer(
    client: &reqwest::Client,
    config: &LlmProviderConfig,
    input: &RunCompletionSummaryInput,
) -> Result<RunCompletionSummaryOutput, AgentError> {
    let request = OpenAiCompatibleChatRequest {
        model: config.model.clone(),
        messages: build_run_completion_summary_messages(input),
        stream: None,
    };

    let response = send_chat_completion_request(client, config, &request).await?;
    let body = response.first_message_content().ok_or_else(|| {
        AgentError::Response("run completion summary response missing assistant content".to_string())
    })?;

    let parsed = serde_json::from_str::<RunCompletionSummaryOutput>(body).map_err(|error| {
        AgentError::Response(format!("invalid run completion summary json: {}", error))
    })?;

    validate_run_completion_summary_output(parsed)
}

#[cfg(test)]
mod tests {
    use super::{
        build_run_completion_summary_messages, validate_run_completion_summary_output,
        RunCompletionResultContext, RunCompletionSummaryInput, RunCompletionSummaryOutput,
    };

    fn fixture_input() -> RunCompletionSummaryInput {
        RunCompletionSummaryInput {
            session_id: "session-1".to_string(),
            user_message: "Please distill these work notes into Distilllab".to_string(),
            run_result: RunCompletionResultContext {
                run_id: "run-1".to_string(),
                run_type: "import_and_distill".to_string(),
                status: "completed".to_string(),
                asset_count: 2,
                work_item_count: 1,
                primary_asset_title: Some("Prototype launch readiness".to_string()),
                asset_summaries: vec![
                    "The launch is gated by scope finalization and clear coordination before next week."
                        .to_string(),
                    "Scope clarity is the key stabilizer for this delivery cycle.".to_string(),
                ],
                execution_summary:
                    "materialized sources, created 1 chunks, extracted 1 work item drafts, resolved project Prototype Program, created 2 assets"
                        .to_string(),
            },
        }
    }

    #[test]
    fn run_completion_summary_messages_include_run_result_context() {
        let messages = build_run_completion_summary_messages(&fixture_input());

        assert_eq!(messages.len(), 2);
        assert!(messages[0].content.contains("RunCompletionSummarizer"));
        assert!(messages[1].content.contains("session_id: session-1"));
        assert!(messages[1].content.contains("asset_count: 2"));
        assert!(messages[1].content.contains("Prototype launch readiness"));
        assert!(messages[1].content.contains("Please distill these work notes into Distilllab"));
    }

    #[test]
    fn validate_run_completion_summary_output_accepts_reply_with_optional_session_summary() {
        let output = RunCompletionSummaryOutput {
            reply_text: "The distill run completed and produced 2 insight assets.".to_string(),
            session_summary: Some("Distill run completed with reusable outputs.".to_string()),
        };

        let validated = validate_run_completion_summary_output(output.clone())
            .expect("valid summary output should pass");
        assert_eq!(validated, output);
    }

    #[test]
    fn validate_run_completion_summary_output_rejects_empty_reply() {
        let output = RunCompletionSummaryOutput {
            reply_text: "   ".to_string(),
            session_summary: None,
        };

        let error = validate_run_completion_summary_output(output)
            .expect_err("empty reply should fail");
        assert!(error.to_string().contains("reply_text cannot be empty"));
    }
}
