# Pyhone

A Python code formatter with custom formatting rules that complement Ruff. Pyhone focuses on stylistic rules that go beyond what typical formatters handle.

## Features

- **Multiline Spacing Rule**: Automatically ensures multi-line statements (3+ lines by default) have blank lines before and after them for better readability
- **Format Mode**: Automatically fix formatting issues
- **Check Mode**: Report violations without modifying files (useful for CI/CD)
- **GitHub Actions Integration**: Output violations in GitHub Actions annotation format
- **Configurable**: Customize rules via `pyhone.toml`
- **Fast**: Built in Rust for maximum performance

## Installation

```bash
cargo build --release
```

The binary will be available at `target/release/pyhone`.

## Usage

### Format Files

Apply fixes to Python files:

```bash
pyhone file.py
```

### Check Mode

Report violations without fixing:

```bash
pyhone --check file.py
```

### Multiple Files

```bash
pyhone file1.py file2.py file3.py
```

### Output Formats

**Human-readable (default)**:
```bash
pyhone --format human --check file.py
```

**GitHub Actions**:
```bash
pyhone --format github --check file.py
```

### Custom Configuration

Specify a custom config file:

```bash
pyhone --config my-config.toml file.py
```

## Configuration

Create a `pyhone.toml` file in your project root:

```toml
[rules.multiline-spacing]
# Enable/disable the multiline-spacing rule
enabled = true

# Minimum number of lines for a statement to be considered "multi-line"
# Default: 3
min_lines = 3
```

## Rules

### Multiline Spacing

Ensures multi-line statements have blank lines before and after them for improved readability.

**Example:**

Before:
```python
x = 1
def foo():
    a = 1
    b = 2
    c = 3
    d = 4
y = 2
```

After:
```python
x = 1

def foo():
    a = 1
    b = 2
    c = 3
    d = 4

y = 2
```

## GitHub Actions Integration

Use Pyhone in your GitHub Actions workflow:

```yaml
name: Code Quality

on: [push, pull_request]

jobs:
  formatting:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Build Pyhone
        run: cargo build --release

      - name: Check Python formatting
        run: |
          find . -name "*.py" -exec ./target/release/pyhone --check --format github {} +
```

This will automatically annotate your PRs with formatting issues.

## Working with Ruff

Pyhone is designed to complement Ruff, not replace it. Use Ruff for:
- Import sorting
- Code linting
- Standard formatting rules

Use Pyhone for:
- Custom spacing rules
- Project-specific style enforcement

Example workflow:
```bash
# Run Ruff first
ruff check --fix .
ruff format .

# Then run Pyhone
pyhone **/*.py
```

## Development

### Running Tests

```bash
cargo test
```

### Project Structure

```
pyhone/
├── src/
│   ├── main.rs              # CLI with clap
│   ├── parser.rs            # Python AST parsing
│   ├── config.rs            # TOML config loading
│   ├── formatter.rs         # Orchestrate rules on source code
│   ├── output.rs            # Output formatters (human, github)
│   └── rules/
│       ├── mod.rs           # Rule trait + registry
│       └── multiline_spacing.rs
└── pyhone.toml             # Example config file
```

### Adding New Rules

1. Create a new file in `src/rules/` (e.g., `my_rule.rs`)
2. Implement the `FormattingRule` trait:

```rust
use crate::rules::{FormattingRule, Violation};
use anyhow::Result;
use rustpython_parser::ast::Mod;

pub struct MyRule;

impl FormattingRule for MyRule {
    fn name(&self) -> &str {
        "my-rule"
    }

    fn description(&self) -> &str {
        "Description of my rule"
    }

    fn apply(&self, source: &str, ast: &Mod) -> Result<Vec<Violation>> {
        // Your rule logic here
        Ok(Vec::new())
    }
}
```

3. Register the rule in `src/rules/mod.rs`
4. Add configuration in `src/config.rs`
5. Update the formatter in `src/formatter.rs` to use your rule

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
