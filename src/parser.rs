use anyhow::{Context, Result};
use ruff_python_ast::ModModule;

pub fn parse_python(source: &str) -> Result<ModModule> {
    let parsed = ruff_python_parser::parse_module(source)
        .context("Failed to parse Python source code")?;

    Ok(parsed.into_syntax())
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

    #[test]
    fn test_parse_unparenthesized_except_tuple() {
        let source = "try:\n    pass\nexcept ValueError, TypeError:\n    pass\n";
        let result = parse_python(source);
        assert!(result.is_ok());
    }
}
