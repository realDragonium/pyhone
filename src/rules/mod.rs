pub mod multiline_spacing;

use rustpython_parser::ast::Mod;

/// Represents a formatting violation found by a rule
#[derive(Debug, Clone)]
pub struct Violation {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub rule_name: String,
}

/// Trait that all formatting rules must implement
pub trait FormattingRule {
    /// Returns the name of the rule
    fn name(&self) -> &str;

    /// Returns a description of what the rule does
    fn description(&self) -> &str;

    /// Apply the rule to source code and return violations
    fn apply(&self, source: &str, ast: &Mod) -> anyhow::Result<Vec<Violation>>;
}

/// Registry of all available formatting rules
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
        registry.register(Box::new(multiline_spacing::MultilineSpacingRule::default()));
        registry
    }
}
