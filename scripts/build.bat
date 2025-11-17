@echo off
REM Build all crates

echo Building Aurora Compass...
cargo build --all

if %ERRORLEVEL% EQU 0 (
    echo ✓ Build complete!
) else (
    echo ✗ Build failed!
    exit /b %ERRORLEVEL%
)
