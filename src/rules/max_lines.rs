use crate::rules::{FixKind, FormattingRule, Violation};
use anyhow::Result;
use ruff_python_ast::{ModModule, Stmt, StmtClassDef};
use ruff_text_size::Ranged;

pub const DEFAULT_MAX_FILE_LINES: usize = 500;
pub const DEFAULT_MAX_CLASS_LINES: usize = 200;

/// Reports files and classes that grow past a configured line count.
/// Both limits are inclusive: a limit of 500 flags a file at 501 lines.
#[derive(Debug)]
pub struct MaxLinesRule {
    pub max_file_lines: usize,
    pub max_class_lines: usize,
}

impl MaxLinesRule {
    pub fn new(max_file_lines: usize, max_class_lines: usize) -> Self {
        Self {
            max_file_lines,
            max_class_lines,
        }
    }

    fn line_number(&self, source: &str, offset: usize) -> usize {
        source[..offset].matches('\n').count() + 1
    }

    fn column_number(&self, source: &str, offset: usize) -> usize {
        let line_start = source[..offset].rfind('\n').map_or(0, |i| i + 1);
        offset - line_start + 1
    }

    /// Ruff includes decorators in a class's range; the length of a class is measured
    /// from its `class` header so decorators don't count against the limit.
    fn class_line_span(&self, source: &str, class: &StmtClassDef) -> (usize, usize) {
        let start = class.name.range().start().to_usize();
        let end = class.range().end().to_usize();

        (self.line_number(source, start), self.line_number(source, end))
    }

    fn check_statements(&self, source: &str, statements: &[Stmt], violations: &mut Vec<Violation>) {
        for stmt in statements {
            match stmt {
                Stmt::ClassDef(class) => {
                    self.check_class(source, class, violations);
                    self.check_statements(source, &class.body, violations);
                }
                Stmt::FunctionDef(func) => {
                    self.check_statements(source, &func.body, violations);
                }
                Stmt::If(if_stmt) => {
                    self.check_statements(source, &if_stmt.body, violations);
                    for clause in &if_stmt.elif_else_clauses {
                        self.check_statements(source, &clause.body, violations);
                    }
                }
                Stmt::For(for_stmt) => {
                    self.check_statements(source, &for_stmt.body, violations);
                    self.check_statements(source, &for_stmt.orelse, violations);
                }
                Stmt::While(while_stmt) => {
                    self.check_statements(source, &while_stmt.body, violations);
                    self.check_statements(source, &while_stmt.orelse, violations);
                }
                Stmt::With(with_stmt) => {
                    self.check_statements(source, &with_stmt.body, violations);
                }
                Stmt::Try(try_stmt) => {
                    self.check_statements(source, &try_stmt.body, violations);
                    for handler in &try_stmt.handlers {
                        let ruff_python_ast::ExceptHandler::ExceptHandler(h) = handler;
                        self.check_statements(source, &h.body, violations);
                    }
                    self.check_statements(source, &try_stmt.orelse, violations);
                    self.check_statements(source, &try_stmt.finalbody, violations);
                }
                _ => {}
            }
        }
    }

    fn check_class(&self, source: &str, class: &StmtClassDef, violations: &mut Vec<Violation>) {
        let (start_line, end_line) = self.class_line_span(source, class);
        let line_count = end_line - start_line + 1;

        if line_count <= self.max_class_lines {
            return;
        }

        let offset = class.name.range().start().to_usize();

        violations.push(Violation {
            line: start_line,
            column: self.column_number(source, offset),
            message: format!(
                "Class '{}' has {} lines, exceeds the maximum of {}",
                class.name, line_count, self.max_class_lines
            ),
            rule_name: self.name().to_string(),
            fix_kind: FixKind::None,
        });
    }

    fn check_file(&self, source: &str, violations: &mut Vec<Violation>) {
        let line_count = source.lines().count();

        if line_count <= self.max_file_lines {
            return;
        }

        violations.push(Violation {
            line: 1,
            column: 1,
            message: format!(
                "File has {} lines, exceeds the maximum of {}",
                line_count, self.max_file_lines
            ),
            rule_name: self.name().to_string(),
            fix_kind: FixKind::None,
        });
    }
}

impl Default for MaxLinesRule {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_FILE_LINES, DEFAULT_MAX_CLASS_LINES)
    }
}

impl FormattingRule for MaxLinesRule {
    fn name(&self) -> &str {
        "max-lines"
    }

    fn apply(&self, source: &str, ast: &ModModule) -> Result<Vec<Violation>> {
        let mut violations = Vec::new();

        self.check_file(source, &mut violations);
        self.check_statements(source, &ast.body, &mut violations);

        Ok(violations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_python;

    fn violations_for(source: &str, max_file_lines: usize, max_class_lines: usize) -> Vec<Violation> {
        let ast = parse_python(source).unwrap();
        let rule = MaxLinesRule::new(max_file_lines, max_class_lines);

        rule.apply(source, &ast).unwrap()
    }

    #[test]
    fn test_file_over_limit() {
        let source = include_str!("../../tests/fixtures/max_lines/long_file.py");
        let violations = violations_for(source, 5, 100);

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, 1);
        assert!(violations[0].message.contains("File has 8 lines"));
        assert!(violations[0].message.contains("maximum of 5"));
    }

    #[test]
    fn test_file_at_limit_is_allowed() {
        let source = include_str!("../../tests/fixtures/max_lines/long_file.py");
        let violations = violations_for(source, 8, 100);

        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_class_over_limit() {
        let source = include_str!("../../tests/fixtures/max_lines/long_class.py");
        let violations = violations_for(source, 1000, 5);

        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("Class 'Big'"));
        assert!(violations[0].message.contains("has 7 lines"));
    }

    #[test]
    fn test_class_at_limit_is_allowed() {
        let source = include_str!("../../tests/fixtures/max_lines/long_class.py");
        let violations = violations_for(source, 1000, 7);

        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_decorators_do_not_count_towards_class_length() {
        let source = include_str!("../../tests/fixtures/max_lines/decorated_class.py");
        let violations = violations_for(source, 1000, 4);

        assert_eq!(violations.len(), 0, "decorator lines should not be counted");
    }

    #[test]
    fn test_nested_class_reported_separately() {
        let source = include_str!("../../tests/fixtures/max_lines/nested_class.py");
        let violations = violations_for(source, 1000, 4);

        assert_eq!(violations.len(), 2);
        assert!(violations[0].message.contains("Class 'Outer'"));
        assert!(violations[1].message.contains("Class 'Inner'"));
        assert!(violations[1].column > 1, "nested class should report its indent");
    }

    #[test]
    fn test_class_inside_function_is_checked() {
        let source = include_str!("../../tests/fixtures/max_lines/class_in_function.py");
        let violations = violations_for(source, 1000, 3);

        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("Class 'Local'"));
    }

    #[test]
    fn test_violations_are_unfixable() {
        let source = include_str!("../../tests/fixtures/max_lines/long_file.py");
        let violations = violations_for(source, 1, 1);

        assert!(violations.iter().all(|v| v.fix_kind == FixKind::None));
    }
}
