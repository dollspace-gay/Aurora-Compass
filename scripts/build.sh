#!/usr/bin/env bash
# Build all crates

set -e

echo "Building Aurora Compass..."
cargo build --all

echo "✓ Build complete!"
