use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub key: String,
    pub name: String,
    pub description: String,
    pub allowed_tool_keys: Vec<String>,
    pub may_create_run_types: Vec<String>,
}

impl SkillDefinition {
    pub fn new(key: &str, name: &str, description: &str) -> Self {
        Self {
            key: key.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            allowed_tool_keys: vec![],
            may_create_run_types: vec![],
        }
    }

    pub fn with_allowed_tools(mut self, tool_keys: &[&str]) -> Self {
        self.allowed_tool_keys = tool_keys.iter().map(|value| value.to_string()).collect();
        self
    }

    pub fn with_run_types(mut self, run_types: &[&str]) -> Self {
        self.may_create_run_types = run_types.iter().map(|value| value.to_string()).collect();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillSelection {
    pub skill_key: String,
    pub reasoning_summary: Option<String>,
    pub expected_outcome: Option<String>,
}

impl SkillSelection {
    pub fn new(skill_key: &str) -> Self {
        Self {
            skill_key: skill_key.to_string(),
            reasoning_summary: None,
            expected_outcome: None,
        }
    }

    pub fn with_reasoning(mut self, reasoning_summary: &str) -> Self {
        self.reasoning_summary = Some(reasoning_summary.to_string());
        self
    }

    pub fn with_expected_outcome(mut self, expected_outcome: &str) -> Self {
        self.expected_outcome = Some(expected_outcome.to_string());
        self
    }
}

#[derive(Debug, Clone)]
pub enum SkillRegistryError {
    SkillNotFound(String),
    SkillAlreadyRegistered(String),
}

impl std::fmt::Display for SkillRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillRegistryError::SkillNotFound(key) => write!(f, "skill not found: {key}"),
            SkillRegistryError::SkillAlreadyRegistered(key) => {
                write!(f, "skill already registered: {key}")
            }
        }
    }
}

impl std::error::Error for SkillRegistryError {}

#[derive(Debug, Default)]
pub struct SkillRegistry {
    skills: std::collections::HashMap<String, SkillDefinition>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, definition: SkillDefinition) -> Result<(), SkillRegistryError> {
        if self.skills.contains_key(&definition.key) {
            return Err(SkillRegistryError::SkillAlreadyRegistered(
                definition.key.clone(),
            ));
        }

        self.skills.insert(definition.key.clone(), definition);
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<&SkillDefinition> {
        self.skills.get(key)
    }

    pub fn contains(&self, key: &str) -> bool {
        self.skills.contains_key(key)
    }

    pub fn allowed_skills(&self, allowed_keys: &[String]) -> Vec<&SkillDefinition> {
        allowed_keys
            .iter()
            .filter_map(|key| self.skills.get(key))
            .collect()
    }
}

pub fn builtin_skill_registry() -> SkillRegistry {
    let mut registry = SkillRegistry::new();

    registry
        .register(
            SkillDefinition::new(
                "import_and_distill_skill",
                "Import And Distill",
                "High-level strategy for importing and distilling raw material into reusable knowledge objects.",
            )
            .with_allowed_tools(&["list_sources", "search_memory"])
            .with_run_types(&["import_and_distill"]),
        )
        .expect("builtin skill registration should succeed");

    registry
        .register(
            SkillDefinition::new(
                "deepen_asset_skill",
                "Deepen Asset",
                "High-level strategy for deepening understanding around an existing asset or topic.",
            )
            .with_allowed_tools(&["get_asset", "search_memory"])
            .with_run_types(&["deepening"]),
        )
        .expect("builtin skill registration should succeed");

    registry
        .register(
            SkillDefinition::new(
                "compose_and_verify_skill",
                "Compose And Verify",
                "High-level strategy for composing an output and checking it against the current project context.",
            )
            .with_allowed_tools(&["get_project", "search_memory"])
            .with_run_types(&["compose_and_verify"]),
        )
        .expect("builtin skill registration should succeed");

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_selection_distinguishes_high_level_strategy_from_tool_call() {
        let selection = SkillSelection::new("compose_and_verify_skill")
            .with_reasoning("Need a reusable output strategy")
            .with_expected_outcome("Produce a checked draft");

        assert_eq!(selection.skill_key, "compose_and_verify_skill");
        assert_eq!(
            selection.reasoning_summary.as_deref(),
            Some("Need a reusable output strategy")
        );
        assert_eq!(
            selection.expected_outcome.as_deref(),
            Some("Produce a checked draft")
        );
    }

    #[test]
    fn skill_registry_can_register_and_lookup_skills() {
        let mut registry = SkillRegistry::new();
        registry
            .register(SkillDefinition::new("skill_a", "Skill A", "desc"))
            .expect("skill registration should succeed");

        assert!(registry.contains("skill_a"));
        assert!(registry.get("skill_a").is_some());
    }

    #[test]
    fn skill_registry_rejects_duplicate_keys() {
        let mut registry = SkillRegistry::new();
        registry
            .register(SkillDefinition::new("skill_a", "Skill A", "desc"))
            .expect("skill registration should succeed");

        let result = registry.register(SkillDefinition::new("skill_a", "Skill B", "desc"));
        assert!(matches!(
            result,
            Err(SkillRegistryError::SkillAlreadyRegistered(_))
        ));
    }

    #[test]
    fn builtin_skill_registry_contains_expected_high_level_strategies() {
        let registry = builtin_skill_registry();

        assert!(registry.contains("import_and_distill_skill"));
        assert!(registry.contains("deepen_asset_skill"));
        assert!(registry.contains("compose_and_verify_skill"));
    }

    #[test]
    fn skill_definition_and_selection_round_trip_via_json() {
        let definition = SkillDefinition::new("compose_and_verify_skill", "Compose", "desc")
            .with_allowed_tools(&["get_project"])
            .with_run_types(&["compose_and_verify"]);
        let selection = SkillSelection::new("compose_and_verify_skill")
            .with_reasoning("Need a higher-level strategy");

        let definition_json = serde_json::to_string(&definition).expect("definition serializes");
        let selection_json = serde_json::to_string(&selection).expect("selection serializes");

        let restored_definition: SkillDefinition =
            serde_json::from_str(&definition_json).expect("definition deserializes");
        let restored_selection: SkillSelection =
            serde_json::from_str(&selection_json).expect("selection deserializes");

        assert_eq!(restored_definition.key, definition.key);
        assert_eq!(restored_selection.skill_key, selection.skill_key);
    }
}
