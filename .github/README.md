# GitHub Configuration

This directory contains GitHub-specific configuration files for Aurora Compass, including CI/CD workflows and Dependabot configuration.

## Workflows

### CI Workflow ([ci.yml](workflows/ci.yml))

**Trigger**: Push to `main` or `develop` branches, or pull requests to these branches

**Jobs**:
- **Format Check**: Verifies code formatting with `rustfmt`
- **Clippy**: Runs Rust linter with strict rules (`-D warnings`)
- **Test Suite**: Runs all tests on multiple platforms (Linux, macOS, Windows) and Rust versions (stable, nightly)
- **Build Check**: Verifies the project builds successfully in both debug and release modes
- **Security Audit**: Checks for known security vulnerabilities in dependencies using `cargo-audit`

**Caching**: Aggressive caching of cargo registry, git index, and target directories for faster builds

### Release Workflow ([release.yml](workflows/release.yml))

**Trigger**: Push of version tags (e.g., `v1.0.0`)

**Jobs**:
- **Create Release**: Creates a GitHub release for the tag
- **Build Release**: Builds release binaries for multiple platforms:
  - Linux (x86_64 gnu and musl)
  - macOS (x86_64 and ARM64)
  - Windows (x86_64)
- **Publish to crates.io**: Optionally publishes to crates.io (requires `CARGO_REGISTRY_TOKEN` secret)

**Artifacts**: Pre-built binaries for each platform are attached to the GitHub release

### Coverage Workflow ([coverage.yml](workflows/coverage.yml))

**Trigger**: Push to `main` or `develop` branches, or pull requests to these branches

**Jobs**:
- **Code Coverage**: Generates code coverage report using `cargo-tarpaulin`
- Uploads coverage to Codecov (requires `CODECOV_TOKEN` secret)
- Archives coverage report as workflow artifact

### Documentation Workflow ([docs.yml](workflows/docs.yml))

**Trigger**: Push to `main` branch, or manual dispatch

**Jobs**:
- **Build and Deploy Documentation**:
  - Builds Rust documentation with `cargo doc`
  - Deploys to GitHub Pages
  - Includes private items for comprehensive internal docs

**Note**: Requires GitHub Pages to be enabled in repository settings

## Dependabot Configuration ([dependabot.yml](dependabot.yml))

Automated dependency updates for:
- **Cargo dependencies**: Weekly updates on Mondays at 9:00 AM
  - Groups development dependencies together
  - Groups production minor/patch updates together
  - Maximum 10 open PRs
- **GitHub Actions**: Weekly updates on Mondays at 9:00 AM
  - Maximum 5 open PRs

All Dependabot PRs are automatically labeled and assigned for review.

## Required Secrets

For full functionality, configure these secrets in repository settings:

- `CARGO_REGISTRY_TOKEN`: For publishing to crates.io (optional)
- `CODECOV_TOKEN`: For uploading coverage reports to Codecov (optional)
- `GITHUB_TOKEN`: Automatically provided by GitHub Actions

## Branch Protection

Recommended branch protection rules for `main`:

- Require status checks to pass before merging:
  - Format Check
  - Clippy
  - Test Suite (all matrix combinations)
  - Build Check
- Require branches to be up to date before merging
- Require pull request reviews (at least 1)
- Dismiss stale pull request approvals when new commits are pushed

## Performance Optimizations

All workflows use caching to improve performance:
- Cargo registry cache
- Cargo git index cache
- Target directory cache (per job)

This reduces CI times significantly after the first run.

## Local Testing

Before pushing, you can run the same checks locally:

```bash
# Format check
cargo fmt --all -- --check

# Clippy check
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test --all

# Build
cargo build --all --release

# Security audit
cargo install cargo-audit
cargo audit
```

Or use the development scripts:

```bash
# Unix
./scripts/check.sh

# Windows
scripts\check.bat
```
