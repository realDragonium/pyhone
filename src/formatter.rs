use crate::config::Config;
use crate::parser::parse_python;
use crate::rules::{
    import_hoisting::ImportHoistingRule, multiline_spacing::MultilineSpacingRule, RuleRegistry,
    Violation,
};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub struct Formatter {
    registry: RuleRegistry,
}

impl Formatter {
    pub fn new(config: Config) -> Self {
        let mut registry = RuleRegistry::new();

        if config.rules.import_hoisting.enabled {
            registry.register(Box::new(ImportHoistingRule::new()));
        }

        if config.rules.multiline_spacing.enabled {
            let rule = MultilineSpacingRule::new(config.rules.multiline_spacing.min_lines);
            registry.register(Box::new(rule));
        }

        Self { registry }
    }

    pub fn check_file<P: AsRef<Path>>(&self, path: P) -> Result<Vec<Violation>> {
        let source = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read file: {:?}", path.as_ref()))?;

        self.check_source(&source)
    }

    pub fn check_source(&self, source: &str) -> Result<Vec<Violation>> {
        let ast = parse_python(source)?;

        let mut all_violations = Vec::new();

        for rule in self.registry.rules() {
            let violations = rule.apply(source, &ast)?;
            all_violations.extend(violations);
        }

        all_violations.sort_by_key(|v| (v.line, v.column));

        Ok(all_violations)
    }

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

    fn apply_fixes(&self, source: &str, violations: &[Violation]) -> Result<String> {
        let mut lines: Vec<&str> = source.lines().collect();
        let mut insertions: Vec<(usize, bool)> = Vec::new();

        for violation in violations {
            if violation.message.contains("should have a blank line before it") {
                insertions.push((violation.line - 1, true));
            } else if violation.message.contains("should have a blank line after it") {
                insertions.push((violation.line - 1, false));
            }
        }

        insertions.sort_by(|a, b| b.0.cmp(&a.0));
        insertions.dedup();

        for (line_index, is_before) in insertions {
            if is_before {
                if line_index < lines.len() {
                    lines.insert(line_index, "");
                }
            } else if line_index < lines.len() {
                lines.insert(line_index + 1, "");
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

        assert!(fixed.contains("\n\ndef foo():"));
        assert!(fixed.contains("c = 3\n\ny = 2"));
    }
}
