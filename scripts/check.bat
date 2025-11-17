@echo off
REM Run all checks: format, lint, and test

echo ======================================
echo Running all checks...
echo ======================================

echo.
echo 1/3 Checking formatting...
cargo fmt --all -- --check
if %ERRORLEVEL% NEQ 0 (
    echo ✗ Formatting check failed!
    exit /b %ERRORLEVEL%
)

echo.
echo 2/3 Running clippy...
cargo clippy --all-targets --all-features -- -D warnings
if %ERRORLEVEL% NEQ 0 (
    echo ✗ Clippy failed!
    exit /b %ERRORLEVEL%
)

echo.
echo 3/3 Running tests...
cargo test --all
if %ERRORLEVEL% NEQ 0 (
    echo ✗ Tests failed!
    exit /b %ERRORLEVEL%
)

echo.
echo ======================================
echo ✓ All checks passed!
echo ======================================
