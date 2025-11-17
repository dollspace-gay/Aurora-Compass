#!/usr/bin/env bash
# Run clippy on all code

set -e

echo "Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings

echo "✓ Linting complete!"
