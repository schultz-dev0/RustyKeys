//! Config and path helpers
//! 
//! This module owns:
//! -- App settings
//! -- User directories for custom sound kits
//! -- Shared key class parsing

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

/// Return the app config directory. Preferred path: ~/.config/rustykeys
pub fn config_dir() -> PathBuf {
    if let Some(dirs) = ProjectDirs::from("dev", "cloudyy", "rustykeys") {
        return dirs.config_dir().to_path_buf();
    }

    PathBuf::from("./.rustykeys-config")
}

/// Path to the main app config file.
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

pub fn save(cfg: &AppConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create config dir failed: {e}"))?;
    }

    let serialized = toml::to_string_pretty(cfg)
        .map_err(|e| format!("serialize config failed: {e}"))?;

    let parent = path
        .parent()
        .ok_or_else(|| String::from("config parent path missing"))?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| format!("create temp config failed: {e}"))?;

    temp.write_all(serialized.as_bytes())
        .map_err(|e| format!("write temp config failed: {e}"))?;
    temp.flush()
        .map_err(|e| format!("flush temp config failed: {e}"))?;

    temp.persist(&path)
        .map_err(|e| format!("persist config failed: {e}"))?;

    Ok(())
}
