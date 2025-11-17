//! Aurora Compass Branding
//!
//! This module contains all branding constants for Aurora Compass.
//! The application maintains the same look and feel as the original Bluesky client
//! with custom branding elements (logo, name, colors).

/// Application name
pub const APP_NAME: &str = "Aurora Compass";

/// Application name short form
pub const APP_NAME_SHORT: &str = "Aurora";

/// Application tagline
pub const APP_TAGLINE: &str = "Navigate the ATmosphere";

/// Application version (from Cargo.toml)
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Logo file path (relative to repository root)
pub const LOGO_PATH: &str = "logo.png";

/// Brand colors extracted from logo
pub mod colors {
    /// Primary brand color (Navy blue from compass)
    pub const PRIMARY: &str = "#1E3A5F";

    /// Secondary brand color (Purple from aurora)
    pub const SECONDARY_PURPLE: &str = "#9D4EDD";

    /// Secondary brand color (Blue from aurora)
    pub const SECONDARY_BLUE: &str = "#3A86FF";

    /// Secondary brand color (Cyan from aurora)
    pub const SECONDARY_CYAN: &str = "#06FFA5";

    /// Accent color (Gold from compass detail)
    pub const ACCENT_GOLD: &str = "#FFB703";

    /// Background color (light theme)
    pub const BACKGROUND_LIGHT: &str = "#FFFFFF";

    /// Background color (dark theme)
    pub const BACKGROUND_DARK: &str = "#0A0F1A";

    /// Background color (dim theme)
    pub const BACKGROUND_DIM: &str = "#1A2332";
}

/// Social media handles (placeholder - update as needed)
pub mod social {
    /// Official website
    pub const WEBSITE: &str = "https://aurora-compass.app";

    /// GitHub repository
    pub const GITHUB: &str = "https://github.com/yourusername/aurora-compass";

    /// Support email
    pub const SUPPORT_EMAIL: &str = "support@aurora-compass.app";
}

/// Copyright information
pub mod copyright {
    /// Copyright year
    pub const YEAR: &str = "2024-2025";

    /// Copyright holder
    pub const HOLDER: &str = "Aurora Compass Team";

    /// License
    pub const LICENSE: &str = "MIT";

    /// Full copyright notice
    pub fn notice() -> String {
        format!("© {} {}. Licensed under {}.", YEAR, HOLDER, LICENSE)
    }
}

/// About information for the app
pub mod about {
    use super::*;

    /// Full about text
    pub fn text() -> String {
        format!(
            "{} v{}\n\n{}\n\nA fork of Bluesky maintaining the same great UX with added freedom:\n\
            • Optional moderation services (not forced)\n\
            • Custom AppView configuration per user\n\
            • Same beautiful design and features\n\n\
            Built with Rust and ❤️\n\n{}",
            APP_NAME,
            APP_VERSION,
            APP_TAGLINE,
            copyright::notice()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_name() {
        assert_eq!(APP_NAME, "Aurora Compass");
        assert_eq!(APP_NAME_SHORT, "Aurora");
    }

    #[test]
    fn test_app_version() {
        assert!(!APP_VERSION.is_empty());
        // Version should follow semver format
        let parts: Vec<&str> = APP_VERSION.split('.').collect();
        assert!(parts.len() >= 2, "Version should have at least major.minor");
    }

    #[test]
    fn test_brand_colors() {
        // Verify all colors are valid hex codes
        let color_list = [
            colors::PRIMARY,
            colors::SECONDARY_PURPLE,
            colors::SECONDARY_BLUE,
            colors::SECONDARY_CYAN,
            colors::ACCENT_GOLD,
            colors::BACKGROUND_LIGHT,
            colors::BACKGROUND_DARK,
            colors::BACKGROUND_DIM,
        ];

        for color in &color_list {
            assert!(color.starts_with('#'), "Color should start with #: {}", color);
            assert!(
                color.len() == 7,
                "Color should be 7 characters (#RRGGBB): {}",
                color
            );
        }
    }

    #[test]
    fn test_copyright_notice() {
        let notice = copyright::notice();
        assert!(notice.contains("Aurora Compass Team"));
        assert!(notice.contains("MIT"));
    }

    #[test]
    fn test_about_text() {
        let text = about::text();
        assert!(text.contains("Aurora Compass"));
        assert!(text.contains("Optional moderation"));
        assert!(text.contains("Custom AppView"));
    }

    #[test]
    fn test_social_urls() {
        // Verify URLs are well-formed
        assert!(social::WEBSITE.starts_with("https://"));
        assert!(social::GITHUB.starts_with("https://github.com/"));
        assert!(social::SUPPORT_EMAIL.contains('@'));
    }
}
