# Aurora Compass

A Rust-based Bluesky client fork with optional moderation and custom AppView support.

**See [BRANDING.md](BRANDING.md) for branding guidelines and design philosophy.**

## Quick Start (Development)

```bash
# 1. Build the Rust workspace
cargo build

# 2. Run tests
cargo test

# 3. Run a specific crate
cargo run -p app-core
```

## Project Structure

```
crates/
├── atproto-client/     # AT Protocol SDK
├── app-core/           # Business logic & branding
├── app-state/          # State management
├── app-ui/             # User interface
├── app-platform/       # Platform-specific code
├── media-processing/   # Image/video processing
├── moderation/         # Content filtering
├── networking/         # HTTP client
├── storage/            # Database & cache
└── i18n/               # Internationalization

original-bluesky/       # Original TypeScript client (for reference)
```

## Key Features

### What Makes Aurora Compass Different?

1. **Optional Moderation**
   - Unlike official Bluesky, labeler subscription is **optional**
   - Users choose which moderation services to use
   - Full control over content filtering

2. **Custom AppView Per User**
   - Each account can specify their own AppView endpoint
   - Not limited to bsky.social
   - Configure in Settings → Advanced → AppView URL

3. **Same Great UX**
   - Maintains the exact UI/UX of the original Bluesky client
   - Reference implementation in `original-bluesky/`
   - Custom branding only (logo, name, colors)

## Development

### Issue Tracking

We use [bd](https://github.com/josephg/bd) for issue tracking:

```bash
# View ready work
bd ready

# View all issues
bd list

# View issue details
bd show Aurora-Compass-xxx

# Update issue status
bd update Aurora-Compass-xxx --status in_progress
```

Current status: **123 issues** (122 open, 1 closed, 91 ready to work)

### Development Guidelines

See [CLAUDE.md](CLAUDE.md) for comprehensive development guidelines:
- No stubs or `unimplemented!()` macros
- Complete implementations only
- Comprehensive testing required
- Follow Rust best practices

### Branding

See [BRANDING.md](BRANDING.md) for:
- Logo usage guidelines
- Brand colors
- Application naming
- Design philosophy

## License

MIT License - See [LICENSE](LICENSE)
