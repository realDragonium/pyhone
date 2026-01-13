pub mod import_hoisting;
pub mod multiline_spacing;

use rustpython_parser::ast::Mod;

#[derive(Debug, Clone)]
pub struct Violation {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub rule_name: String,
}

pub trait FormattingRule {
    fn name(&self) -> &str;
    fn apply(&self, source: &str, ast: &Mod) -> anyhow::Result<Vec<Violation>>;
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
        registry.register(Box::new(import_hoisting::ImportHoistingRule::default()));
        registry.register(Box::new(multiline_spacing::MultilineSpacingRule::default()));
        registry
    }
}
