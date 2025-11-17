#!/usr/bin/env bash
# Run all tests

set -e

echo "Running tests..."
cargo test --all

echo "✓ All tests passed!"
