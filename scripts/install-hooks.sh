#!/usr/bin/env bash
# Install git hooks

set -e

HOOKS_DIR=".git/hooks"
PRE_COMMIT_HOOK="$HOOKS_DIR/pre-commit"

# Check if we're in a git repository
if [ ! -d ".git" ]; then
    echo "Error: Not in a git repository!"
    exit 1
fi

echo "Installing pre-commit hook..."

# Create the pre-commit hook
cat > "$PRE_COMMIT_HOOK" << 'EOF'
#!/usr/bin/env bash
# Pre-commit hook for Aurora Compass
# Runs formatting check, clippy, and tests

set -e

echo "Running pre-commit checks..."

# Check formatting
echo "1/3 Checking code formatting..."
if ! cargo fmt --all -- --check; then
    echo "Error: Code is not formatted. Run 'cargo fmt --all' to fix."
    exit 1
fi

# Run clippy
echo "2/3 Running clippy..."
if ! cargo clippy --all-targets --all-features -- -D warnings; then
    echo "Error: Clippy found issues. Fix them before committing."
    exit 1
fi

# Run tests
echo "3/3 Running tests..."
if ! cargo test --all --quiet; then
    echo "Error: Tests failed. Fix them before committing."
    exit 1
fi

echo "✓ Pre-commit checks passed!"
EOF

# Make the hook executable
chmod +x "$PRE_COMMIT_HOOK"

echo "✓ Pre-commit hook installed successfully!"
echo ""
echo "The hook will run automatically before each commit."
echo "To skip the hook (not recommended), use: git commit --no-verify"
