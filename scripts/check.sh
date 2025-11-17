#!/usr/bin/env bash
# Run all checks: format, lint, and test

set -e

echo "======================================"
echo "Running all checks..."
echo "======================================"

echo ""
echo "1/3 Checking formatting..."
cargo fmt --all -- --check

echo ""
echo "2/3 Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings

echo ""
echo "3/3 Running tests..."
cargo test --all

echo ""
echo "======================================"
echo "✓ All checks passed!"
echo "======================================"
