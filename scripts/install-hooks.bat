@echo off
REM Install git hooks

set HOOKS_DIR=.git\hooks
set PRE_COMMIT_HOOK=%HOOKS_DIR%\pre-commit

REM Check if we're in a git repository
if not exist ".git" (
    echo Error: Not in a git repository!
    exit /b 1
)

echo Installing pre-commit hook...

REM Create the pre-commit hook
(
echo #!/usr/bin/env bash
echo # Pre-commit hook for Aurora Compass
echo # Runs formatting check, clippy, and tests
echo.
echo set -e
echo.
echo echo "Running pre-commit checks..."
echo.
echo # Check formatting
echo echo "1/3 Checking code formatting..."
echo if ! cargo fmt --all -- --check; then
echo     echo "Error: Code is not formatted. Run 'cargo fmt --all' to fix."
echo     exit 1
echo fi
echo.
echo # Run clippy
echo echo "2/3 Running clippy..."
echo if ! cargo clippy --all-targets --all-features -- -D warnings; then
echo     echo "Error: Clippy found issues. Fix them before committing."
echo     exit 1
echo fi
echo.
echo # Run tests
echo echo "3/3 Running tests..."
echo if ! cargo test --all --quiet; then
echo     echo "Error: Tests failed. Fix them before committing."
echo     exit 1
echo fi
echo.
echo echo "✓ Pre-commit checks passed!"
) > "%PRE_COMMIT_HOOK%"

echo ✓ Pre-commit hook installed successfully!
echo.
echo The hook will run automatically before each commit.
echo To skip the hook (not recommended), use: git commit --no-verify
