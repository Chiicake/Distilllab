pub mod app;
pub mod agents;
pub mod config;
pub mod contracts;
pub mod flows;
pub mod runs;
pub mod services;

pub use app::AppRuntime;
pub use config::{
    AppConfig, CurrentModelSelection, DesktopUiConfig, DistilllabConfigSection, ModelConfigEntry,
    ProviderConfigEntry, ProviderOptions, ResolvedProviderModel, default_app_config_path,
    delete_provider_entry, import_providers_from_opencode_path, load_app_config_from_path,
    resolve_current_model_selection, resolve_current_provider_model, save_app_config_to_path,
    set_current_provider_model, upsert_provider_entry,
};
pub use contracts::{
    ChatStreamEvent, ChatStreamPhase, DistillRunStepPreview, LlmSessionDebugRequest,
    MaterializeDedupePolicy, MaterializeFailure, MaterializeSkip, MaterializeSourcesInput,
    MaterializeSourcesResult, MaterializedSourceRef, RunExecutionOutput,
    RunHandoffPreview, RunResultContext,
    RunProgressPhase, RunProgressUpdate, SessionIntakePreview,
    SessionMessageExecutionResult, SessionMessageRequest, SourceOriginKind,
};
pub use flows::{build_import_and_distill_handoff_preview, execute_materialize_sources};
pub use services::{
    build_demo_assets, chunk_demo_source, create_demo_run, create_demo_session,
    create_session, create_session_and_send_first_message_with_config,
    delete_failed_first_send_session, delete_session_and_related,
    create_demo_source, decide_llm_session_message_with_config, extract_demo_work_items,
    group_demo_project, list_assets, list_chunks_for_source, list_projects,
    list_session_messages, list_sessions, list_sources, list_work_items, list_runs,
    pin_session, rename_session,
    preview_session_intake, preview_session_intake_with_config, send_session_message,
    send_session_message_with_config, send_session_message_with_config_and_result,
    send_session_message_with_config_and_result_streaming,
    send_session_message_with_config_and_result_streaming_with_progress,
    ToolExecutionError, ToolExecutor,
};
