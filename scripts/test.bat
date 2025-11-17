@echo off
REM Run all tests

echo Running tests...
cargo test --all

if %ERRORLEVEL% EQU 0 (
    echo ✓ All tests passed!
) else (
    echo ✗ Tests failed!
    exit /b %ERRORLEVEL%
)
