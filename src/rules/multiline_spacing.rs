use crate::rules::{FormattingRule, Violation};
use anyhow::Result;
use rustpython_parser::ast::{Mod, Ranged, Stmt};

#[derive(Debug)]
pub struct MultilineSpacingRule {
    pub min_lines: usize,
}

impl MultilineSpacingRule {
    pub fn new(min_lines: usize) -> Self {
        Self { min_lines }
    }

    fn is_multiline(&self, source: &str, stmt: &Stmt) -> bool {
        let range = stmt.range();
        let start = range.start().to_usize();
        let end = range.end().to_usize();

        let stmt_source = &source[start..end];
        let line_count = stmt_source.lines().count();

        line_count >= self.min_lines
    }

    fn get_line_number(&self, source: &str, offset: usize) -> usize {
        if offset == 0 {
            return 1;
        }
        source[..offset].matches('\n').count() + 1
    }

    fn has_blank_line_before(&self, source: &str, line_num: usize) -> bool {
        if line_num <= 1 {
            return true;
        }

        let lines: Vec<&str> = source.lines().collect();
        if line_num > lines.len() {
            return true;
        }

        if line_num >= 2 {
            let prev_line = lines[line_num - 2];
            prev_line.trim().is_empty()
        } else {
            true
        }
    }

    fn has_blank_line_after(&self, source: &str, line_num: usize) -> bool {
        let lines: Vec<&str> = source.lines().collect();
        if line_num >= lines.len() {
            return true;
        }

        if line_num < lines.len() {
            let next_line = lines[line_num];
            next_line.trim().is_empty()
        } else {
            true
        }
    }
}

impl Default for MultilineSpacingRule {
    fn default() -> Self {
        Self::new(3)
    }
}

impl FormattingRule for MultilineSpacingRule {
    fn name(&self) -> &str {
        "multiline-spacing"
    }

    fn description(&self) -> &str {
        "Ensures multi-line statements have blank lines before and after them"
    }

    fn apply(&self, source: &str, ast: &Mod) -> Result<Vec<Violation>> {
        let mut violations = Vec::new();

        let statements = match ast {
            Mod::Module(module) => &module.body,
            Mod::Interactive(interactive) => &interactive.body,
            Mod::Expression(_) => return Ok(violations),
            Mod::FunctionType(_) => return Ok(violations),
        };

        for (i, stmt) in statements.iter().enumerate() {
            if !self.is_multiline(source, stmt) {
                continue;
            }

            let range = stmt.range();
            let start = range.start().to_usize();
            let end = range.end().to_usize();

            let start_line = self.get_line_number(source, start);
            let end_line = self.get_line_number(source, end);
            let line_count = source[start..end].lines().count();

            let is_first = i == 0;
            let prev_is_multiline = if i > 0 {
                self.is_multiline(source, &statements[i - 1])
            } else {
                false
            };

            if !is_first && !prev_is_multiline && !self.has_blank_line_before(source, start_line) {
                violations.push(Violation {
                    line: start_line,
                    column: 1,
                    message: format!(
                        "Multi-line statement ({} lines) should have a blank line before it",
                        line_count
                    ),
                    rule_name: self.name().to_string(),
                });
            }

            let is_last = i == statements.len() - 1;
            let next_is_multiline = if i < statements.len() - 1 {
                self.is_multiline(source, &statements[i + 1])
            } else {
                false
            };

            if !is_last && !next_is_multiline && !self.has_blank_line_after(source, end_line) {
                violations.push(Violation {
                    line: end_line,
                    column: 1,
                    message: format!(
                        "Multi-line statement ({} lines) should have a blank line after it",
                        line_count
                    ),
                    rule_name: self.name().to_string(),
                });
            }
        }

        Ok(violations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_python;

    #[test]
    fn test_multiline_without_spacing() {
        let source = r#"x = 1
def foo():
    a = 1
    b = 2
    c = 3
y = 2
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(3);
        let violations = rule.apply(source, &ast).unwrap();

        assert!(!violations.is_empty());
    }

    #[test]
    fn test_multiline_with_spacing() {
        let source = r#"x = 1

def foo():
    a = 1
    b = 2
    c = 3

y = 2
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(3);
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_single_line_no_spacing_needed() {
        let source = r#"x = 1
y = 2
z = 3
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(3);
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 0);
    }
}
