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
    #[serde(rename = "max-lines", default)]
    pub max_lines: MaxLinesConfig,
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
    2
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

/// Opt-in: line limits are project specific, so this rule stays off until enabled.
#[derive(Debug, Deserialize, Serialize)]
pub struct MaxLinesConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_max_file_lines")]
    pub max_file_lines: usize,
    #[serde(default = "default_max_class_lines")]
    pub max_class_lines: usize,
}

fn default_max_file_lines() -> usize {
    crate::rules::max_lines::DEFAULT_MAX_FILE_LINES
}

fn default_max_class_lines() -> usize {
    crate::rules::max_lines::DEFAULT_MAX_CLASS_LINES
}

impl Default for MaxLinesConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_file_lines: default_max_file_lines(),
            max_class_lines: default_max_class_lines(),
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
        assert_eq!(config.rules.multiline_spacing.min_lines, 2);
    }

    #[test]
    fn test_max_lines_defaults_to_disabled() {
        let config = Config::default();
        assert!(!config.rules.max_lines.enabled);
        assert_eq!(config.rules.max_lines.max_file_lines, 500);
        assert_eq!(config.rules.max_lines.max_class_lines, 200);
    }

    #[test]
    fn test_parse_max_lines_config() {
        let toml = r#"
[rules.max-lines]
enabled = true
max_file_lines = 300
max_class_lines = 120
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.rules.max_lines.enabled);
        assert_eq!(config.rules.max_lines.max_file_lines, 300);
        assert_eq!(config.rules.max_lines.max_class_lines, 120);
    }

    #[test]
    fn test_max_lines_partial_config_keeps_defaults() {
        let toml = r#"
[rules.max-lines]
enabled = true
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.rules.max_lines.enabled);
        assert_eq!(config.rules.max_lines.max_file_lines, 500);
        assert_eq!(config.rules.max_lines.max_class_lines, 200);
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
