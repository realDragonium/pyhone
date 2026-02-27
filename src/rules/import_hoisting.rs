use crate::rules::{FixKind, FormattingRule, Violation};
use anyhow::Result;
use rustpython_parser::ast::{Mod, Ranged, Stmt};

#[derive(Debug)]
pub struct ImportHoistingRule;

impl ImportHoistingRule {
    pub fn new() -> Self {
        Self
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
                    self.check_conditional_imports(source, &try_stmt.body, func_name, violations);
                    for handler in &try_stmt.handlers {
                        let rustpython_parser::ast::ExceptHandler::ExceptHandler(h) = handler;
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
    fn test_import_in_try_except() {
        let source = r#"def safe_import():
    try:
        import optional_module
    except ImportError:
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
