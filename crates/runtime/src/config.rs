use directories::ProjectDirs;
use serde::de::Deserializer;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

type ConfigError = Box<dyn std::error::Error + Send + Sync>;

const DEFAULT_MAX_AGENT_CONCURRENCY: u8 = 4;
const MIN_MAX_AGENT_CONCURRENCY: u8 = 1;
const MAX_MAX_AGENT_CONCURRENCY: u8 = 16;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(rename = "$schema")]
    pub schema: Option<String>,
    #[serde(default)]
    pub provider: BTreeMap<String, ProviderConfigEntry>,
    #[serde(default)]
    pub distilllab: DistilllabConfigSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistilllabConfigSection {
    #[serde(rename = "currentProvider")]
    pub current_provider: Option<String>,
    #[serde(rename = "currentModel")]
    pub current_model: Option<String>,
    #[serde(rename = "desktopUi", default, skip_serializing_if = "Option::is_none")]
    pub desktop_ui: Option<DesktopUiConfig>,
    #[serde(
        rename = "maxAgentConcurrency",
        default = "default_max_agent_concurrency",
        deserialize_with = "deserialize_max_agent_concurrency"
    )]
    pub max_agent_concurrency: u8,
}

impl Default for DistilllabConfigSection {
    fn default() -> Self {
        Self {
            current_provider: None,
            current_model: None,
            desktop_ui: None,
            max_agent_concurrency: default_max_agent_concurrency(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct DesktopUiConfig {
    pub theme: String,
    pub locale: String,
    #[serde(rename = "showDebugPanel")]
    pub show_debug_panel: bool,
    #[serde(
        rename = "lastOpenedCanvasProjectId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub last_opened_canvas_project_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderConfigEntry {
    pub npm: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub options: ProviderOptions,
    #[serde(default)]
    pub models: BTreeMap<String, ModelConfigEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderOptions {
    #[serde(rename = "baseURL")]
    pub base_url: Option<String>,
    #[serde(rename = "apiKey")]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelConfigEntry {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub limit: Option<serde_json::Value>,
    #[serde(default)]
    pub options: Option<serde_json::Value>,
    #[serde(default)]
    pub variants: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentModelSelection {
    pub provider_id: String,
    pub model_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedProviderModel {
    pub provider_id: String,
    pub model_id: String,
    pub provider_type: String,
    pub base_url: String,
    pub api_key: Option<String>,
}

fn default_max_agent_concurrency() -> u8 {
    DEFAULT_MAX_AGENT_CONCURRENCY
}

fn deserialize_max_agent_concurrency<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    let value = i64::deserialize(deserializer)?;
    Ok(value.clamp(0, u8::MAX as i64) as u8)
}

fn normalize_max_agent_concurrency(value: u8) -> u8 {
    value.clamp(MIN_MAX_AGENT_CONCURRENCY, MAX_MAX_AGENT_CONCURRENCY)
}

fn normalize_app_config(config: &mut AppConfig) {
    config.distilllab.max_agent_concurrency =
        normalize_max_agent_concurrency(config.distilllab.max_agent_concurrency);
}

pub fn load_app_config_from_path(path: &Path) -> Result<AppConfig, ConfigError> {
    let content = fs::read_to_string(path)?;
    let mut config = serde_json::from_str::<AppConfig>(&content)?;
    normalize_app_config(&mut config);
    Ok(config)
}

pub fn default_app_config_path() -> Result<PathBuf, ConfigError> {
    let project_dirs = ProjectDirs::from("ai", "Distilllab", "Distilllab")
        .ok_or_else(|| std::io::Error::other("failed to resolve platform config directory"))?;

    Ok(project_dirs.config_dir().join("config.json"))
}

pub fn save_app_config_to_path(config: &AppConfig, path: &Path) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut normalized = config.clone();
    normalize_app_config(&mut normalized);

    let content = serde_json::to_string_pretty(&normalized)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn import_providers_from_opencode_path(
    source_path: &Path,
    target_path: &Path,
) -> Result<AppConfig, ConfigError> {
    let source_config = load_app_config_from_path(source_path)?;

    let mut target_config = if target_path.exists() {
        load_app_config_from_path(target_path)?
    } else {
        let config = AppConfig {
            schema: Some("https://opencode.ai/config.json".to_string()),
            ..Default::default()
        };
        save_app_config_to_path(&config, target_path)?;
        config
    };

    let had_current_provider = target_config.distilllab.current_provider.is_some();

    for (provider_id, provider_entry) in source_config.provider {
        let first_model_id = provider_entry.models.keys().next().cloned();
        target_config
            .provider
            .insert(provider_id.clone(), provider_entry);

        if !had_current_provider {
            target_config.distilllab.current_provider = Some(provider_id.clone());
            if target_config.distilllab.current_model.is_none() {
                target_config.distilllab.current_model = first_model_id;
            }
        }
    }

    if target_config.schema.is_none() {
        target_config.schema = Some("https://opencode.ai/config.json".to_string());
    }

    save_app_config_to_path(&target_config, target_path)?;
    Ok(target_config)
}

pub fn upsert_provider_entry(
    config_path: &Path,
    provider_id: &str,
    provider_entry: ProviderConfigEntry,
    current_model: Option<String>,
) -> Result<AppConfig, ConfigError> {
    let mut config = if config_path.exists() {
        load_app_config_from_path(config_path)?
    } else {
        AppConfig {
            schema: Some("https://opencode.ai/config.json".to_string()),
            ..Default::default()
        }
    };

    let selected_model = current_model.or_else(|| provider_entry.models.keys().next().cloned());
    config
        .provider
        .insert(provider_id.to_string(), provider_entry);
    config.distilllab.current_provider = Some(provider_id.to_string());
    config.distilllab.current_model = selected_model;
    if config.schema.is_none() {
        config.schema = Some("https://opencode.ai/config.json".to_string());
    }

    save_app_config_to_path(&config, config_path)?;
    Ok(config)
}

pub fn delete_provider_entry(
    config_path: &Path,
    provider_id: &str,
) -> Result<AppConfig, ConfigError> {
    let mut config = load_app_config_from_path(config_path)?;
    config.provider.remove(provider_id);

    let next_provider = config.provider.keys().next().cloned();
    let next_model = next_provider
        .as_ref()
        .and_then(|id| config.provider[id].models.keys().next().cloned());

    match next_provider {
        Some(provider) => {
            config.distilllab.current_provider = Some(provider);
            config.distilllab.current_model = next_model;
        }
        None => {
            config.distilllab.current_provider = None;
            config.distilllab.current_model = None;
        }
    }

    save_app_config_to_path(&config, config_path)?;
    Ok(config)
}

pub fn set_current_provider_model(
    config_path: &Path,
    provider_id: &str,
    model_id: &str,
) -> Result<AppConfig, ConfigError> {
    let mut config = load_app_config_from_path(config_path)?;

    let provider = config.provider.get(provider_id).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("provider not found: {provider_id}"),
        )
    })?;

    if !provider.models.contains_key(model_id) {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("model not found for provider {provider_id}: {model_id}"),
        )));
    }

    config.distilllab.current_provider = Some(provider_id.to_string());
    config.distilllab.current_model = Some(model_id.to_string());
    save_app_config_to_path(&config, config_path)?;
    Ok(config)
}

pub fn resolve_current_model_selection(
    config: &AppConfig,
) -> Result<CurrentModelSelection, ConfigError> {
    let provider_id = config.distilllab.current_provider.clone().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "distilllab.currentProvider is missing",
        )
    })?;

    let model_id = config.distilllab.current_model.clone().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "distilllab.currentModel is missing",
        )
    })?;

    if !config.provider.contains_key(&provider_id) {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("provider not found: {provider_id}"),
        )));
    }

    if !config.provider[&provider_id].models.contains_key(&model_id) {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("model not found for provider {provider_id}: {model_id}"),
        )));
    }

    Ok(CurrentModelSelection {
        provider_id,
        model_id,
    })
}

fn provider_type_from_npm(npm: Option<&str>) -> String {
    match npm {
        Some("@ai-sdk/openai-compatible") => "openai-compatible".to_string(),
        Some(other) => other.to_string(),
        None => "openai-compatible".to_string(),
    }
}

fn expand_file_reference(value: &str, config_path: &Path) -> Result<Option<String>, ConfigError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    if let Some(path_part) = trimmed
        .strip_prefix("{file:")
        .and_then(|rest| rest.strip_suffix('}'))
    {
        let secret_path = if path_part.starts_with("~/") {
            let home_dir = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .map(PathBuf::from)?;
            home_dir.join(path_part.trim_start_matches("~/"))
        } else {
            let candidate = PathBuf::from(path_part);
            if candidate.is_absolute() {
                candidate
            } else {
                config_path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(candidate)
            }
        };

        let secret = fs::read_to_string(secret_path)?;
        return Ok(Some(secret.trim().to_string()));
    }

    Ok(Some(trimmed.to_string()))
}

pub fn resolve_current_provider_model(
    config: &AppConfig,
    config_path: &Path,
) -> Result<ResolvedProviderModel, ConfigError> {
    let selection = resolve_current_model_selection(config)?;
    let provider = &config.provider[&selection.provider_id];

    let base_url = provider.options.base_url.clone().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "provider {} is missing options.baseURL",
                selection.provider_id
            ),
        )
    })?;

    let api_key = match provider.options.api_key.as_deref() {
        Some(value) => expand_file_reference(value, config_path)?,
        None => None,
    };

    Ok(ResolvedProviderModel {
        provider_id: selection.provider_id,
        model_id: selection.model_id,
        provider_type: provider_type_from_npm(provider.npm.as_deref()),
        base_url,
        api_key,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        import_providers_from_opencode_path, load_app_config_from_path,
        resolve_current_model_selection, resolve_current_provider_model, save_app_config_to_path,
        DesktopUiConfig,
    };
    use std::collections::BTreeMap;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn loads_opencode_compatible_provider_config_from_json_file() {
        let temp_dir =
            std::env::temp_dir().join(format!("distilllab-config-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let config_path = temp_dir.join("config.json");

        fs::write(
            &config_path,
            r#"{
                "$schema": "https://opencode.ai/config.json",
                "provider": {
                    "ice": {
                        "npm": "@ai-sdk/openai-compatible",
                        "name": "Ice",
                        "options": {
                            "baseURL": "https://ice.v.ua/v1",
                            "apiKey": "{file:secrets/ice.key}"
                        },
                        "models": {
                            "gpt-5.4": {
                                "name": "GPT-5.4"
                            }
                        }
                    }
                },
                "distilllab": {
                    "currentProvider": "ice",
                    "currentModel": "gpt-5.4"
                }
            }"#,
        )
        .expect("config file should be written");

        let config = load_app_config_from_path(&config_path).expect("config should load");

        assert_eq!(
            config.schema.as_deref(),
            Some("https://opencode.ai/config.json")
        );
        assert!(config.provider.contains_key("ice"));
        assert_eq!(config.provider["ice"].name, "Ice");
        assert_eq!(
            config.provider["ice"].options.base_url.as_deref(),
            Some("https://ice.v.ua/v1")
        );
    }

    #[test]
    fn resolves_current_provider_and_model_from_distilllab_section() {
        let temp_dir =
            std::env::temp_dir().join(format!("distilllab-config-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let config_path = temp_dir.join("config.json");

        fs::write(
            &config_path,
            r#"{
                "provider": {
                    "quickly": {
                        "npm": "@ai-sdk/openai-compatible",
                        "name": "Quickly",
                        "options": {
                            "baseURL": "https://sub.jlypx.de/v1"
                        },
                        "models": {
                            "gpt-5.4": {
                                "name": "GPT-5.4"
                            }
                        }
                    }
                },
                "distilllab": {
                    "currentProvider": "quickly",
                    "currentModel": "gpt-5.4"
                }
            }"#,
        )
        .expect("config file should be written");

        let config = load_app_config_from_path(&config_path).expect("config should load");
        let selection = resolve_current_model_selection(&config).expect("selection should resolve");

        assert_eq!(selection.provider_id, "quickly");
        assert_eq!(selection.model_id, "gpt-5.4");
    }

    #[test]
    fn resolves_current_provider_model_and_expands_file_api_key() {
        let temp_dir =
            std::env::temp_dir().join(format!("distilllab-config-test-{}", Uuid::new_v4()));
        fs::create_dir_all(temp_dir.join("secrets")).expect("secret dir should be created");
        let config_path = temp_dir.join("config.json");
        let secret_path = temp_dir.join("secrets/ice.key");
        fs::write(&secret_path, "secret-value\n").expect("secret file should be written");

        fs::write(
            &config_path,
            r#"{
                "provider": {
                    "ice": {
                        "npm": "@ai-sdk/openai-compatible",
                        "name": "Ice",
                        "options": {
                            "baseURL": "https://ice.v.ua/v1",
                            "apiKey": "{file:secrets/ice.key}"
                        },
                        "models": {
                            "gpt-5.4": {
                                "name": "GPT-5.4"
                            }
                        }
                    }
                },
                "distilllab": {
                    "currentProvider": "ice",
                    "currentModel": "gpt-5.4"
                }
            }"#,
        )
        .expect("config file should be written");

        let config = load_app_config_from_path(&config_path).expect("config should load");
        let resolved = resolve_current_provider_model(&config, &config_path)
            .expect("provider model should resolve");

        assert_eq!(resolved.provider_id, "ice");
        assert_eq!(resolved.model_id, "gpt-5.4");
        assert_eq!(resolved.provider_type, "openai-compatible");
        assert_eq!(resolved.base_url, "https://ice.v.ua/v1");
        assert_eq!(resolved.api_key.as_deref(), Some("secret-value"));
    }

    #[test]
    fn saves_config_back_to_json_file() {
        let temp_dir =
            std::env::temp_dir().join(format!("distilllab-config-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let config_path = temp_dir.join("config.json");

        let mut config = super::AppConfig::default();
        config.schema = Some("https://opencode.ai/config.json".to_string());
        config.distilllab.current_provider = Some("ice".to_string());
        config.distilllab.current_model = Some("gpt-5.4".to_string());
        config.provider.insert(
            "ice".to_string(),
            super::ProviderConfigEntry {
                npm: Some("@ai-sdk/openai-compatible".to_string()),
                name: "Ice".to_string(),
                options: super::ProviderOptions {
                    base_url: Some("https://ice.v.ua/v1".to_string()),
                    api_key: Some("{file:secrets/ice.key}".to_string()),
                },
                models: BTreeMap::from([(
                    "gpt-5.4".to_string(),
                    super::ModelConfigEntry {
                        name: "GPT-5.4".to_string(),
                        ..Default::default()
                    },
                )]),
            },
        );

        save_app_config_to_path(&config, &config_path).expect("config should save");
        let saved = fs::read_to_string(&config_path).expect("config file should exist");

        assert!(saved.contains("\"provider\""));
        assert!(saved.contains("\"ice\""));
        assert!(saved.contains("\"currentProvider\": \"ice\""));
    }

    #[test]
    fn computes_platform_config_path_inside_distilllab_directory() {
        let path = super::default_app_config_path().expect("default path should resolve");

        assert_eq!(
            path.file_name().and_then(|x| x.to_str()),
            Some("config.json")
        );
        assert!(path.to_string_lossy().to_lowercase().contains("distilllab"));
    }

    #[test]
    fn imports_provider_entries_from_opencode_config_file() {
        let temp_dir =
            std::env::temp_dir().join(format!("distilllab-config-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let source_path = temp_dir.join("opencode.json");
        let target_path = temp_dir.join("distilllab-config.json");

        fs::write(
            &source_path,
            r#"{
                "$schema": "https://opencode.ai/config.json",
                "provider": {
                    "ice": {
                        "npm": "@ai-sdk/openai-compatible",
                        "name": "Ice",
                        "options": {
                            "baseURL": "https://ice.v.ua/v1",
                            "apiKey": "{file:~/.config/opencode/ice.key}"
                        },
                        "models": {
                            "gpt-5.4": {
                                "name": "GPT-5.4"
                            }
                        }
                    }
                }
            }"#,
        )
        .expect("source config should be written");

        let mut target = super::AppConfig::default();
        target.schema = Some("https://opencode.ai/config.json".to_string());
        super::save_app_config_to_path(&target, &target_path).expect("target config should save");

        import_providers_from_opencode_path(&source_path, &target_path)
            .expect("providers should import from opencode file");

        let imported =
            load_app_config_from_path(&target_path).expect("imported config should load");
        assert!(imported.provider.contains_key("ice"));
        assert_eq!(imported.provider["ice"].name, "Ice");
        assert_eq!(
            imported.provider["ice"].options.base_url.as_deref(),
            Some("https://ice.v.ua/v1")
        );
        assert!(imported.provider["ice"].models.contains_key("gpt-5.4"));
    }

    #[test]
    fn upserts_provider_entry_and_sets_current_selection() {
        let temp_dir =
            std::env::temp_dir().join(format!("distilllab-config-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let config_path = temp_dir.join("config.json");

        let config = super::upsert_provider_entry(
            &config_path,
            "copilot",
            super::ProviderConfigEntry {
                npm: Some("@ai-sdk/openai-compatible".to_string()),
                name: "GitHub Copilot".to_string(),
                options: super::ProviderOptions {
                    base_url: Some("https://api.githubcopilot.com".to_string()),
                    api_key: Some("token".to_string()),
                },
                models: BTreeMap::from([(
                    "gpt-4.1".to_string(),
                    super::ModelConfigEntry {
                        name: "GPT-4.1".to_string(),
                        ..Default::default()
                    },
                )]),
            },
            Some("gpt-4.1".to_string()),
        )
        .expect("provider should upsert");

        assert!(config.provider.contains_key("copilot"));
        assert_eq!(
            config.distilllab.current_provider.as_deref(),
            Some("copilot")
        );
        assert_eq!(config.distilllab.current_model.as_deref(), Some("gpt-4.1"));
    }

    #[test]
    fn upsert_provider_entry_preserves_desktop_ui_preferences() {
        let temp_dir =
            std::env::temp_dir().join(format!("distilllab-config-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let config_path = temp_dir.join("config.json");

        let mut config = super::AppConfig::default();
        config.schema = Some("https://opencode.ai/config.json".to_string());
        config.distilllab.desktop_ui = Some(DesktopUiConfig {
            theme: "dark".to_string(),
            locale: "zh-CN".to_string(),
            show_debug_panel: false,
            last_opened_canvas_project_id: None,
        });
        save_app_config_to_path(&config, &config_path).expect("seed config should save");

        let updated = super::upsert_provider_entry(
            &config_path,
            "ice",
            super::ProviderConfigEntry {
                npm: Some("@ai-sdk/openai-compatible".to_string()),
                name: "Ice".to_string(),
                options: super::ProviderOptions {
                    base_url: Some("https://ice.v.ua/v1".to_string()),
                    api_key: Some("token".to_string()),
                },
                models: BTreeMap::from([(
                    "gpt-5.4".to_string(),
                    super::ModelConfigEntry {
                        name: "GPT-5.4".to_string(),
                        ..Default::default()
                    },
                )]),
            },
            Some("gpt-5.4".to_string()),
        )
        .expect("provider should upsert");

        assert_eq!(
            updated.distilllab.desktop_ui,
            Some(DesktopUiConfig {
                theme: "dark".to_string(),
                locale: "zh-CN".to_string(),
                show_debug_panel: false,
                last_opened_canvas_project_id: None,
            })
        );

        let persisted = load_app_config_from_path(&config_path).expect("config should reload");
        assert_eq!(
            persisted.distilllab.desktop_ui,
            updated.distilllab.desktop_ui
        );
    }

    #[test]
    fn saves_and_loads_last_opened_canvas_project_id_in_desktop_ui_config() {
        let temp_dir =
            std::env::temp_dir().join(format!("distilllab-config-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let config_path = temp_dir.join("config.json");

        let mut config = super::AppConfig::default();
        config.distilllab.desktop_ui = Some(DesktopUiConfig {
            theme: "dark".to_string(),
            locale: "en-US".to_string(),
            show_debug_panel: false,
            last_opened_canvas_project_id: Some("project-remembered".to_string()),
        });

        save_app_config_to_path(&config, &config_path).expect("config should save");

        let saved = fs::read_to_string(&config_path).expect("config file should exist");
        assert!(saved.contains("\"lastOpenedCanvasProjectId\": \"project-remembered\""));

        let reloaded = load_app_config_from_path(&config_path).expect("config should reload");
        assert_eq!(
            reloaded
                .distilllab
                .desktop_ui
                .as_ref()
                .and_then(|desktop_ui| desktop_ui.last_opened_canvas_project_id.as_deref()),
            Some("project-remembered")
        );
    }

    #[test]
    fn loads_missing_max_agent_concurrency_with_default_of_four() {
        let temp_dir =
            std::env::temp_dir().join(format!("distilllab-config-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let config_path = temp_dir.join("config.json");

        fs::write(
            &config_path,
            r#"{
                "distilllab": {
                    "currentProvider": "ice"
                }
            }"#,
        )
        .expect("config file should be written");

        let config = load_app_config_from_path(&config_path).expect("config should load");

        assert_eq!(config.distilllab.max_agent_concurrency, 4);
    }

    #[test]
    fn loads_max_agent_concurrency_below_min_clamped_to_one() {
        let temp_dir =
            std::env::temp_dir().join(format!("distilllab-config-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let config_path = temp_dir.join("config.json");

        fs::write(
            &config_path,
            r#"{
                "distilllab": {
                    "maxAgentConcurrency": 0
                }
            }"#,
        )
        .expect("config file should be written");

        let config = load_app_config_from_path(&config_path).expect("config should load");

        assert_eq!(config.distilllab.max_agent_concurrency, 1);
    }

    #[test]
    fn loads_max_agent_concurrency_above_max_clamped_to_sixteen() {
        let temp_dir =
            std::env::temp_dir().join(format!("distilllab-config-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let config_path = temp_dir.join("config.json");

        fs::write(
            &config_path,
            r#"{
                "distilllab": {
                    "maxAgentConcurrency": 99
                }
            }"#,
        )
        .expect("config file should be written");

        let config = load_app_config_from_path(&config_path).expect("config should load");

        assert_eq!(config.distilllab.max_agent_concurrency, 16);
    }

    #[test]
    fn loads_max_agent_concurrency_over_u8_range_clamped_to_sixteen() {
        let temp_dir =
            std::env::temp_dir().join(format!("distilllab-config-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let config_path = temp_dir.join("config.json");

        fs::write(
            &config_path,
            r#"{
                "distilllab": {
                    "maxAgentConcurrency": 256
                }
            }"#,
        )
        .expect("config file should be written");

        let config = load_app_config_from_path(&config_path).expect("config should load");

        assert_eq!(config.distilllab.max_agent_concurrency, 16);
    }

    #[test]
    fn loads_negative_max_agent_concurrency_clamped_to_one() {
        let temp_dir =
            std::env::temp_dir().join(format!("distilllab-config-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let config_path = temp_dir.join("config.json");

        fs::write(
            &config_path,
            r#"{
                "distilllab": {
                    "maxAgentConcurrency": -1
                }
            }"#,
        )
        .expect("config file should be written");

        let config = load_app_config_from_path(&config_path).expect("config should load");

        assert_eq!(config.distilllab.max_agent_concurrency, 1);
    }

    #[test]
    fn saves_and_loads_valid_max_agent_concurrency_without_changing_it() {
        let temp_dir =
            std::env::temp_dir().join(format!("distilllab-config-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let config_path = temp_dir.join("config.json");

        let mut config = super::AppConfig::default();
        config.distilllab.max_agent_concurrency = 7;

        save_app_config_to_path(&config, &config_path).expect("config should save");

        let saved = fs::read_to_string(&config_path).expect("config file should exist");
        assert!(saved.contains("\"maxAgentConcurrency\": 7"));

        let reloaded = load_app_config_from_path(&config_path).expect("config should reload");

        assert_eq!(reloaded.distilllab.max_agent_concurrency, 7);
    }

    #[test]
    fn deletes_provider_entry_and_reselects_remaining_provider() {
        let temp_dir =
            std::env::temp_dir().join(format!("distilllab-config-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let config_path = temp_dir.join("config.json");

        let mut config = super::AppConfig::default();
        config.schema = Some("https://opencode.ai/config.json".to_string());
        config.distilllab.current_provider = Some("ice".to_string());
        config.distilllab.current_model = Some("gpt-5.4".to_string());
        config.provider.insert(
            "ice".to_string(),
            super::ProviderConfigEntry {
                npm: Some("@ai-sdk/openai-compatible".to_string()),
                name: "Ice".to_string(),
                options: super::ProviderOptions::default(),
                models: BTreeMap::from([(
                    "gpt-5.4".to_string(),
                    super::ModelConfigEntry {
                        name: "GPT-5.4".to_string(),
                        ..Default::default()
                    },
                )]),
            },
        );
        config.provider.insert(
            "openai".to_string(),
            super::ProviderConfigEntry {
                npm: Some("@ai-sdk/openai-compatible".to_string()),
                name: "OpenAI".to_string(),
                options: super::ProviderOptions::default(),
                models: BTreeMap::from([(
                    "gpt-5".to_string(),
                    super::ModelConfigEntry {
                        name: "GPT-5".to_string(),
                        ..Default::default()
                    },
                )]),
            },
        );
        super::save_app_config_to_path(&config, &config_path).expect("seed config should save");

        let updated =
            super::delete_provider_entry(&config_path, "ice").expect("provider should delete");

        assert!(!updated.provider.contains_key("ice"));
        assert_eq!(
            updated.distilllab.current_provider.as_deref(),
            Some("openai")
        );
        assert_eq!(updated.distilllab.current_model.as_deref(), Some("gpt-5"));
    }

    #[test]
    fn updates_current_provider_and_model_selection() {
        let temp_dir =
            std::env::temp_dir().join(format!("distilllab-config-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let config_path = temp_dir.join("config.json");

        let mut config = super::AppConfig::default();
        config.schema = Some("https://opencode.ai/config.json".to_string());
        config.provider.insert(
            "ice".to_string(),
            super::ProviderConfigEntry {
                npm: Some("@ai-sdk/openai-compatible".to_string()),
                name: "Ice".to_string(),
                options: super::ProviderOptions::default(),
                models: BTreeMap::from([(
                    "gpt-5.4".to_string(),
                    super::ModelConfigEntry {
                        name: "GPT-5.4".to_string(),
                        ..Default::default()
                    },
                )]),
            },
        );
        config.provider.insert(
            "copilot".to_string(),
            super::ProviderConfigEntry {
                npm: Some("@ai-sdk/openai-compatible".to_string()),
                name: "GitHub Copilot".to_string(),
                options: super::ProviderOptions::default(),
                models: BTreeMap::from([(
                    "gpt-4.1".to_string(),
                    super::ModelConfigEntry {
                        name: "GPT-4.1".to_string(),
                        ..Default::default()
                    },
                )]),
            },
        );
        super::save_app_config_to_path(&config, &config_path).expect("seed config should save");

        let updated = super::set_current_provider_model(&config_path, "copilot", "gpt-4.1")
            .expect("current selection should update");

        assert_eq!(
            updated.distilllab.current_provider.as_deref(),
            Some("copilot")
        );
        assert_eq!(updated.distilllab.current_model.as_deref(), Some("gpt-4.1"));
    }
}
