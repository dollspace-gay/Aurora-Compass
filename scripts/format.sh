#!/usr/bin/env bash
# Format all Rust code using rustfmt

set -e

echo "Formatting Rust code..."
cargo fmt --all

echo "✓ Formatting complete!"
