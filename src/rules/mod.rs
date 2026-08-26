pub mod import_hoisting;
pub mod max_lines;
pub mod multiline_spacing;

use ruff_python_ast::ModModule;

#[derive(Debug, Clone, PartialEq)]
pub enum FixKind {
    InsertBlankBefore,
    InsertBlankAfter,
    RemoveBlankBefore,
    None,
}

#[derive(Debug, Clone)]
pub struct Violation {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub rule_name: String,
    pub fix_kind: FixKind,
}

pub trait FormattingRule {
    fn name(&self) -> &str;
    fn apply(&self, source: &str, ast: &ModModule) -> anyhow::Result<Vec<Violation>>;
}

pub struct RuleRegistry {
    rules: Vec<Box<dyn FormattingRule>>,
}

impl RuleRegistry {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn register(&mut self, rule: Box<dyn FormattingRule>) {
        self.rules.push(rule);
    }

    pub fn rules(&self) -> &[Box<dyn FormattingRule>] {
        &self.rules
    }
}

impl Default for RuleRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(import_hoisting::ImportHoistingRule));
        registry.register(Box::new(max_lines::MaxLinesRule::default()));
        registry.register(Box::new(multiline_spacing::MultilineSpacingRule::default()));
        registry
    }
}
