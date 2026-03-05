use crate::rules::Violation;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    Human,
    Github,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "human" => Some(Self::Human),
            "github" => Some(Self::Github),
            _ => None,
        }
    }
}

pub struct OutputFormatter {
    format: OutputFormat,
}

impl OutputFormatter {
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }

    pub fn format_violations(&self, file_path: &Path, violations: &[Violation]) -> Vec<String> {
        match self.format {
            OutputFormat::Human => self.format_human(file_path, violations),
            OutputFormat::Github => self.format_github(file_path, violations),
        }
    }

    fn format_human(&self, file_path: &Path, violations: &[Violation]) -> Vec<String> {
        let mut lines = Vec::new();

        if violations.is_empty() {
            return lines;
        }

        lines.push(format!("\n{}:", file_path.display()));

        for violation in violations {
            lines.push(format!(
                "  {}:{} - {} [{}]",
                violation.line, violation.column, violation.message, violation.rule_name
            ));
        }

        lines
    }

    fn format_github(&self, file_path: &Path, violations: &[Violation]) -> Vec<String> {
        violations
            .iter()
            .map(|v| {
                format!(
                    "::error file={},line={},col={}::{} [{}]",
                    file_path.display(),
                    v.line,
                    v.column,
                    v.message,
                    v.rule_name
                )
            })
            .collect()
    }

    pub fn print_summary(&self, total_files: usize, total_violations: usize, check: bool) {
        if self.format == OutputFormat::Human && check {
            println!("\n{} file(s) checked, {} violation(s) found", total_files, total_violations);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{FixKind, Violation};

    #[test]
    fn test_format_human() {
        let formatter = OutputFormatter::new(OutputFormat::Human);
        let violations = vec![
            Violation {
                line: 10,
                column: 5,
                message: "Missing blank line".to_string(),
                rule_name: "multiline-spacing".to_string(),
                fix_kind: FixKind::InsertBlankBefore,
            },
        ];

        let output = formatter.format_violations(Path::new("test.py"), &violations);
        assert_eq!(output.len(), 2);
        assert!(output[0].contains("test.py"));
        assert!(output[1].contains("10:5"));
    }

    #[test]
    fn test_format_github() {
        let formatter = OutputFormatter::new(OutputFormat::Github);
        let violations = vec![
            Violation {
                line: 10,
                column: 5,
                message: "Missing blank line".to_string(),
                rule_name: "multiline-spacing".to_string(),
                fix_kind: FixKind::InsertBlankBefore,
            },
        ];

        let output = formatter.format_violations(Path::new("test.py"), &violations);
        assert_eq!(output.len(), 1);
        assert!(output[0].starts_with("::error"));
        assert!(output[0].contains("file=test.py"));
        assert!(output[0].contains("line=10"));
    }
}
