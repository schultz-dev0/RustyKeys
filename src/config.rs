use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum KeyClass {
    Normal,
    Space,
    Enter,
    Backspace,
    Modifier,
}

impl KeyClass {
    pub fn from_wire(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "space" => Self::Space,
            "enter" => Self::Enter,
            "backspace" => Self::Backspace,
            "modifier" => Self::Modifier,
            _ => Self::Normal,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub enabled: bool,
    pub volume: f32,
    pub matugen_css_path: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 0.45,
            matugen_css_path: None,
        }
    }
}

pub fn config_dir() -> PathBuf {
    if let Some(dirs) = ProjectDirs::from("dev", "cloudyy", "rustykeys") {
        return dirs.config_dir().to_path_buf();
    }

    PathBuf::from("./.rustykeys-config")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn override_sounds_dir() -> PathBuf {
    config_dir().join("sounds")
}

pub fn load() -> AppConfig {
    let path = config_path();
    let Ok(raw) = fs::read_to_string(path) else {
        return AppConfig::default();
    };

    toml::from_str(&raw).unwrap_or_default()
}

pub fn save(cfg: &AppConfig) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("Failed to create config directory")?;
    }

    let serialized = toml::to_string_pretty(cfg).context("Failed to serialize config")?;

    let parent = path
        .parent()
        .context("Config parent path missing")?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .context("Failed to create temp config file")?;

    temp.write_all(serialized.as_bytes())
        .context("Failed to write to temp config")?;
    temp.flush().context("Failed to flush temp config")?;

    temp.persist(&path).context("Failed to save config file")?;

    Ok(())
}
