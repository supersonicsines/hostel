use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::service::ServiceMetadata;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub keybind_mode: KeybindMode,
    #[serde(default)]
    pub hidden_keywords: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            keybind_mode: KeybindMode::Regular,
            hidden_keywords: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum KeybindMode {
    #[default]
    Regular,
    Vim,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppData {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, ServiceMetadata>,
    #[serde(default)]
    pub memos: HashMap<String, String>,
    #[serde(default)]
    pub url_overrides: HashMap<String, String>,
}

fn config_dir() -> PathBuf {
    if let Ok(path) = std::env::var("HOSTEL_CONFIG_DIR") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }

    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hostel")
}

fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

fn data_path() -> PathBuf {
    config_dir().join("data.json")
}

pub fn config_exists() -> bool {
    config_path().exists()
}

pub fn load_config() -> Result<Config> {
    let path = config_path();
    if !path.exists() {
        return Ok(Config::default());
    }

    let contents =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&contents).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn save_config(config: &Config) -> Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    let contents = serde_json::to_string_pretty(config).context("failed to serialize config")?;
    fs::write(config_path(), contents).context("failed to write config")
}

pub fn load_data() -> Result<AppData> {
    let path = data_path();
    if !path.exists() {
        return Ok(AppData::default());
    }

    let contents =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&contents).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn save_data(data: &AppData) -> Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    let contents = serde_json::to_string_pretty(data).context("failed to serialize data")?;
    fs::write(data_path(), contents).context("failed to write data")
}
