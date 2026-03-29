use crate::app::AppRuntime;
use agent::{
    BasicSessionAgent, LlmProviderConfig, LlmSessionAgent, SessionAgent, SessionAgentDecision,
    SessionAgentInput,
};
use chrono::Utc;
use memory::db::open_database;
use memory::migrations::run_migrations;
use memory::session_store::{insert_session, list_sessions as memory_list_sessions};
use schema::{Session, SessionStatus};
use uuid::Uuid;

pub async fn decide_demo_session_message(
    _runtime: &AppRuntime,
    user_message: &str,
) -> Result<SessionAgentDecision, Box<dyn std::error::Error>> {
    let now = Utc::now().to_string();

    let session = Session {
        id: "session-demo".to_string(),
        title: "Demo Session".to_string(),
        status: SessionStatus::Active,
        current_intent: "idle".to_string(),
        current_object_type: "none".to_string(),
        current_object_id: "none".to_string(),
        summary: "Demo session for session-agent decision".to_string(),
        started_at: now.clone(),
        updated_at: now.clone(),
        last_user_message_at: now.clone(),
        last_run_at: now.clone(),
        last_compacted_at: now,
        metadata_json: "{}".to_string(),
    };

    let input = SessionAgentInput {
        session,
        recent_messages: vec![],
        user_message: user_message.to_string(),
    };

    let session_agent = BasicSessionAgent;
    let decision = session_agent.decide(input).await?;

    Ok(decision)
}

pub async fn decide_llm_session_message(
    _runtime: &AppRuntime,
    user_message: &str,
) -> Result<SessionAgentDecision, Box<dyn std::error::Error>> {
    let base_url = std::env::var("DISTILLLAB_LLM_BASE_URL")?;
    let model = std::env::var("DISTILLLAB_LLM_MODEL")?;
    let api_key = std::env::var("DISTILLLAB_LLM_API_KEY").ok();

    let config = LlmProviderConfig {
        provider_kind: "openai_compatible".to_string(),
        base_url,
        model,
        api_key,
    };

    let now = Utc::now().to_string();
    let session = Session {
        id: "session-llm-demo".to_string(),
        title: "LLM Demo Session".to_string(),
        status: SessionStatus::Active,
        current_intent: "idle".to_string(),
        current_object_type: "none".to_string(),
        current_object_id: "none".to_string(),
        summary: "Demo session for llm-backed session-agent decision".to_string(),
        started_at: now.clone(),
        updated_at: now.clone(),
        last_user_message_at: now.clone(),
        last_run_at: now.clone(),
        last_compacted_at: now,
        metadata_json: "{}".to_string(),
    };

    let input = SessionAgentInput {
        session,
        recent_messages: vec![],
        user_message: user_message.to_string(),
    };

    let session_agent = LlmSessionAgent::new(config);
    let decision = session_agent.decide(input).await?;

    Ok(decision)
}

pub fn create_demo_session(runtime: &AppRuntime) -> Result<Session, Box<dyn std::error::Error>> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let now = Utc::now().to_string();
    let session = Session {
        id: format!("session-{}", Uuid::new_v4()),
        title: "Demo Session".to_string(),
        status: SessionStatus::Active,
        current_intent: "idle".to_string(),
        current_object_type: "none".to_string(),
        current_object_id: "none".to_string(),
        summary: "Demo session created".to_string(),
        started_at: now.clone(),
        updated_at: now.clone(),
        last_user_message_at: now.clone(),
        last_run_at: now.clone(),
        last_compacted_at: now,
        metadata_json: "{}".to_string(),
    };

    insert_session(&conn, &session)?;
    Ok(session)
}

pub fn list_sessions(runtime: &AppRuntime) -> Result<Vec<Session>, Box<dyn std::error::Error>> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let sessions = memory_list_sessions(&conn)?;
    Ok(sessions)
}

#[cfg(test)]
mod tests {
    use super::{decide_demo_session_message, decide_llm_session_message};
    use crate::app::AppRuntime;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn runtime_can_get_structured_decision_from_session_agent() {
        let runtime = AppRuntime::new("/tmp/distilllab-runtime-test.db".to_string());

        let decision = decide_demo_session_message(&runtime, "Hello Distilllab")
            .await
            .expect("runtime should receive a session agent decision");

        assert_eq!(decision.intent, "general_reply");
        assert_eq!(
            decision.reply_text,
            "Hello! I am ready to help with your Distilllab session."
        );
    }

    #[tokio::test]
    async fn runtime_can_get_llm_backed_decision_from_session_agent() {
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
                            "content": "Hello from runtime llm"
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

        unsafe {
            std::env::set_var("DISTILLLAB_LLM_BASE_URL", format!("http://{}", address));
            std::env::set_var("DISTILLLAB_LLM_MODEL", "gpt-test");
            std::env::remove_var("DISTILLLAB_LLM_API_KEY");
        }

        let runtime = AppRuntime::new("/tmp/distilllab-runtime-test-llm.db".to_string());

        let decision = decide_llm_session_message(&runtime, "Hello from runtime")
            .await
            .expect("runtime should receive an llm-backed session agent decision");

        assert_eq!(decision.intent, "llm_direct_reply");
        assert_eq!(decision.reply_text, "Hello from runtime llm");
    }
}
