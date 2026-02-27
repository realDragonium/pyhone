use crate::rules::{FixKind, FormattingRule, Violation};
use anyhow::Result;
use rustpython_parser::ast::{Expr, ExceptHandler, Mod, Ranged, Stmt};

#[derive(Debug)]
pub struct ImportHoistingRule;

impl ImportHoistingRule {
    pub fn new() -> Self {
        Self
    }

    /// Returns true if any except handler catches ImportError or ModuleNotFoundError,
    /// indicating this try block is the standard pattern for optional imports.
    fn is_optional_import_block(&self, handlers: &[rustpython_parser::ast::ExceptHandler]) -> bool {
        handlers.iter().any(|h| {
            let ExceptHandler::ExceptHandler(handler) = h;
            handler.type_.as_ref().map_or(false, |t| {
                matches!(t.as_ref(), Expr::Name(n) if
                    n.id.as_str() == "ImportError" || n.id.as_str() == "ModuleNotFoundError"
                )
            })
        })
    }

    fn get_line_number(&self, source: &str, offset: usize) -> usize {
        if offset == 0 {
            return 1;
        }
        source[..offset].matches('\n').count() + 1
    }

    fn check_statements_for_imports(
        &self,
        source: &str,
        statements: &[Stmt],
        violations: &mut Vec<Violation>,
        _scope_name: &str,
    ) {
        for stmt in statements {
            match stmt {
                Stmt::FunctionDef(func) => {
                    self.check_function_body(source, &func.body, &func.name, violations);
                }
                Stmt::AsyncFunctionDef(func) => {
                    self.check_function_body(source, &func.body, &func.name, violations);
                }
                Stmt::ClassDef(class) => {
                    self.check_statements_for_imports(
                        source,
                        &class.body,
                        violations,
                        &class.name,
                    );
                }
                _ => {}
            }
        }
    }

    fn check_function_body(
        &self,
        source: &str,
        body: &[Stmt],
        func_name: &str,
        violations: &mut Vec<Violation>,
    ) {
        for stmt in body {
            match stmt {
                Stmt::Import(import_stmt) => {
                    let range = import_stmt.range();
                    let start = range.start().to_usize();
                    let line = self.get_line_number(source, start);

                    let module_names: Vec<String> = import_stmt
                        .names
                        .iter()
                        .map(|alias| alias.name.to_string())
                        .collect();

                    violations.push(Violation {
                        line,
                        column: 1,
                        message: format!(
                            "Import '{}' inside function '{}' should be moved to top of file",
                            module_names.join(", "),
                            func_name
                        ),
                        rule_name: self.name().to_string(),
                        fix_kind: FixKind::None,
                    });
                }
                Stmt::ImportFrom(import_from) => {
                    let range = import_from.range();
                    let start = range.start().to_usize();
                    let line = self.get_line_number(source, start);

                    let module_name = import_from
                        .module
                        .as_ref()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| ".".to_string());

                    violations.push(Violation {
                        line,
                        column: 1,
                        message: format!(
                            "Import from '{}' inside function '{}' should be moved to top of file",
                            module_name,
                            func_name
                        ),
                        rule_name: self.name().to_string(),
                        fix_kind: FixKind::None,
                    });
                }
                Stmt::FunctionDef(nested_func) => {
                    self.check_function_body(source, &nested_func.body, &nested_func.name, violations);
                }
                Stmt::AsyncFunctionDef(nested_func) => {
                    self.check_function_body(source, &nested_func.body, &nested_func.name, violations);
                }
                Stmt::ClassDef(class) => {
                    self.check_statements_for_imports(source, &class.body, violations, &class.name);
                }
                Stmt::If(if_stmt) => {
                    self.check_conditional_imports(source, &if_stmt.body, func_name, violations);
                    self.check_conditional_imports(source, &if_stmt.orelse, func_name, violations);
                }
                Stmt::For(for_stmt) => {
                    self.check_conditional_imports(source, &for_stmt.body, func_name, violations);
                    self.check_conditional_imports(source, &for_stmt.orelse, func_name, violations);
                }
                Stmt::AsyncFor(for_stmt) => {
                    self.check_conditional_imports(source, &for_stmt.body, func_name, violations);
                    self.check_conditional_imports(source, &for_stmt.orelse, func_name, violations);
                }
                Stmt::While(while_stmt) => {
                    self.check_conditional_imports(source, &while_stmt.body, func_name, violations);
                    self.check_conditional_imports(source, &while_stmt.orelse, func_name, violations);
                }
                Stmt::With(with_stmt) => {
                    self.check_conditional_imports(source, &with_stmt.body, func_name, violations);
                }
                Stmt::AsyncWith(with_stmt) => {
                    self.check_conditional_imports(source, &with_stmt.body, func_name, violations);
                }
                Stmt::Try(try_stmt) => {
                    // A try/except ImportError (or ModuleNotFoundError) is the standard
                    // pattern for optional imports — don't flag them
                    if !self.is_optional_import_block(&try_stmt.handlers) {
                        self.check_conditional_imports(source, &try_stmt.body, func_name, violations);
                    }
                    for handler in &try_stmt.handlers {
                        let ExceptHandler::ExceptHandler(h) = handler;
                        self.check_conditional_imports(source, &h.body, func_name, violations);
                    }
                    self.check_conditional_imports(source, &try_stmt.orelse, func_name, violations);
                    self.check_conditional_imports(source, &try_stmt.finalbody, func_name, violations);
                }
                _ => {}
            }
        }
    }

    fn check_conditional_imports(
        &self,
        source: &str,
        body: &[Stmt],
        func_name: &str,
        violations: &mut Vec<Violation>,
    ) {
        for stmt in body {
            match stmt {
                Stmt::Import(import_stmt) => {
                    let range = import_stmt.range();
                    let start = range.start().to_usize();
                    let line = self.get_line_number(source, start);

                    let module_names: Vec<String> = import_stmt
                        .names
                        .iter()
                        .map(|alias| alias.name.to_string())
                        .collect();

                    violations.push(Violation {
                        line,
                        column: 1,
                        message: format!(
                            "Import '{}' inside function '{}' should be moved to top of file",
                            module_names.join(", "),
                            func_name
                        ),
                        rule_name: self.name().to_string(),
                        fix_kind: FixKind::None,
                    });
                }
                Stmt::ImportFrom(import_from) => {
                    let range = import_from.range();
                    let start = range.start().to_usize();
                    let line = self.get_line_number(source, start);

                    let module_name = import_from
                        .module
                        .as_ref()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| ".".to_string());

                    violations.push(Violation {
                        line,
                        column: 1,
                        message: format!(
                            "Import from '{}' inside function '{}' should be moved to top of file",
                            module_name,
                            func_name
                        ),
                        rule_name: self.name().to_string(),
                        fix_kind: FixKind::None,
                    });
                }
                Stmt::Try(try_stmt) => {
                    if !self.is_optional_import_block(&try_stmt.handlers) {
                        self.check_conditional_imports(source, &try_stmt.body, func_name, violations);
                    }
                    for handler in &try_stmt.handlers {
                        let ExceptHandler::ExceptHandler(h) = handler;
                        self.check_conditional_imports(source, &h.body, func_name, violations);
                    }
                    self.check_conditional_imports(source, &try_stmt.orelse, func_name, violations);
                    self.check_conditional_imports(source, &try_stmt.finalbody, func_name, violations);
                }
                _ => {
                    self.check_function_body(source, std::slice::from_ref(stmt), func_name, violations);
                }
            }
        }
    }
}

impl Default for ImportHoistingRule {
    fn default() -> Self {
        Self::new()
    }
}

impl FormattingRule for ImportHoistingRule {
    fn name(&self) -> &str {
        "import-hoisting"
    }

    fn apply(&self, source: &str, ast: &Mod) -> Result<Vec<Violation>> {
        let mut violations = Vec::new();

        let statements = match ast {
            Mod::Module(module) => &module.body,
            Mod::Interactive(interactive) => &interactive.body,
            Mod::Expression(_) => return Ok(violations),
            Mod::FunctionType(_) => return Ok(violations),
        };

        self.check_statements_for_imports(source, statements, &mut violations, "module");

        Ok(violations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_python;

    #[test]
    fn test_import_inside_function() {
        let source = r#"def foo():
    import os
    print(os.getcwd())
"#;

        let ast = parse_python(source).unwrap();
        let rule = ImportHoistingRule::new();
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("Import"));
        assert!(violations[0].message.contains("foo"));
    }

    #[test]
    fn test_import_from_inside_function() {
        let source = r#"def bar():
    from pathlib import Path
    return Path.cwd()
"#;

        let ast = parse_python(source).unwrap();
        let rule = ImportHoistingRule::new();
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("pathlib"));
        assert!(violations[0].message.contains("bar"));
    }

    #[test]
    fn test_module_level_import_ok() {
        let source = r#"import os
from pathlib import Path

def foo():
    print(os.getcwd())
    return Path.cwd()
"#;

        let ast = parse_python(source).unwrap();
        let rule = ImportHoistingRule::new();
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_nested_function_import() {
        let source = r#"def outer():
    def inner():
        import json
        return json.dumps({})
    return inner()
"#;

        let ast = parse_python(source).unwrap();
        let rule = ImportHoistingRule::new();
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("inner"));
    }

    #[test]
    fn test_import_in_conditional() {
        let source = r#"def process():
    if True:
        import sys
        print(sys.version)
"#;

        let ast = parse_python(source).unwrap();
        let rule = ImportHoistingRule::new();
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_import_in_try_except_import_error_allowed() {
        // try/except ImportError is the standard pattern for optional imports — not flagged
        let source = r#"def safe_import():
    try:
        import optional_module
    except ImportError:
        pass
"#;

        let ast = parse_python(source).unwrap();
        let rule = ImportHoistingRule::new();
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_import_in_try_except_module_not_found_allowed() {
        // ModuleNotFoundError is a subclass of ImportError — also allowed
        let source = r#"def safe_import():
    try:
        import optional_module
        tracer = optional_module.tracer()
    except ModuleNotFoundError:
        tracer = None
"#;

        let ast = parse_python(source).unwrap();
        let rule = ImportHoistingRule::new();
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_import_in_try_except_other_exception_flagged() {
        // A try/except catching something other than ImportError is still flagged
        let source = r#"def process():
    try:
        import some_module
    except Exception:
        pass
"#;

        let ast = parse_python(source).unwrap();
        let rule = ImportHoistingRule::new();
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_async_function_import() {
        let source = r#"async def fetch():
    import aiohttp
    async with aiohttp.ClientSession() as session:
        pass
"#;

        let ast = parse_python(source).unwrap();
        let rule = ImportHoistingRule::new();
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("fetch"));
    }

    #[test]
    fn test_class_method_import() {
        let source = r#"class MyClass:
    def method(self):
        import re
        return re.match(r'\d+', '123')
"#;

        let ast = parse_python(source).unwrap();
        let rule = ImportHoistingRule::new();
        let violations = rule.apply(source, &ast).unwrap();

        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("method"));
    }
}
