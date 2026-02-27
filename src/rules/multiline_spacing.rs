use crate::rules::{FormattingRule, Violation};
use anyhow::Result;
use rustpython_parser::ast::{ExceptHandler, Mod, Ranged, Stmt};

#[derive(Debug)]
pub struct MultilineSpacingRule {
    pub min_lines: usize,
}

impl MultilineSpacingRule {
    pub fn new(min_lines: usize) -> Self {
        Self { min_lines }
    }

    fn is_multiline(&self, source: &str, stmt: &Stmt) -> bool {
        // Imports are handled by ruff — ignore them regardless of line count
        if matches!(stmt, Stmt::Import(_) | Stmt::ImportFrom(_)) {
            return false;
        }

        let range = stmt.range();
        let start = range.start().to_usize();
        let end = range.end().to_usize();

        let stmt_source = &source[start..end];
        let line_count = stmt_source.lines().count();

        line_count >= self.min_lines
    }

    fn is_compound_statement(&self, stmt: &Stmt) -> bool {
        matches!(
            stmt,
            Stmt::If(_)
                | Stmt::For(_)
                | Stmt::AsyncFor(_)
                | Stmt::While(_)
                | Stmt::With(_)
                | Stmt::AsyncWith(_)
                | Stmt::Try(_)
                | Stmt::FunctionDef(_)
                | Stmt::AsyncFunctionDef(_)
                | Stmt::ClassDef(_)
                | Stmt::Match(_)
        )
    }

    fn has_multiline_header(&self, source: &str, stmt: &Stmt) -> bool {
        if !self.is_compound_statement(stmt) {
            return false;
        }

        let range = stmt.range();
        let start = range.start().to_usize();
        let end = range.end().to_usize();
        let stmt_source = &source[start..end];

        // Find the first colon followed by newline (end of header)
        // We need to be careful about colons inside strings/parentheses
        let mut paren_depth: i32 = 0;
        let mut bracket_depth: i32 = 0;
        let mut in_string = false;
        let mut string_char = ' ';
        let mut prev_char = ' ';
        let mut colon_pos = None;

        for (i, ch) in stmt_source.char_indices() {
            if in_string {
                if ch == string_char && prev_char != '\\' {
                    in_string = false;
                }
            } else {
                match ch {
                    '"' | '\'' => {
                        in_string = true;
                        string_char = ch;
                    }
                    '(' => paren_depth += 1,
                    ')' => paren_depth = paren_depth.saturating_sub(1),
                    '[' => bracket_depth += 1,
                    ']' => bracket_depth = bracket_depth.saturating_sub(1),
                    ':' if paren_depth == 0 && bracket_depth == 0 => {
                        colon_pos = Some(i);
                        break;
                    }
                    _ => {}
                }
            }
            prev_char = ch;
        }

        if let Some(pos) = colon_pos {
            let header = &stmt_source[..=pos];
            let header_lines = header.lines().count();
            header_lines >= self.min_lines
        } else {
            false
        }
    }

    fn get_line_number(&self, source: &str, offset: usize) -> usize {
        if offset == 0 {
            return 1;
        }
        source[..offset].matches('\n').count() + 1
    }

    /// For decorated statements, the AST range starts at `def`/`class`, not the decorator.
    /// This returns the line of the first decorator if one exists, so blank lines are
    /// inserted before the decorator rather than between it and the definition.
    fn get_effective_start_line(&self, source: &str, stmt: &Stmt) -> usize {
        let decorator_offset = match stmt {
            Stmt::FunctionDef(s) => s.decorator_list.first().map(|d| d.range().start().to_usize()),
            Stmt::AsyncFunctionDef(s) => s.decorator_list.first().map(|d| d.range().start().to_usize()),
            Stmt::ClassDef(s) => s.decorator_list.first().map(|d| d.range().start().to_usize()),
            _ => None,
        };

        if let Some(offset) = decorator_offset {
            self.get_line_number(source, offset)
        } else {
            self.get_line_number(source, stmt.range().start().to_usize())
        }
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
        Self::new(2)
    }
}

impl MultilineSpacingRule {
    fn check_statement_list(&self, source: &str, statements: &[Stmt], violations: &mut Vec<Violation>) {
        for (i, stmt) in statements.iter().enumerate() {
            // Recurse into nested bodies regardless of whether this statement is multiline
            self.recurse_into_stmt(source, stmt, violations);

            if !self.is_multiline(source, stmt) {
                continue;
            }

            let range = stmt.range();
            let start = range.start().to_usize();
            let end = range.end().to_usize();

            let start_line = self.get_effective_start_line(source, stmt);
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

            // Skip blank-after check for compound statements with multiline headers
            // (the body follows at a different indentation level)
            let has_multiline_header = self.has_multiline_header(source, stmt);

            if !is_last && !has_multiline_header && !self.has_blank_line_after(source, end_line) {
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
    }

    fn recurse_into_stmt(&self, source: &str, stmt: &Stmt, violations: &mut Vec<Violation>) {
        match stmt {
            Stmt::FunctionDef(s) => self.check_statement_list(source, &s.body, violations),
            Stmt::AsyncFunctionDef(s) => self.check_statement_list(source, &s.body, violations),
            Stmt::ClassDef(s) => self.check_statement_list(source, &s.body, violations),
            Stmt::If(s) => {
                self.check_statement_list(source, &s.body, violations);
                self.check_statement_list(source, &s.orelse, violations);
            }
            Stmt::For(s) => {
                self.check_statement_list(source, &s.body, violations);
                self.check_statement_list(source, &s.orelse, violations);
            }
            Stmt::AsyncFor(s) => {
                self.check_statement_list(source, &s.body, violations);
                self.check_statement_list(source, &s.orelse, violations);
            }
            Stmt::While(s) => {
                self.check_statement_list(source, &s.body, violations);
                self.check_statement_list(source, &s.orelse, violations);
            }
            Stmt::With(s) => self.check_statement_list(source, &s.body, violations),
            Stmt::AsyncWith(s) => self.check_statement_list(source, &s.body, violations),
            Stmt::Try(s) => {
                self.check_statement_list(source, &s.body, violations);
                for handler in &s.handlers {
                    let ExceptHandler::ExceptHandler(h) = handler;
                    self.check_statement_list(source, &h.body, violations);
                }
                self.check_statement_list(source, &s.orelse, violations);
                self.check_statement_list(source, &s.finalbody, violations);
            }
            Stmt::Match(s) => {
                for case in &s.cases {
                    self.check_statement_list(source, &case.body, violations);
                }
            }
            _ => {}
        }
    }
}

impl FormattingRule for MultilineSpacingRule {
    fn name(&self) -> &str {
        "multiline-spacing"
    }

    fn apply(&self, source: &str, ast: &Mod) -> Result<Vec<Violation>> {
        let mut violations = Vec::new();

        let statements = match ast {
            Mod::Module(module) => &module.body,
            Mod::Interactive(interactive) => &interactive.body,
            Mod::Expression(_) => return Ok(violations),
            Mod::FunctionType(_) => return Ok(violations),
        };

        self.check_statement_list(source, statements, &mut violations);

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

    #[test]
    fn test_multiline_if_header_no_blank_after() {
        // Multiline if header should need blank before, but NOT after
        let source = r#"x = 1

if (
    a
    and b
):
    pass
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(3);
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 0, "Should not require blank after multiline if header");
    }

    #[test]
    fn test_multiline_if_header_needs_blank_before() {
        // Multiline if header still needs blank before
        let source = r#"x = 1
if (
    a
    and b
):
    pass
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(3);
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 1, "Should require blank before multiline if header");
        assert!(violations[0].message.contains("before"));
    }

    #[test]
    fn test_multiline_function_header_no_blank_after() {
        // Multiline function header should need blank before, but NOT after
        let source = r#"y = 2

def foo(
    arg1,
    arg2,
):
    pass
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(3);
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 0, "Should not require blank after multiline function header");
    }

    #[test]
    fn test_multiline_class_header_no_blank_after() {
        // Multiline class header should need blank before, but NOT after
        let source = r#"x = 1

class MyClass(
    BaseOne,
    BaseTwo,
):
    pass
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(3);
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 0, "Should not require blank after multiline class header");
    }

    #[test]
    fn test_multiline_with_header_no_blank_after() {
        // Multiline with header should need blank before, but NOT after
        let source = r#"x = 1

with (
    open('a') as f1,
    open('b') as f2,
):
    pass
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(3);
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 0, "Should not require blank after multiline with header");
    }

    #[test]
    fn test_multiline_import_ignored() {
        // Multi-line imports are handled by ruff and should never be flagged
        let source = r#"from some.module import (
    ClassA,
    ClassB,
    ClassC,
)
from other.module import SomeClass

x = 1
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(2);
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 0, "Multi-line imports should not be flagged");
    }

    #[test]
    fn test_no_blank_between_decorator_and_def() {
        // Blank line should go BEFORE the decorator, not between decorator and def
        let source = r#"x = 1
@classmethod
def foo(
    cls,
    arg,
):
    pass
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(3);
        let violations = rule.apply(source, &ast).unwrap();

        // Should flag that a blank is needed before @classmethod (line 2), not between decorator and def
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, 2, "Violation should point to the decorator line");
        assert!(violations[0].message.contains("before"));
    }

    #[test]
    fn test_blank_before_decorator_is_sufficient() {
        // A blank line before the decorator — no violations
        let source = r#"x = 1

@classmethod
def foo(
    cls,
    arg,
):
    pass
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(3);
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 0, "Blank before decorator should satisfy the rule");
    }

    #[test]
    fn test_nested_multiline_inside_function_body() {
        // A multiline assignment inside a for loop inside a function should be detected
        let source = r#"def process(items):
    result = {}
    for i, item in enumerate(items):
        key = item
        value = build(
            item,
            index=i,
            extra=None,
        )
        result[i] = value
    return result
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(3);
        let violations = rule.apply(source, &ast).unwrap();

        assert!(!violations.is_empty(), "Should detect violations inside nested scopes");
        let before = violations.iter().any(|v| v.message.contains("before"));
        let after = violations.iter().any(|v| v.message.contains("after"));
        assert!(before, "Should require blank line before the multiline assignment");
        assert!(after, "Should require blank line after the multiline assignment");
    }

    #[test]
    fn test_nested_multiline_with_spacing() {
        // Same structure but with blank lines everywhere needed — should have no violations
        let source = r#"def process(items):
    result = {}

    for i, item in enumerate(items):
        key = item

        value = build(
            item,
            index=i,
            extra=None,
        )

        result[i] = value

    return result
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(3);
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 0, "No violations when nested multiline has surrounding blank lines");
    }

    #[test]
    fn test_single_line_header_still_needs_blank_after() {
        // Single-line header function that spans multiple lines overall still needs blank after
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

        assert_eq!(violations.len(), 1, "Should require blank after function with single-line header");
        assert!(violations[0].message.contains("after"));
    }
}
