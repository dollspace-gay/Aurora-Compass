# Aurora Compass Branding Guide

## Overview

Aurora Compass is a fork of Bluesky that maintains the same look and feel while providing:
- Custom branding (logo, name, colors)
- Optional moderation services (not forced like official client)
- Per-user custom AppView configuration

## Design Philosophy

**IMPORTANT**: Aurora Compass uses the exact same UI/UX as the original Bluesky client. We are NOT redesigning the interface. The only changes are:
1. Logo and app name
2. Brand colors (used sparingly to maintain Bluesky's design language)
3. Key feature differentiators (optional moderation, custom AppView)

## Logo

The Aurora Compass logo is located at: `logo.png`

![Aurora Compass Logo](logo.png)

### Logo Usage
- **App Icon**: Use for application icon on all platforms
- **Splash Screen**: Display during app startup
- **About Page**: Show in about/settings screen
- **Navigation**: Can be used as home button (following Bluesky's pattern)

### Logo Colors
The logo features:
- Navy blue compass (#1E3A5F)
- Purple/blue/cyan aurora gradient (#9D4EDD, #3A86FF, #06FFA5)
- Gold accent (#FFB703)

## Application Name

- **Full Name**: Aurora Compass
- **Short Name**: Aurora
- **Tagline**: Navigate the ATmosphere

## Brand Colors

### Primary Colors
```rust
use app_core::branding::colors;

colors::PRIMARY            // #1E3A5F (Navy blue - compass)
colors::SECONDARY_PURPLE   // #9D4EDD (Aurora purple)
colors::SECONDARY_BLUE     // #3A86FF (Aurora blue)
colors::SECONDARY_CYAN     // #06FFA5 (Aurora cyan)
colors::ACCENT_GOLD        // #FFB703 (Compass gold)
```

### Theme Colors
Aurora Compass supports the same three themes as Bluesky:
- **Light Theme**: Uses Bluesky's light theme with subtle Aurora branding
- **Dark Theme**: Uses Bluesky's dark theme with subtle Aurora branding
- **Dim Theme**: Uses Bluesky's dim theme with subtle Aurora branding

## Using Branding in Code

### Rust Code

```rust
use app_core::branding::{APP_NAME, APP_VERSION, colors, about};

// Application name
println!("Welcome to {}", APP_NAME);

// Version
println!("Version: {}", APP_VERSION);

// About text
let about_text = about::text();

// Colors
let primary_color = colors::PRIMARY;
```

### Tests

The branding module includes comprehensive tests:

```bash
cd crates/app-core
cargo test branding
```

## Key Differentiators

### 1. Optional Moderation
Unlike the official Bluesky client, Aurora Compass does NOT force subscription to Bluesky's moderation service. Users can:
- Choose which labeling services to subscribe to
- Opt out of all moderation services
- Add custom moderation services

### 2. Custom AppView
Users can configure custom AppView endpoints per account:
- Default: `https://bsky.social`
- Configurable in Settings → Advanced → AppView URL
- Each account can use a different AppView

## Reference Implementation

The original Bluesky client implementation is preserved in:
```
original-bluesky/
```

Always reference this directory when implementing UI components to maintain design consistency.

## Copyright

© 2024-2025 Aurora Compass Team. Licensed under MIT.

Aurora Compass is not affiliated with Bluesky Social PBC. It is an independent fork of the Bluesky client.

## Contact

- **Website**: https://aurora-compass.app
- **GitHub**: https://github.com/yourusername/aurora-compass
- **Support**: support@aurora-compass.app
