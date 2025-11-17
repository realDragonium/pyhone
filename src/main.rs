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
use std::path::PathBuf;

/// Pyhone - A Python code formatter with custom formatting rules
#[derive(Parser, Debug)]
#[command(name = "pyhone")]
#[command(about = "A Python code formatter that complements Ruff", long_about = None)]
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

fn main() -> Result<()> {
    let args = Args::parse();

    // Parse output format
    let output_format = OutputFormat::from_str(&args.format)
        .ok_or_else(|| anyhow::anyhow!("Invalid output format: {}", args.format))?;

    let output_formatter = OutputFormatter::new(output_format);

    // Load configuration
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

    // Process each file
    for file_path in &args.files {
        if !file_path.exists() {
            eprintln!("Warning: File not found: {}", file_path.display());
            continue;
        }

        if !file_path.is_file() {
            eprintln!("Warning: Not a file: {}", file_path.display());
            continue;
        }

        total_files += 1;

        let violations = if args.check {
            // Check mode: just report violations
            formatter.check_file(file_path)
                .with_context(|| format!("Failed to check file: {}", file_path.display()))?
        } else {
            // Format mode: apply fixes
            formatter.format_file(file_path)
                .with_context(|| format!("Failed to format file: {}", file_path.display()))?
        };

        // Output violations
        if !violations.is_empty() {
            let output_lines = output_formatter.format_violations(file_path, &violations);
            for line in output_lines {
                println!("{}", line);
            }
            total_violations += violations.len();
        }
    }

    // Print summary
    output_formatter.print_summary(total_files, total_violations);

    // Exit with error code if violations found in check mode
    if args.check && total_violations > 0 {
        std::process::exit(1);
    }

    Ok(())
}
