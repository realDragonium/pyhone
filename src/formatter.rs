use crate::config::Config;
use crate::parser::parse_python;
use crate::rules::{multiline_spacing::MultilineSpacingRule, FormattingRule, RuleRegistry, Violation};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Mode for running the formatter
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FormatMode {
    /// Check for violations without fixing
    Check,
    /// Apply fixes to files
    Format,
}

/// Main formatter that orchestrates rules
pub struct Formatter {
    config: Config,
    registry: RuleRegistry,
}

impl Formatter {
    pub fn new(config: Config) -> Self {
        let mut registry = RuleRegistry::new();

        // Register enabled rules
        if config.rules.multiline_spacing.enabled {
            let rule = MultilineSpacingRule::new(config.rules.multiline_spacing.min_lines);
            registry.register(Box::new(rule));
        }

        Self { config, registry }
    }

    /// Check a Python file for violations
    pub fn check_file<P: AsRef<Path>>(&self, path: P) -> Result<Vec<Violation>> {
        let source = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read file: {:?}", path.as_ref()))?;

        self.check_source(&source)
    }

    /// Check Python source code for violations
    pub fn check_source(&self, source: &str) -> Result<Vec<Violation>> {
        let ast = parse_python(source)?;

        let mut all_violations = Vec::new();

        for rule in self.registry.rules() {
            let violations = rule.apply(source, &ast)?;
            all_violations.extend(violations);
        }

        // Sort violations by line number
        all_violations.sort_by_key(|v| (v.line, v.column));

        Ok(all_violations)
    }

    /// Format a Python file by applying fixes
    pub fn format_file<P: AsRef<Path>>(&self, path: P) -> Result<Vec<Violation>> {
        let source = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read file: {:?}", path.as_ref()))?;

        let violations = self.check_source(&source)?;

        if !violations.is_empty() {
            let fixed_source = self.apply_fixes(&source, &violations)?;
            fs::write(path.as_ref(), fixed_source)
                .with_context(|| format!("Failed to write file: {:?}", path.as_ref()))?;
        }

        Ok(violations)
    }

    /// Apply fixes to source code based on violations
    fn apply_fixes(&self, source: &str, violations: &[Violation]) -> Result<String> {
        let mut lines: Vec<&str> = source.lines().collect();
        let mut insertions: Vec<(usize, bool)> = Vec::new(); // (line_index, is_before)

        // Collect all the insertions we need to make
        // Note: violation.line is 1-indexed, but lines vector is 0-indexed
        for violation in violations {
            if violation.message.contains("should have a blank line before it") {
                // Insert before line N: insert at index N-1
                insertions.push((violation.line - 1, true));
            } else if violation.message.contains("should have a blank line after it") {
                // Insert after line N: insert at index N (which is after the line at index N-1)
                insertions.push((violation.line - 1, false));
            }
        }

        // Sort in reverse order so we can insert from bottom to top
        insertions.sort_by(|a, b| b.0.cmp(&a.0));
        insertions.dedup();

        // Apply insertions
        for (line_index, is_before) in insertions {
            if is_before {
                // Insert before the line
                if line_index < lines.len() {
                    lines.insert(line_index, "");
                }
            } else {
                // Insert after the line
                if line_index < lines.len() {
                    lines.insert(line_index + 1, "");
                }
            }
        }

        Ok(lines.join("\n") + "\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_source_with_violations() {
        let config = Config::default();
        let formatter = Formatter::new(config);

        let source = r#"x = 1
def foo():
    a = 1
    b = 2
    c = 3
y = 2
"#;

        let violations = formatter.check_source(source).unwrap();
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_check_source_without_violations() {
        let config = Config::default();
        let formatter = Formatter::new(config);

        let source = r#"x = 1

def foo():
    a = 1
    b = 2
    c = 3

y = 2
"#;

        let violations = formatter.check_source(source).unwrap();
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_apply_fixes() {
        let config = Config::default();
        let formatter = Formatter::new(config);

        let source = r#"x = 1
def foo():
    a = 1
    b = 2
    c = 3
y = 2
"#;

        let violations = formatter.check_source(source).unwrap();
        let fixed = formatter.apply_fixes(source, &violations).unwrap();

        // Should have blank lines added
        assert!(fixed.contains("\n\ndef foo():"));
        assert!(fixed.contains("c = 3\n\ny = 2"));
    }
}
