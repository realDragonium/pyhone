use crate::config::Config;
use crate::parser::parse_python;
use crate::rules::{
    import_hoisting::ImportHoistingRule, max_lines::MaxLinesRule,
    multiline_spacing::MultilineSpacingRule, RuleRegistry, Violation,
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

        if config.rules.max_lines.enabled {
            let rule = MaxLinesRule::new(
                config.rules.max_lines.max_file_lines,
                config.rules.max_lines.max_class_lines,
            );
            registry.register(Box::new(rule));
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
            // Rules like max-lines only report, so guard the write to avoid
            // rewriting files that have no actual fix applied to them.
            let fixed_source = self.apply_fixes(&source, &violations)?;
            if fixed_source != source {
                fs::write(path.as_ref(), fixed_source)
                    .with_context(|| format!("Failed to write file: {:?}", path.as_ref()))?;
            }
        }

        Ok(violations)
    }

    fn apply_fixes(&self, source: &str, violations: &[Violation]) -> Result<String> {
        use crate::rules::FixKind;

        enum Op {
            Insert(usize, bool), // (line_index 0-based, is_before)
            Remove(usize),       // line_index 0-based
        }

        let mut ops: Vec<Op> = Vec::new();

        for violation in violations {
            match violation.fix_kind {
                FixKind::InsertBlankBefore => ops.push(Op::Insert(violation.line - 1, true)),
                FixKind::InsertBlankAfter => ops.push(Op::Insert(violation.line - 1, false)),
                FixKind::RemoveBlankBefore if violation.line >= 2 => {
                    ops.push(Op::Remove(violation.line - 2));
                }
                _ => {}
            }
        }

        // Process from bottom to top so earlier line indices stay valid
        ops.sort_by(|a, b| {
            let la = match a { Op::Insert(l, _) | Op::Remove(l) => *l };
            let lb = match b { Op::Insert(l, _) | Op::Remove(l) => *l };
            lb.cmp(&la)
        });

        let mut lines: Vec<&str> = source.lines().collect();

        for op in ops {
            match op {
                Op::Insert(idx, is_before) => {
                    if is_before {
                        if idx < lines.len() {
                            lines.insert(idx, "");
                        }
                    } else if idx < lines.len() {
                        lines.insert(idx + 1, "");
                    }
                }
                Op::Remove(idx) => {
                    if idx < lines.len() && lines[idx].trim().is_empty() {
                        lines.remove(idx);
                    }
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

        assert!(fixed.contains("\n\ndef foo():"));
        assert!(fixed.contains("c = 3\n\ny = 2"));
    }
}
