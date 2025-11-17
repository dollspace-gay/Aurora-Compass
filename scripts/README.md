# Development Scripts

This directory contains development scripts for Aurora Compass.

## Available Scripts

### build.sh / build.bat
Build all crates in the workspace.

```bash
# Unix/Mac
./scripts/build.sh

# Windows
scripts\build.bat
```

### format.sh / format.bat
Format all Rust code using rustfmt.

```bash
# Unix/Mac
./scripts/format.sh

# Windows
scripts\format.bat
```

### lint.sh / lint.bat
Run clippy on all code with strict linting rules.

```bash
# Unix/Mac
./scripts/lint.sh

# Windows
scripts\lint.bat
```

### test.sh / test.bat
Run all tests in the workspace.

```bash
# Unix/Mac
./scripts/test.sh

# Windows
scripts\test.bat
```

### check.sh / check.bat
Run all checks: formatting, linting, and tests. This is the comprehensive check that runs before commits.

```bash
# Unix/Mac
./scripts/check.sh

# Windows
scripts\check.bat
```

### install-hooks.sh / install-hooks.bat
Install git pre-commit hooks that automatically run checks before each commit.

```bash
# Unix/Mac
./scripts/install-hooks.sh

# Windows
scripts\install-hooks.bat
```

## Pre-commit Hook

The pre-commit hook automatically runs:
1. Code formatting check (`cargo fmt -- --check`)
2. Clippy linting (`cargo clippy -- -D warnings`)
3. All tests (`cargo test --all`)

To skip the hook (not recommended), use:
```bash
git commit --no-verify
```

## Configuration Files

### rustfmt.toml
Rust code formatting configuration. Key settings:
- 100 character line width
- 4-space indentation
- Optimized import grouping
- Comment wrapping enabled

### clippy.toml
Clippy linting configuration. Key settings:
- Cognitive complexity threshold: 30
- Max function lines: 150
- Max function arguments: 8
- Strict linting for code quality

## Usage in Development

Typical development workflow:

1. **Initial setup** (once):
   ```bash
   ./scripts/install-hooks.sh  # Install pre-commit hooks
   ```

2. **Before committing**:
   ```bash
   ./scripts/format.sh  # Format code
   ./scripts/check.sh   # Run all checks
   ```

3. **During development**:
   ```bash
   ./scripts/build.sh   # Build
   ./scripts/test.sh    # Test
   ./scripts/lint.sh    # Lint
   ```

The pre-commit hook will automatically run checks, but you can run them manually anytime using `./scripts/check.sh`.
