use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default)]
pub struct ProjectConfig {
    #[serde(default)]
    pub providers: Vec<ProviderEntry>,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ProviderEntry {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(default = "default_true", skip_serializing_if = "std::ops::Not::not")]
    pub enabled: bool,
    #[serde(default)]
    pub config: HashMap<String, serde_yaml::Value>,
}

impl ProviderEntry {
    pub fn display_name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.name)
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.config.get(key).and_then(|v| v.as_str())
    }

    pub fn get_list(&self, key: &str) -> Vec<String> {
        self.config
            .get(key)
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn config_path(root: &Path) -> PathBuf {
    root.join(".banco/config.yml")
}

pub fn load(root: &Path) -> anyhow::Result<ProjectConfig> {
    let path = config_path(root);
    if !path.exists() {
        return Ok(ProjectConfig::default());
    }
    let content = std::fs::read_to_string(&path)?;
    let config: ProjectConfig = serde_yaml::from_str(&content)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(config)
}

pub fn save(root: &Path, config: &ProjectConfig) -> anyhow::Result<()> {
    let path = config_path(root);
    std::fs::create_dir_all(path.parent().unwrap())?;
    let content = serde_yaml::to_string(config)?;
    std::fs::write(&path, content)?;
    Ok(())
}
