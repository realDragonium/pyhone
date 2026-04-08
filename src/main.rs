mod config;
mod formatter;
mod output;
mod parser;
mod rules;

use anyhow::{Context, Result};
use clap::Parser;
use config::Config;
use formatter::Formatter;
use output::{OutputFormat, OutputFormatter};
use rules::FixKind;
use std::path::PathBuf;

/// Pyhone - A Python code formatter with custom formatting rules
#[derive(Parser, Debug)]
#[command(name = "pyhone")]
#[command(version, about = "A Python code formatter that complements Ruff", long_about = None)]
struct Args {
    /// Python files to format
    #[arg(required = true)]
    files: Vec<PathBuf>,

    /// Path to configuration file
    #[arg(short, long, default_value = "pyhone.toml")]
    config: PathBuf,

    /// Check mode: report violations without fixing
    #[arg(long)]
    check: bool,

    /// Output format: human or github
    #[arg(short, long, default_value = "human")]
    format: String,
}

const EXCLUDED_DIRS: &[&str] = &[
    ".venv",
    "venv",
    "env",
    ".env",
    "__pycache__",
    ".tox",
    ".nox",
    ".mypy_cache",
    ".ruff_cache",
    ".pytest_cache",
    "node_modules",
    "dist",
    "build",
    "site-packages",
    ".git",
];

fn collect_python_files(dir: &PathBuf, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?
    {
        let path = entry?.path();
        if path.is_dir() {
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if EXCLUDED_DIRS.contains(&dir_name) {
                continue;
            }
            collect_python_files(&path, files)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("py") {
            files.push(path);
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    let output_format = OutputFormat::from_str(&args.format)
        .ok_or_else(|| anyhow::anyhow!("Invalid output format: {}", args.format))?;

    let output_formatter = OutputFormatter::new(output_format);

    let config = if args.config.exists() {
        Config::from_file(&args.config)
            .context("Failed to load configuration file")?
    } else {
        if output_format == OutputFormat::Human {
            eprintln!("Config file not found, using defaults");
        }
        Config::default()
    };

    let formatter = Formatter::new(config);

    let mut total_violations = 0;
    let mut total_files = 0;

    let mut python_files: Vec<PathBuf> = Vec::new();
    for path in &args.files {
        if !path.exists() {
            eprintln!("Warning: Path not found: {}", path.display());
            continue;
        }
        if path.is_dir() {
            collect_python_files(path, &mut python_files)?;
        } else {
            python_files.push(path.clone());
        }
    }

    for file_path in &python_files {
        total_files += 1;

        let violations = if args.check {
            formatter.check_file(file_path)
                .with_context(|| format!("Failed to check file: {}", file_path.display()))?
        } else {
            formatter.format_file(file_path)
                .with_context(|| format!("Failed to format file: {}", file_path.display()))?
        };

        if !violations.is_empty() {
            if args.check {
                let output_lines = output_formatter.format_violations(file_path, &violations);
                for line in output_lines {
                    println!("{}", line);
                }
            } else {
                let unfixable: Vec<_> = violations
                    .iter()
                    .filter(|v| v.fix_kind == FixKind::None)
                    .cloned()
                    .collect();
                if !unfixable.is_empty() {
                    let output_lines = output_formatter.format_violations(file_path, &unfixable);
                    for line in output_lines {
                        println!("{}", line);
                    }
                }
            }
            total_violations += violations.len();
        }
    }

    output_formatter.print_summary(total_files, total_violations, args.check);

    if args.check && total_violations > 0 {
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_collect_python_files_recurses_directories() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::write(root.join("a.py"), "x = 1\n").unwrap();
        fs::write(root.join("b.txt"), "not python\n").unwrap();

        let sub = root.join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("c.py"), "y = 2\n").unwrap();

        let mut files = Vec::new();
        collect_python_files(&root.to_path_buf(), &mut files).unwrap();

        let mut names: Vec<_> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap())
            .collect();
        names.sort();

        assert_eq!(names, vec!["a.py", "c.py"]);
    }

    #[test]
    fn test_collect_python_files_excludes_dependency_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::write(root.join("app.py"), "x = 1\n").unwrap();

        for excluded in &[".venv", "venv", "__pycache__", ".tox", "node_modules"] {
            let excluded_dir = root.join(excluded);
            fs::create_dir(&excluded_dir).unwrap();
            fs::write(excluded_dir.join("should_be_ignored.py"), "y = 2\n").unwrap();
        }

        let mut files = Vec::new();
        collect_python_files(&root.to_path_buf(), &mut files).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name().unwrap().to_str().unwrap(), "app.py");
    }

    #[test]
    fn test_collect_python_files_ignores_non_python() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::write(root.join("script.py"), "x = 1\n").unwrap();
        fs::write(root.join("readme.md"), "# docs\n").unwrap();
        fs::write(root.join("config.toml"), "[tool]\n").unwrap();

        let mut files = Vec::new();
        collect_python_files(&root.to_path_buf(), &mut files).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name().unwrap().to_str().unwrap(), "script.py");
    }
}
