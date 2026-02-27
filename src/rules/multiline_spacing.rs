use crate::rules::{FixKind, FormattingRule, Violation};
use anyhow::Result;
use rustpython_parser::ast::{Expr, ExceptHandler, Mod, Ranged, Stmt};

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
        self.check_statement_list_inner(source, statements, violations, false);
    }

    fn check_class_body(&self, source: &str, statements: &[Stmt], violations: &mut Vec<Violation>) {
        self.check_statement_list_inner(source, statements, violations, true);
    }

    fn check_statement_list_inner(&self, source: &str, statements: &[Stmt], violations: &mut Vec<Violation>, in_class_body: bool) {
        for (i, stmt) in statements.iter().enumerate() {
            // Recurse into nested bodies regardless of whether this statement is multiline
            self.recurse_into_stmt(source, stmt, violations);

            // Class-level Assign/AnnAssign are attribute definitions — skip spacing checks.
            // If a blank line exists between two consecutive class-level assignments, flag it
            // for removal.
            if in_class_body && matches!(stmt, Stmt::Assign(_) | Stmt::AnnAssign(_)) {
                if i > 0 && matches!(statements[i - 1], Stmt::Assign(_) | Stmt::AnnAssign(_)) {
                    let start_line = self.get_line_number(source, stmt.range().start().to_usize());
                    if self.has_blank_line_before(source, start_line) {
                        violations.push(Violation {
                            line: start_line,
                            column: 1,
                            message: "Class-level assignment should not have a blank line before it".to_string(),
                            rule_name: self.name().to_string(),
                            fix_kind: FixKind::RemoveBlankBefore,
                        });
                    }
                }
                continue;
            }

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
            let prev_stmt = if i > 0 { Some(&statements[i - 1]) } else { None };
            let prev_is_multiline = prev_stmt.map_or(false, |p| self.is_multiline(source, p));
            let prev_is_loop_setup = prev_stmt.map_or(false, |p| self.is_loop_setup(source, p, stmt));
            let prev_is_if_guard_setup = prev_stmt.map_or(false, |p| self.is_if_guard_setup(source, p, stmt));
            let prev_is_paired_setup = prev_is_loop_setup || prev_is_if_guard_setup;

            if prev_is_paired_setup && self.has_blank_line_before(source, start_line) {
                let msg = if prev_is_loop_setup {
                    "Loop setup assignment should not have a blank line before the loop"
                } else {
                    "Guard assignment should not have a blank line before the if statement"
                };
                violations.push(Violation {
                    line: start_line,
                    column: 1,
                    message: msg.to_string(),
                    rule_name: self.name().to_string(),
                    fix_kind: FixKind::RemoveBlankBefore,
                });
            }

            if !is_first && !prev_is_multiline && !prev_is_paired_setup && !self.has_blank_line_before(source, start_line) {
                violations.push(Violation {
                    line: start_line,
                    column: 1,
                    message: format!(
                        "Multi-line statement ({} lines) should have a blank line before it",
                        line_count
                    ),
                    rule_name: self.name().to_string(),
                    fix_kind: FixKind::InsertBlankBefore,
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
                    fix_kind: FixKind::InsertBlankAfter,
                });
            }
        }
    }

    /// A single-line assignment immediately before a for/while loop is treated as
    /// loop setup (e.g. accumulator initialisation) and doesn't need a blank line.
    fn is_loop_setup(&self, source: &str, prev: &Stmt, curr: &Stmt) -> bool {
        let prev_is_single_assignment = matches!(prev, Stmt::Assign(_) | Stmt::AnnAssign(_))
            && !self.is_multiline(source, prev);
        let curr_is_loop = matches!(curr, Stmt::For(_) | Stmt::AsyncFor(_) | Stmt::While(_));
        prev_is_single_assignment && curr_is_loop
    }

    /// A single-line assignment immediately before an if statement that references
    /// the assigned variable in its condition is treated as a guard setup and
    /// doesn't need a blank line.
    fn is_if_guard_setup(&self, source: &str, prev: &Stmt, curr: &Stmt) -> bool {
        let prev_is_single_assignment = matches!(prev, Stmt::Assign(_) | Stmt::AnnAssign(_))
            && !self.is_multiline(source, prev);

        if !prev_is_single_assignment {
            return false;
        }

        let Stmt::If(if_stmt) = curr else { return false };

        let Some(var_name) = self.extract_assign_target_name(prev) else { return false };

        // Case 1: variable appears in the if condition (guard check)
        //   created_at = value.created_at
        //   if created_at is None: ...
        let test_range = if_stmt.test.range();
        let condition_src = &source[test_range.start().to_usize()..test_range.end().to_usize()];
        let in_condition = condition_src
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .any(|word| word == var_name);

        if in_condition {
            return true;
        }

        // Case 2: default assignment + single-statement override in if body
        //   Accepted with or without an else, as long as every branch assigns
        //   to the same variable.
        //
        //   Without else:
        //     created_by_mask = None
        //     if dto.something:
        //         created_by_mask = compute(...)
        //
        //   With else:
        //     created_by_mask = None
        //     if dto.something:
        //         created_by_mask = compute(...)
        //     else:
        //         created_by_mask = other_value()
        if if_stmt.body.len() == 1 {
            if let Some(body_var) = self.extract_assign_target_name(&if_stmt.body[0]) {
                if body_var == var_name {
                    // No else, or else is also a single assignment to the same variable
                    let else_ok = if_stmt.orelse.is_empty()
                        || (if_stmt.orelse.len() == 1
                            && self.extract_assign_target_name(&if_stmt.orelse[0])
                                == Some(var_name));
                    if else_ok {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Extract the simple variable name from an Assign or AnnAssign target, if it's a plain Name.
    fn extract_assign_target_name<'a>(&self, stmt: &'a Stmt) -> Option<&'a str> {
        match stmt {
            Stmt::Assign(a) => {
                if let Some(target) = a.targets.first() {
                    if let Expr::Name(name) = target {
                        return Some(name.id.as_str());
                    }
                }
                None
            }
            Stmt::AnnAssign(a) => {
                if let Expr::Name(name) = &*a.target {
                    return Some(name.id.as_str());
                }
                None
            }
            _ => None,
        }
    }

    fn recurse_into_stmt(&self, source: &str, stmt: &Stmt, violations: &mut Vec<Violation>) {
        match stmt {
            Stmt::FunctionDef(s) => self.check_statement_list(source, &s.body, violations),
            Stmt::AsyncFunctionDef(s) => self.check_statement_list(source, &s.body, violations),
            Stmt::ClassDef(s) => self.check_class_body(source, &s.body, violations),
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
    fn test_if_guard_setup_blank_removed() {
        // A blank line between an assignment and an if that checks the variable should be removed
        let source = r#"def process(value):
    created_at = value.created_at

    if created_at is None:
        created_at = now()

    return created_at
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(2);
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fix_kind, crate::rules::FixKind::RemoveBlankBefore);
        assert!(violations[0].message.contains("Guard assignment"));
    }

    #[test]
    fn test_if_default_override_blank_removed() {
        // Default assignment followed by a single-statement if that overrides it
        let source = r#"def process(dto):
    created_by_mask = None

    if dto.created_by:
        created_by_mask = get_mask(dto.created_by)

    return created_by_mask
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(2);
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fix_kind, crate::rules::FixKind::RemoveBlankBefore);
    }

    #[test]
    fn test_if_else_default_override_blank_removed() {
        // if/else where both branches assign to the same variable — blank should be removed
        let source = r#"def process(dto):
    created_by_mask = None

    if dto.created_by:
        created_by_mask = get_mask(dto.created_by)
    else:
        created_by_mask = default_mask()

    return created_by_mask
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(2);
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fix_kind, crate::rules::FixKind::RemoveBlankBefore);
    }

    #[test]
    fn test_if_else_different_variables_kept() {
        // if/else branches assign to different variables — leave the blank alone
        let source = r#"def process(dto):
    created_by_mask = None

    if dto.created_by:
        created_by_mask = get_mask(dto.created_by)
    else:
        other_mask = default_mask()

    return created_by_mask
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(2);
        let violations = rule.apply(source, &ast).unwrap();

        assert!(
            violations.iter().all(|v| v.fix_kind != crate::rules::FixKind::RemoveBlankBefore),
            "if/else with different variables should not have its blank removed"
        );
    }

    #[test]
    fn test_if_guard_unrelated_variable_kept() {
        // A blank line before an if that does NOT use the assigned variable should NOT be removed
        let source = r#"def process(value):
    result = compute(value)

    if some_flag:
        do_something()

    return result
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(2);
        let violations = rule.apply(source, &ast).unwrap();

        assert!(
            violations.iter().all(|v| v.fix_kind != crate::rules::FixKind::RemoveBlankBefore),
            "Blank before unrelated if should not be removed"
        );
    }

    #[test]
    fn test_loop_setup_blank_removed() {
        // A blank line between a loop setup assignment and the loop should be flagged for removal
        let source = r#"def collect(items):
    results: list[str] = []

    for item in items:
        results.append(item)

    return results
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(2);
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fix_kind, crate::rules::FixKind::RemoveBlankBefore);
        assert!(violations[0].message.contains("Loop setup"));
    }

    #[test]
    fn test_loop_setup_no_blank_needed() {
        // A single-line assignment directly before a loop is treated as
        // loop setup — no blank line required between them
        let source = r#"def collect(items):
    results = []
    for item in items:
        results.append(item)

    return results
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(2);
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 0, "Accumulator assignment before loop should not require blank line");
    }

    #[test]
    fn test_loop_setup_annotated_assignment() {
        // Annotated assignment (e.g. type hint) before a loop is also loop setup
        let source = r#"def collect(items):
    results: list[str] = []
    for item in items:
        results.append(item)

    return results
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(2);
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 0, "Annotated accumulator assignment before loop should not require blank line");
    }

    #[test]
    fn test_class_body_blank_between_assignments_flagged() {
        // A blank line between two consecutive class-level assignments should be flagged for removal
        let source = r#"class Status(Enum):
    PENDING = "pending"

    DONE = "done"
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(2);
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fix_kind, crate::rules::FixKind::RemoveBlankBefore);
        assert!(violations[0].message.contains("should not have a blank line"));
    }

    #[test]
    fn test_class_body_assignments_ignored() {
        // Assign/AnnAssign directly inside a class body should not be flagged —
        // this covers Enum members, dataclass fields, Pydantic models, etc.
        let source = r#"class Status(Enum):
    PENDING = "pending"
    IN_PROGRESS = "in_progress"
    DONE = "done"


class MyModel(BaseModel):
    name: str
    age: int
    tags: list[str] = []
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(2);
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 0, "Class-level assignments should not require blank lines between them");
    }

    #[test]
    fn test_class_body_methods_still_checked() {
        // Methods inside a class ARE still subject to spacing rules
        let source = r#"class MyClass:
    x = 1
    def foo(
        self,
        arg,
    ):
        pass
"#;

        let ast = parse_python(source).unwrap();
        let rule = MultilineSpacingRule::new(2);
        let violations = rule.apply(source, &ast).unwrap();

        // foo() is multiline and follows x = 1 (an assignment, but curr is FunctionDef not loop)
        // so a blank is still required before foo()
        assert!(!violations.is_empty(), "Multiline methods in a class should still be checked");
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
