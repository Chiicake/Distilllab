use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

type ConfigError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(rename = "$schema")]
    pub schema: Option<String>,
    #[serde(default)]
    pub provider: BTreeMap<String, ProviderConfigEntry>,
    #[serde(default)]
    pub distilllab: DistilllabConfigSection,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DistilllabConfigSection {
    #[serde(rename = "currentProvider")]
    pub current_provider: Option<String>,
    #[serde(rename = "currentModel")]
    pub current_model: Option<String>,
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

pub fn load_app_config_from_path(path: &Path) -> Result<AppConfig, ConfigError> {
    let content = fs::read_to_string(path)?;
    let config = serde_json::from_str::<AppConfig>(&content)?;
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

    let content = serde_json::to_string_pretty(config)?;
    fs::write(path, content)?;
    Ok(())
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
        load_app_config_from_path, resolve_current_model_selection, resolve_current_provider_model,
        save_app_config_to_path,
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
}
