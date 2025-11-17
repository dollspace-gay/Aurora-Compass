@echo off
REM Format all Rust code using rustfmt

echo Formatting Rust code...
cargo fmt --all

if %ERRORLEVEL% EQU 0 (
    echo ✓ Formatting complete!
) else (
    echo ✗ Formatting failed!
    exit /b %ERRORLEVEL%
)
