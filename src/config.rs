use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub keybind_mode: KeybindMode,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            keybind_mode: KeybindMode::Regular,
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
    #[serde(default)]
    pub memos: HashMap<String, String>,
}

fn config_dir() -> PathBuf {
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
