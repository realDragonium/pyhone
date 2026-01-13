use anyhow::{Context, Result};
use rustpython_parser::ast::Mod;

pub fn parse_python(source: &str) -> Result<Mod> {
    rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<string>")
        .context("Failed to parse Python source code")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_python() {
        let source = "x = 1\n";
        let result = parse_python(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_invalid_python() {
        let source = "x = \n";
        let result = parse_python(source);
        assert!(result.is_err());
    }
}
