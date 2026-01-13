use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub rules: RulesConfig,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct RulesConfig {
    #[serde(rename = "multiline-spacing", default)]
    pub multiline_spacing: MultilineSpacingConfig,
    #[serde(rename = "import-hoisting", default)]
    pub import_hoisting: ImportHoistingConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MultilineSpacingConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_min_lines")]
    pub min_lines: usize,
}

fn default_enabled() -> bool {
    true
}

fn default_min_lines() -> usize {
    3
}

impl Default for MultilineSpacingConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            min_lines: default_min_lines(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ImportHoistingConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

impl Default for ImportHoistingConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
        }
    }
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;

        toml::from_str(&content)
            .context("Failed to parse TOML configuration")
    }

    pub fn default() -> Self {
        Self {
            rules: RulesConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.rules.multiline_spacing.enabled);
        assert_eq!(config.rules.multiline_spacing.min_lines, 3);
    }

    #[test]
    fn test_parse_config() {
        let toml = r#"
[rules.multiline-spacing]
enabled = true
min_lines = 5
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.rules.multiline_spacing.enabled);
        assert_eq!(config.rules.multiline_spacing.min_lines, 5);
    }
}
