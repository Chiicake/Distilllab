use schema::Source;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMaterializationInput {
    pub session_id: String,
    pub user_message: String,
    pub file_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMaterializationResult {
    pub sources: Vec<Source>,
    pub source_ids: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::{SourceMaterializationInput, SourceMaterializationResult};

    #[test]
    fn source_materialization_contract_types_are_local_to_runtime() {
        let input = SourceMaterializationInput {
            session_id: "session-1".to_string(),
            user_message: "Import these notes".to_string(),
            file_refs: vec!["notes/runtime.md".to_string()],
        };

        let result = SourceMaterializationResult {
            sources: vec![],
            source_ids: vec![],
        };

        assert_eq!(input.session_id, "session-1");
        assert!(result.sources.is_empty());
    }
}
