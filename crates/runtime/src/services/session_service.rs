use crate::app::AppRuntime;
use crate::contracts::{LlmSessionDebugRequest, SessionIntakePreview, SessionMessageRequest};
use crate::flows::build_import_and_distill_handoff_preview;
use agent::{
    BasicSessionAgent, LlmProviderConfig, LlmSessionAgent, SessionAgent, SessionAgentDecision,
    SessionAgentInput, SessionIntent,
};
use chrono::Utc;
use memory::db::open_database;
use memory::migrations::run_migrations;
use memory::session_message_store::{
    insert_session_message, list_session_messages_for_session,
};
use memory::session_store::{
    get_session_by_id, insert_session, list_sessions as memory_list_sessions, update_session,
};
use schema::{Session, SessionIntake, SessionMessage, SessionMessageRole, SessionStatus};
use uuid::Uuid;

type RuntimeError = Box<dyn std::error::Error + Send + Sync>;

fn build_demo_agent_session(session_id: &str, title: &str, summary: &str) -> Session {
    let now = Utc::now().to_string();

    Session {
        id: session_id.to_string(),
        title: title.to_string(),
        status: SessionStatus::Active,
        current_intent: "idle".to_string(),
        current_object_type: "none".to_string(),
        current_object_id: "none".to_string(),
        summary: summary.to_string(),
        started_at: now.clone(),
        updated_at: now.clone(),
        last_user_message_at: now.clone(),
        last_run_at: now.clone(),
        last_compacted_at: now,
        metadata_json: "{}".to_string(),
    }
}

fn normalize_optional_api_key(api_key: Option<String>) -> Option<String> {
    api_key.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn build_session_message(
    session_id: &str,
    run_id: Option<String>,
    message_type: &str,
    role: SessionMessageRole,
    content: String,
) -> SessionMessage {
    SessionMessage {
        id: format!("message-{}", Uuid::new_v4()),
        session_id: session_id.to_string(),
        run_id,
        message_type: message_type.to_string(),
        role,
        content,
        data_json: "{}".to_string(),
        created_at: Utc::now().to_string(),
    }
}

async fn decide_llm_session_message_with_provider_config(
    config: LlmProviderConfig,
    user_message: &str,
) -> Result<SessionAgentDecision, RuntimeError> {
    let session = build_demo_agent_session(
        "session-llm-demo",
        "LLM Demo Session",
        "Demo session for llm-backed session-agent decision",
    );

    let input = SessionAgentInput {
        session,
        recent_messages: vec![],
        intake: SessionIntake {
            session_id: "session-llm-demo".to_string(),
            user_message: user_message.to_string(),
            attachments: vec![],
            current_object_type: None,
            current_object_id: None,
        },
    };

    let session_agent = LlmSessionAgent::new(config);
    let decision = session_agent.decide(input).await?;

    Ok(decision)
}

async fn send_session_message_with_optional_provider_config(
    runtime: &AppRuntime,
    session_id: &str,
    user_message: &str,
    provider_config: Option<LlmProviderConfig>,
) -> Result<SessionAgentDecision, RuntimeError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let mut session = get_session_by_id(&conn, session_id)?.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("session not found: {session_id}"),
        )
    })?;

    let user_session_message = build_session_message(
        &session.id,
        None,
        "user_message",
        SessionMessageRole::User,
        user_message.to_string(),
    );
    insert_session_message(&conn, &user_session_message)?;

    let recent_messages = list_session_messages_for_session(&conn, &session.id)?;
    let input = SessionAgentInput {
        session: session.clone(),
        recent_messages,
        intake: SessionIntake {
            session_id: session.id.clone(),
            user_message: user_message.to_string(),
            attachments: vec![],
            current_object_type: match session.current_object_type.as_str() {
                "none" => None,
                other => Some(other.to_string()),
            },
            current_object_id: match session.current_object_id.as_str() {
                "none" => None,
                other => Some(other.to_string()),
            },
        },
    };

    let decision = if let Some(config) = provider_config {
        let session_agent = LlmSessionAgent::new(config);
        session_agent.decide(input).await?
    } else {
        let session_agent = BasicSessionAgent;
        session_agent.decide(input).await?
    };

    let assistant_message_type = match decision.action_type {
        agent::SessionActionType::DirectReply => "assistant_message",
        agent::SessionActionType::RequestClarification => "clarification_message",
        agent::SessionActionType::ToolCall => "system_message",
        agent::SessionActionType::CreateRun => "system_message",
    };

    let assistant_session_message = build_session_message(
        &session.id,
        None,
        assistant_message_type,
        SessionMessageRole::Assistant,
        decision.reply_text.clone(),
    );
    insert_session_message(&conn, &assistant_session_message)?;

    let now = Utc::now().to_string();
    session.current_intent = decision.intent.as_str().to_string();
    session.summary = decision
        .session_summary
        .clone()
        .unwrap_or_else(|| session.summary.clone());
    session.updated_at = now.clone();
    session.last_user_message_at = now;
    update_session(&conn, &session)?;

    Ok(decision)
}

fn llm_provider_config_from_env() -> Result<Option<LlmProviderConfig>, RuntimeError> {
    let base_url = match std::env::var("DISTILLLAB_LLM_BASE_URL") {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(error) => return Err(Box::new(error)),
    };

    let model = match std::env::var("DISTILLLAB_LLM_MODEL") {
        Ok(value) => value,
        Err(error) => return Err(Box::new(error)),
    };

    let api_key = normalize_optional_api_key(std::env::var("DISTILLLAB_LLM_API_KEY").ok());

    Ok(Some(LlmProviderConfig {
        provider_kind: "openai_compatible".to_string(),
        base_url,
        model,
        api_key,
    }))
}

pub async fn decide_demo_session_message(
    _runtime: &AppRuntime,
    user_message: &str,
) -> Result<SessionAgentDecision, RuntimeError> {
    let session = build_demo_agent_session(
        "session-demo",
        "Demo Session",
        "Demo session for session-agent decision",
    );

    let input = SessionAgentInput {
        session,
        recent_messages: vec![],
        intake: SessionIntake {
            session_id: "session-demo".to_string(),
            user_message: user_message.to_string(),
            attachments: vec![],
            current_object_type: None,
            current_object_id: None,
        },
    };

    let session_agent = BasicSessionAgent;
    let decision = session_agent.decide(input).await?;

    Ok(decision)
}

pub async fn decide_llm_session_message(
    _runtime: &AppRuntime,
    user_message: &str,
) -> Result<SessionAgentDecision, RuntimeError> {
    let config = llm_provider_config_from_env()?.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "DISTILLLAB_LLM_BASE_URL is not configured",
        )
    })?;

    decide_llm_session_message_with_provider_config(config, user_message).await
}

pub async fn decide_llm_session_message_with_config(
    _runtime: &AppRuntime,
    request: LlmSessionDebugRequest,
) -> Result<SessionAgentDecision, RuntimeError> {
    let config = LlmProviderConfig {
        provider_kind: request.provider_kind,
        base_url: request.base_url,
        model: request.model,
        api_key: normalize_optional_api_key(request.api_key),
    };

    decide_llm_session_message_with_provider_config(config, &request.user_message).await
}

pub async fn send_session_message(
    runtime: &AppRuntime,
    session_id: &str,
    user_message: &str,
) -> Result<SessionAgentDecision, RuntimeError> {
    send_session_message_with_optional_provider_config(
        runtime,
        session_id,
        user_message,
        llm_provider_config_from_env()?,
    )
    .await
}

pub async fn send_session_message_with_config(
    runtime: &AppRuntime,
    request: SessionMessageRequest,
) -> Result<SessionAgentDecision, RuntimeError> {
    let provider_config = LlmProviderConfig {
        provider_kind: request.provider_kind,
        base_url: request.base_url,
        model: request.model,
        api_key: normalize_optional_api_key(request.api_key),
    };

    send_session_message_with_optional_provider_config(
        runtime,
        &request.session_id,
        &request.user_message,
        Some(provider_config),
    )
    .await
}

pub async fn preview_session_intake(
    runtime: &AppRuntime,
    intake: SessionIntake,
) -> Result<SessionIntakePreview, RuntimeError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let session = get_session_by_id(&conn, &intake.session_id)?.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("session not found: {}", intake.session_id),
        )
    })?;

    let recent_messages = list_session_messages_for_session(&conn, &session.id)?;
    let input = SessionAgentInput {
        session,
        recent_messages,
        intake,
    };

    let session_agent = BasicSessionAgent;
    let decision = session_agent.decide(input).await?;

    let run_handoff_preview = if decision.intent == SessionIntent::DistillMaterial
        && decision.suggested_run_type.as_deref() == Some("import_and_distill")
    {
        Some(build_import_and_distill_handoff_preview(
            decision.primary_object_type.clone().or(Some("material".to_string())),
            decision.primary_object_id.clone(),
        ))
    } else {
        None
    };

    Ok(SessionIntakePreview {
        decision,
        run_handoff_preview,
    })
}

pub fn create_demo_session(runtime: &AppRuntime) -> Result<Session, RuntimeError> {
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

pub fn list_sessions(runtime: &AppRuntime) -> Result<Vec<Session>, RuntimeError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let sessions = memory_list_sessions(&conn)?;
    Ok(sessions)
}

pub fn list_session_messages(
    runtime: &AppRuntime,
    session_id: &str,
) -> Result<Vec<SessionMessage>, RuntimeError> {
    let conn = open_database(&runtime.database_path)?;
    run_migrations(&conn)?;

    let messages = list_session_messages_for_session(&conn, session_id)?;
    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::{
        LlmSessionDebugRequest, SessionMessageRequest, create_demo_session,
        decide_demo_session_message, decide_llm_session_message,
        decide_llm_session_message_with_config, preview_session_intake, send_session_message,
    };
    use crate::app::AppRuntime;
    use agent::SessionIntent;
    use schema::SessionIntake;
    use memory::db::open_database;
    use memory::session_message_store::list_session_messages_for_session;
    use memory::session_store::get_session_by_id;
    use std::sync::{Mutex, OnceLock};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use uuid::Uuid;

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct TestLlmEnvGuard {
        previous_base_url: Option<String>,
        previous_model: Option<String>,
        previous_api_key: Option<String>,
    }

    impl TestLlmEnvGuard {
        fn set(base_url: String, model: &str, api_key: Option<&str>) -> Self {
            let previous_base_url = std::env::var("DISTILLLAB_LLM_BASE_URL").ok();
            let previous_model = std::env::var("DISTILLLAB_LLM_MODEL").ok();
            let previous_api_key = std::env::var("DISTILLLAB_LLM_API_KEY").ok();

            unsafe {
                std::env::set_var("DISTILLLAB_LLM_BASE_URL", base_url);
                std::env::set_var("DISTILLLAB_LLM_MODEL", model);
                match api_key {
                    Some(value) => std::env::set_var("DISTILLLAB_LLM_API_KEY", value),
                    None => std::env::remove_var("DISTILLLAB_LLM_API_KEY"),
                }
            }

            Self {
                previous_base_url,
                previous_model,
                previous_api_key,
            }
        }

        fn clear() -> Self {
            let previous_base_url = std::env::var("DISTILLLAB_LLM_BASE_URL").ok();
            let previous_model = std::env::var("DISTILLLAB_LLM_MODEL").ok();
            let previous_api_key = std::env::var("DISTILLLAB_LLM_API_KEY").ok();

            unsafe {
                std::env::remove_var("DISTILLLAB_LLM_BASE_URL");
                std::env::remove_var("DISTILLLAB_LLM_MODEL");
                std::env::remove_var("DISTILLLAB_LLM_API_KEY");
            }

            Self {
                previous_base_url,
                previous_model,
                previous_api_key,
            }
        }
    }

    impl Drop for TestLlmEnvGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.previous_base_url {
                    Some(value) => std::env::set_var("DISTILLLAB_LLM_BASE_URL", value),
                    None => std::env::remove_var("DISTILLLAB_LLM_BASE_URL"),
                }
                match &self.previous_model {
                    Some(value) => std::env::set_var("DISTILLLAB_LLM_MODEL", value),
                    None => std::env::remove_var("DISTILLLAB_LLM_MODEL"),
                }
                match &self.previous_api_key {
                    Some(value) => std::env::set_var("DISTILLLAB_LLM_API_KEY", value),
                    None => std::env::remove_var("DISTILLLAB_LLM_API_KEY"),
                }
            }
        }
    }

    #[tokio::test]
    async fn runtime_can_get_structured_decision_from_session_agent() {
        let runtime = AppRuntime::new("/tmp/distilllab-runtime-test.db".to_string());

        let decision = decide_demo_session_message(&runtime, "Hello Distilllab")
            .await
            .expect("runtime should receive a session agent decision");

        assert_eq!(decision.intent, SessionIntent::GeneralReply);
        assert_eq!(
            decision.reply_text,
            "Hello! I am ready to help with your Distilllab session."
        );
    }

    #[tokio::test]
    async fn runtime_can_get_llm_backed_decision_from_session_agent() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
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

        let _env_guard = TestLlmEnvGuard::set(format!("http://{}", address), "gpt-test", None);

        let runtime = AppRuntime::new("/tmp/distilllab-runtime-test-llm.db".to_string());

        let decision = decide_llm_session_message(&runtime, "Hello from runtime")
            .await
            .expect("runtime should receive an llm-backed session agent decision");

        assert_eq!(decision.intent, SessionIntent::GeneralReply);
        assert_eq!(decision.reply_text, "Hello from runtime llm");
    }

    #[tokio::test]
    async fn runtime_can_get_llm_backed_decision_from_explicit_config() {
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
                            "content": "Hello from explicit config"
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

        let runtime = AppRuntime::new("/tmp/distilllab-runtime-test-llm-explicit.db".to_string());
        let request = LlmSessionDebugRequest {
            provider_kind: "openai_compatible".to_string(),
            base_url: format!("http://{}", address),
            model: "gpt-test".to_string(),
            api_key: Some(String::new()),
            user_message: "Hello from runtime explicit config".to_string(),
        };

        let decision = decide_llm_session_message_with_config(&runtime, request)
            .await
            .expect("runtime should receive an llm-backed session agent decision");

        assert_eq!(decision.intent, SessionIntent::GeneralReply);
        assert_eq!(decision.reply_text, "Hello from explicit config");
    }

    #[tokio::test]
    async fn send_session_message_persists_user_and_assistant_messages() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();
        let runtime = AppRuntime::new("/tmp/distilllab-runtime-session-flow.db".to_string());
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let reply = send_session_message(&runtime, &session.id, "Hello Distilllab")
            .await
            .expect("runtime should send a session message");

        assert_eq!(reply.intent, SessionIntent::GeneralReply);

        let conn = open_database(&runtime.database_path).expect("database should open");
        let messages = list_session_messages_for_session(&conn, &session.id)
            .expect("session messages should load");

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role.as_str(), "user");
        assert_eq!(messages[0].content, "Hello Distilllab");
        assert_eq!(messages[1].role.as_str(), "assistant");
        assert_eq!(
            messages[1].content,
            "Hello! I am ready to help with your Distilllab session."
        );
    }

    #[tokio::test]
    async fn send_session_message_updates_session_intent_and_summary() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();
        let runtime = AppRuntime::new("/tmp/distilllab-runtime-session-update.db".to_string());
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let reply = send_session_message(&runtime, &session.id, "Hello again")
            .await
            .expect("runtime should send a session message");

        let conn = open_database(&runtime.database_path).expect("database should open");
        let updated_session = get_session_by_id(&conn, &session.id)
            .expect("query should succeed")
            .expect("session should exist");

        assert_eq!(updated_session.current_intent, reply.intent.as_str());
        assert_eq!(updated_session.summary, "General session assistance");
    }

    #[tokio::test]
    async fn send_session_message_uses_llm_path_when_env_config_is_present() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
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
            let mut buffer = [0_u8; 8192];
            let bytes_read = stream
                .read(&mut buffer)
                .await
                .expect("server should read request");
            let request_text = String::from_utf8_lossy(&buffer[..bytes_read]);

            assert!(request_text.contains("Earlier question"));
            assert!(request_text.contains("Hello with context"));

            let response_body = r#"{
                "choices": [
                    {
                        "message": {
                            "role": "assistant",
                            "content": "LLM reply with history"
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

        let _env_guard = TestLlmEnvGuard::set(format!("http://{}", address), "gpt-test", None);

        let runtime = AppRuntime::new(
            format!(
                "/tmp/distilllab-runtime-session-llm-flow-{}.db",
                Uuid::new_v4()
            ),
        );
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let conn = open_database(&runtime.database_path).expect("database should open");
        let earlier_message = schema::SessionMessage {
            id: "message-seeded-1".to_string(),
            session_id: session.id.clone(),
            run_id: None,
            message_type: "user_message".to_string(),
            role: schema::SessionMessageRole::User,
            content: "Earlier question".to_string(),
            data_json: "{}".to_string(),
            created_at: "2026-03-29T00:00:00Z".to_string(),
        };
        memory::session_message_store::insert_session_message(&conn, &earlier_message)
            .expect("seed message should insert");
        drop(conn);

        let reply = send_session_message(&runtime, &session.id, "Hello with context")
            .await
            .expect("runtime should send llm-backed session message");

        assert_eq!(reply.intent, SessionIntent::GeneralReply);
        assert_eq!(reply.reply_text, "LLM reply with history");
    }

    #[tokio::test]
    async fn send_session_message_with_config_uses_llm_without_env_variables() {
        let _env_guard_lock = env_lock().lock().expect("env lock should acquire");
        let _env_guard = TestLlmEnvGuard::clear();

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
            let mut buffer = [0_u8; 8192];
            let bytes_read = stream
                .read(&mut buffer)
                .await
                .expect("server should read request");
            let request_text = String::from_utf8_lossy(&buffer[..bytes_read]);

            assert!(request_text.contains("Earlier explicit message"));
            assert!(request_text.contains("Current explicit message"));

            let response_body = r#"{
                "choices": [
                    {
                        "message": {
                            "role": "assistant",
                            "content": "LLM reply from explicit session config"
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

        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-session-explicit-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let conn = open_database(&runtime.database_path).expect("database should open");
        memory::session_message_store::insert_session_message(
            &conn,
            &schema::SessionMessage {
                id: "message-explicit-1".to_string(),
                session_id: session.id.clone(),
                run_id: None,
                message_type: "user_message".to_string(),
                role: schema::SessionMessageRole::User,
                content: "Earlier explicit message".to_string(),
                data_json: "{}".to_string(),
                created_at: "2026-03-29T00:00:00Z".to_string(),
            },
        )
        .expect("seed message should insert");
        drop(conn);

        let reply = super::send_session_message_with_config(
            &runtime,
            SessionMessageRequest {
                session_id: session.id.clone(),
                user_message: "Current explicit message".to_string(),
                provider_kind: "openai_compatible".to_string(),
                base_url: format!("http://{}", address),
                model: "gpt-test".to_string(),
                api_key: Some(String::new()),
            },
        )
        .await
        .expect("runtime should send llm-backed session message with explicit config");

        assert_eq!(reply.intent, SessionIntent::GeneralReply);
        assert_eq!(reply.reply_text, "LLM reply from explicit session config");
    }

    #[test]
    fn list_session_messages_returns_timeline_messages_for_session() {
        let runtime = AppRuntime::new(format!("/tmp/distilllab-runtime-list-messages-{}.db", Uuid::new_v4()));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let conn = open_database(&runtime.database_path).expect("database should open");
        memory::session_message_store::insert_session_message(
            &conn,
            &schema::SessionMessage {
                id: "message-list-1".to_string(),
                session_id: session.id.clone(),
                run_id: None,
                message_type: "user_message".to_string(),
                role: schema::SessionMessageRole::User,
                content: "Timeline hello".to_string(),
                data_json: "{}".to_string(),
                created_at: "2026-03-29T00:00:00Z".to_string(),
            },
        )
        .expect("seed message should insert");
        drop(conn);

        let messages = super::list_session_messages(&runtime, &session.id)
            .expect("runtime should list session messages");

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Timeline hello");
        assert_eq!(messages[0].role.as_str(), "user");
    }

    #[tokio::test]
    async fn preview_session_intake_returns_distill_run_handoff_with_planned_steps() {
        let runtime = AppRuntime::new(format!(
            "/tmp/distilllab-runtime-session-intake-preview-{}.db",
            Uuid::new_v4()
        ));
        let session = create_demo_session(&runtime).expect("runtime should create a demo session");

        let preview = preview_session_intake(
            &runtime,
            SessionIntake {
                session_id: session.id.clone(),
                user_message: "Please distill these work notes into Distilllab".to_string(),
                attachments: vec![],
                current_object_type: None,
                current_object_id: None,
            },
        )
        .await
        .expect("runtime should preview session intake");

        assert_eq!(preview.decision.intent, SessionIntent::DistillMaterial);

        let handoff = preview
            .run_handoff_preview
            .expect("distill intake should produce a handoff preview");

        assert_eq!(handoff.run_type, "import_and_distill");
        assert_eq!(handoff.planned_steps.len(), 3);
        assert_eq!(handoff.planned_steps[0].step_key, "materialize_sources");
        assert_eq!(handoff.planned_steps[1].step_key, "chunk_sources");
        assert_eq!(handoff.planned_steps[2].step_key, "extract_work_items");
    }
}
