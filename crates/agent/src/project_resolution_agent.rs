use crate::{
    send_chat_completion_request, AgentError, LlmProviderConfig, OpenAiCompatibleChatMessage,
    OpenAiCompatibleChatRequest,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSummaryInput {
    pub project_id: String,
    pub name: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectResolutionChunkInput {
    pub chunk_id: String,
    pub title: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectResolutionWorkItemInput {
    pub title: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectResolutionInput {
    pub run_id: String,
    pub distill_goal: String,
    pub chunks: Vec<ProjectResolutionChunkInput>,
    pub work_items: Vec<ProjectResolutionWorkItemInput>,
    pub existing_projects: Vec<ProjectSummaryInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum ProjectResolutionDecision {
    UseExistingProject {
        project_id: String,
        reasoning_summary: String,
    },
    CreateNewProject {
        title: String,
        summary: String,
        reasoning_summary: String,
    },
}

pub fn project_resolution_system_prompt() -> &'static str {
    r#"You are ProjectResolutionAgent for Distilllab.

Your job is to decide whether the current distill outputs belong to an existing project or require a new project.

Rules:
1. Return valid JSON only.
2. Do not include markdown fences.
3. Output must match exactly one of these shapes:

For using an existing project:
{
  "decision": "use_existing_project",
  "project_id": "string",
  "reasoning_summary": "string"
}

For creating a new project:
{
  "decision": "create_new_project",
  "title": "string",
  "summary": "string",
  "reasoning_summary": "string"
}

4. Prefer an existing project only when the evidence clearly belongs to it.
5. Otherwise create a new project.
6. Do not invent project IDs that are not present in the provided project list.
7. Base the decision on chunks, work items, and the existing project summaries.
8. Keep reasoning concise and factual."#
}

pub fn build_project_resolution_messages(
    input: &ProjectResolutionInput,
) -> Vec<OpenAiCompatibleChatMessage> {
    let chunk_lines = input
        .chunks
        .iter()
        .map(|chunk| {
            format!(
                "- chunk_id: {}\n  title: {}\n  summary: {}",
                chunk.chunk_id, chunk.title, chunk.summary
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

    let project_lines = input
        .existing_projects
        .iter()
        .map(|project| {
            format!(
                "- project_id: {}\n  name: {}\n  summary: {}",
                project.project_id, project.name, project.summary
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    vec![
        OpenAiCompatibleChatMessage {
            role: "system".to_string(),
            content: project_resolution_system_prompt().to_string(),
        },
        OpenAiCompatibleChatMessage {
            role: "user".to_string(),
            content: format!(
                "run_id: {}\ndistill_goal: {}\n\nChunks:\n{}\n\nWork Items:\n{}\n\nExisting Projects:\n{}",
                input.run_id,
                input.distill_goal,
                chunk_lines,
                work_item_lines,
                project_lines,
            ),
        },
    ]
}

pub fn validate_project_resolution_decision(
    decision: ProjectResolutionDecision,
) -> Result<ProjectResolutionDecision, AgentError> {
    match &decision {
        ProjectResolutionDecision::UseExistingProject {
            project_id,
            reasoning_summary,
        } => {
            if project_id.trim().is_empty() {
                return Err(AgentError::Response(
                    "project_id cannot be empty".to_string(),
                ));
            }
            if reasoning_summary.trim().is_empty() {
                return Err(AgentError::Response(
                    "reasoning_summary cannot be empty".to_string(),
                ));
            }
        }
        ProjectResolutionDecision::CreateNewProject {
            title,
            summary,
            reasoning_summary,
        } => {
            if title.trim().is_empty() {
                return Err(AgentError::Response("title cannot be empty".to_string()));
            }
            if summary.trim().is_empty() {
                return Err(AgentError::Response("summary cannot be empty".to_string()));
            }
            if reasoning_summary.trim().is_empty() {
                return Err(AgentError::Response(
                    "reasoning_summary cannot be empty".to_string(),
                ));
            }
        }
    }

    Ok(decision)
}

pub async fn run_project_resolution_agent(
    client: &reqwest::Client,
    config: &LlmProviderConfig,
    input: &ProjectResolutionInput,
) -> Result<ProjectResolutionDecision, AgentError> {
    let request = OpenAiCompatibleChatRequest {
        model: config.model.clone(),
        messages: build_project_resolution_messages(input),
        stream: None,
    };

    let response = send_chat_completion_request(client, config, &request).await?;
    let body = response.first_message_content().ok_or_else(|| {
        AgentError::Response("project resolution response missing assistant content".to_string())
    })?;

    let parsed = serde_json::from_str::<ProjectResolutionDecision>(body)
        .map_err(|error| AgentError::Response(format!("invalid project resolution json: {}", error)))?;

    validate_project_resolution_decision(parsed)
}

#[cfg(test)]
mod tests {
    use super::{
        build_project_resolution_messages, validate_project_resolution_decision,
        ProjectResolutionChunkInput, ProjectResolutionDecision, ProjectResolutionInput,
        ProjectResolutionWorkItemInput, ProjectSummaryInput,
    };

    fn fixture_input() -> ProjectResolutionInput {
        ProjectResolutionInput {
            run_id: "run-1".to_string(),
            distill_goal: "Distill these notes into reusable recap assets".to_string(),
            chunks: vec![ProjectResolutionChunkInput {
                chunk_id: "chunk-1".to_string(),
                title: "Prototype launch".to_string(),
                summary: "The prototype will ship next week.".to_string(),
            }],
            work_items: vec![ProjectResolutionWorkItemInput {
                title: "Finalize scope".to_string(),
                summary: "Finalize the scope before launch.".to_string(),
            }],
            existing_projects: vec![ProjectSummaryInput {
                project_id: "project-1".to_string(),
                name: "Prototype Program".to_string(),
                summary: "Research and delivery for the prototype track.".to_string(),
            }],
        }
    }

    #[test]
    fn project_resolution_messages_include_projects_and_work_items() {
        let messages = build_project_resolution_messages(&fixture_input());

        assert_eq!(messages.len(), 2);
        assert!(messages[0].content.contains("ProjectResolutionAgent"));
        assert!(messages[1].content.contains("run_id: run-1"));
        assert!(messages[1].content.contains("Prototype Program"));
        assert!(messages[1].content.contains("Finalize scope"));
        assert!(messages[1].content.contains("Prototype launch"));
    }

    #[test]
    fn validate_project_resolution_accepts_existing_project_decision() {
        let decision = ProjectResolutionDecision::UseExistingProject {
            project_id: "project-1".to_string(),
            reasoning_summary: "The extracted items clearly belong to the prototype track.".to_string(),
        };

        let validated = validate_project_resolution_decision(decision.clone())
            .expect("existing project decision should validate");
        assert_eq!(validated, decision);
    }

    #[test]
    fn validate_project_resolution_rejects_empty_project_id() {
        let decision = ProjectResolutionDecision::UseExistingProject {
            project_id: "  ".to_string(),
            reasoning_summary: "The extracted items clearly belong to the prototype track.".to_string(),
        };

        let error = validate_project_resolution_decision(decision)
            .expect_err("empty project id should fail");
        assert!(error.to_string().contains("project_id cannot be empty"));
    }

    #[test]
    fn validate_project_resolution_rejects_empty_new_project_title() {
        let decision = ProjectResolutionDecision::CreateNewProject {
            title: " ".to_string(),
            summary: "A new project is needed for these recap assets.".to_string(),
            reasoning_summary: "No existing project matches this new body of work.".to_string(),
        };

        let error = validate_project_resolution_decision(decision)
            .expect_err("empty new project title should fail");
        assert!(error.to_string().contains("title cannot be empty"));
    }
}
