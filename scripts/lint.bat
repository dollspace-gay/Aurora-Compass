@echo off
REM Run clippy on all code

echo Running clippy...
cargo clippy --all-targets --all-features -- -D warnings

if %ERRORLEVEL% EQU 0 (
    echo ✓ Linting complete!
) else (
    echo ✗ Linting failed!
    exit /b %ERRORLEVEL%
)
