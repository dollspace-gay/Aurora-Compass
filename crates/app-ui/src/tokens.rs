//! Design tokens for Aurora Compass
//!
//! This module provides design tokens for spacing, sizing, typography,
//! breakpoints, and other design system primitives.

use serde::{Deserialize, Serialize};

// =============================================================================
// Spacing Tokens
// =============================================================================

/// Spacing scale in pixels
/// Based on a 4px base unit with t-shirt sizes
pub mod spacing {
    /// 2px - Extra extra small
    pub const SPACE_2XS: f32 = 2.0;
    /// 4px - Extra small
    pub const SPACE_XS: f32 = 4.0;
    /// 8px - Small
    pub const SPACE_SM: f32 = 8.0;
    /// 12px - Medium
    pub const SPACE_MD: f32 = 12.0;
    /// 16px - Large
    pub const SPACE_LG: f32 = 16.0;
    /// 20px - Extra large
    pub const SPACE_XL: f32 = 20.0;
    /// 24px - 2x large
    pub const SPACE_2XL: f32 = 24.0;
    /// 32px - 3x large
    pub const SPACE_3XL: f32 = 32.0;
    /// 40px - 4x large
    pub const SPACE_4XL: f32 = 40.0;
    /// 48px - 5x large
    pub const SPACE_5XL: f32 = 48.0;

    /// Get spacing value by name
    pub fn get(name: &str) -> Option<f32> {
        match name {
            "2xs" => Some(SPACE_2XS),
            "xs" => Some(SPACE_XS),
            "sm" => Some(SPACE_SM),
            "md" => Some(SPACE_MD),
            "lg" => Some(SPACE_LG),
            "xl" => Some(SPACE_XL),
            "2xl" => Some(SPACE_2XL),
            "3xl" => Some(SPACE_3XL),
            "4xl" => Some(SPACE_4XL),
            "5xl" => Some(SPACE_5XL),
            _ => None,
        }
    }
}

// =============================================================================
// Sizing Tokens
// =============================================================================

/// Size tokens for component dimensions
pub mod sizing {
    /// Icon sizes
    pub mod icon {
        /// Extra small icon (12px)
        pub const XS: f32 = 12.0;
        /// Small icon (16px)
        pub const SM: f32 = 16.0;
        /// Medium icon (20px)
        pub const MD: f32 = 20.0;
        /// Large icon (24px)
        pub const LG: f32 = 24.0;
        /// Extra large icon (32px)
        pub const XL: f32 = 32.0;
        /// 2x large icon (40px)
        pub const XXL: f32 = 40.0;
    }

    /// Avatar sizes
    pub mod avatar {
        /// Extra small avatar (24px)
        pub const XS: f32 = 24.0;
        /// Small avatar (32px)
        pub const SM: f32 = 32.0;
        /// Medium avatar (48px)
        pub const MD: f32 = 48.0;
        /// Large avatar (64px)
        pub const LG: f32 = 64.0;
        /// Extra large avatar (90px)
        pub const XL: f32 = 90.0;
        /// Profile avatar (120px)
        pub const PROFILE: f32 = 120.0;
    }

    /// Button sizes
    pub mod button {
        /// Small button height (32px)
        pub const SM_HEIGHT: f32 = 32.0;
        /// Medium button height (40px)
        pub const MD_HEIGHT: f32 = 40.0;
        /// Large button height (48px)
        pub const LG_HEIGHT: f32 = 48.0;
        /// Small button padding x (12px)
        pub const SM_PADDING_X: f32 = 12.0;
        /// Medium button padding x (16px)
        pub const MD_PADDING_X: f32 = 16.0;
        /// Large button padding x (24px)
        pub const LG_PADDING_X: f32 = 24.0;
    }

    /// Input field sizes
    pub mod input {
        /// Small input height (36px)
        pub const SM_HEIGHT: f32 = 36.0;
        /// Medium input height (44px)
        pub const MD_HEIGHT: f32 = 44.0;
        /// Large input height (52px)
        pub const LG_HEIGHT: f32 = 52.0;
    }
}

// =============================================================================
// Border Radius Tokens
// =============================================================================

/// Border radius tokens
pub mod radius {
    /// No radius (0px)
    pub const NONE: f32 = 0.0;
    /// Small radius (4px)
    pub const SM: f32 = 4.0;
    /// Medium radius (8px)
    pub const MD: f32 = 8.0;
    /// Large radius (12px)
    pub const LG: f32 = 12.0;
    /// Extra large radius (16px)
    pub const XL: f32 = 16.0;
    /// 2x large radius (24px)
    pub const XXL: f32 = 24.0;
    /// Full/round radius (9999px)
    pub const FULL: f32 = 9999.0;
}

// =============================================================================
// Border Width Tokens
// =============================================================================

/// Border width tokens
pub mod border {
    /// No border (0px)
    pub const NONE: f32 = 0.0;
    /// Hairline border (0.5px)
    pub const HAIRLINE: f32 = 0.5;
    /// Thin border (1px)
    pub const THIN: f32 = 1.0;
    /// Medium border (2px)
    pub const MEDIUM: f32 = 2.0;
    /// Thick border (3px)
    pub const THICK: f32 = 3.0;
}

// =============================================================================
// Shadow Tokens
// =============================================================================

/// Shadow definitions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shadow {
    /// Horizontal offset
    pub offset_x: f32,
    /// Vertical offset
    pub offset_y: f32,
    /// Blur radius
    pub blur: f32,
    /// Spread radius
    pub spread: f32,
    /// Shadow color (with alpha)
    pub color: String,
}

impl Shadow {
    /// Create a new shadow
    pub fn new(offset_x: f32, offset_y: f32, blur: f32, spread: f32, color: &str) -> Self {
        Self {
            offset_x,
            offset_y,
            blur,
            spread,
            color: color.to_string(),
        }
    }
}

/// Shadow presets
pub mod shadows {
    use super::Shadow;

    /// No shadow
    pub fn none() -> Shadow {
        Shadow::new(0.0, 0.0, 0.0, 0.0, "transparent")
    }

    /// Extra small shadow
    pub fn xs() -> Shadow {
        Shadow::new(0.0, 1.0, 2.0, 0.0, "rgba(0, 0, 0, 0.05)")
    }

    /// Small shadow
    pub fn sm() -> Shadow {
        Shadow::new(0.0, 1.0, 3.0, 0.0, "rgba(0, 0, 0, 0.1)")
    }

    /// Medium shadow
    pub fn md() -> Shadow {
        Shadow::new(0.0, 4.0, 6.0, -1.0, "rgba(0, 0, 0, 0.1)")
    }

    /// Large shadow
    pub fn lg() -> Shadow {
        Shadow::new(0.0, 10.0, 15.0, -3.0, "rgba(0, 0, 0, 0.1)")
    }

    /// Extra large shadow
    pub fn xl() -> Shadow {
        Shadow::new(0.0, 20.0, 25.0, -5.0, "rgba(0, 0, 0, 0.1)")
    }

    /// 2x large shadow
    pub fn xxl() -> Shadow {
        Shadow::new(0.0, 25.0, 50.0, -12.0, "rgba(0, 0, 0, 0.25)")
    }
}

// =============================================================================
// Breakpoint Tokens
// =============================================================================

/// Breakpoint configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Breakpoint {
    /// Phone (< 500px)
    Phone,
    /// Greater than phone (>= 500px)
    GtPhone,
    /// Greater than mobile (>= 800px)
    GtMobile,
    /// Greater than tablet (>= 1300px)
    GtTablet,
}

/// Breakpoint widths
pub mod breakpoints {
    /// Phone breakpoint (500px)
    pub const PHONE: u32 = 500;
    /// Mobile breakpoint (800px)
    pub const MOBILE: u32 = 800;
    /// Tablet breakpoint (1300px)
    pub const TABLET: u32 = 1300;

    /// Right nav visible threshold
    pub const RIGHT_NAV_VISIBLE: u32 = 1100;
    /// Center column offset threshold
    pub const CENTER_COLUMN_OFFSET_MIN: u32 = 1100;
    /// Center column offset max threshold
    pub const CENTER_COLUMN_OFFSET_MAX: u32 = 1300;
    /// Left nav minimal threshold
    pub const LEFT_NAV_MINIMAL: u32 = 1300;

    /// Check if width is greater than phone
    pub fn is_gt_phone(width: u32) -> bool {
        width >= PHONE
    }

    /// Check if width is greater than mobile
    pub fn is_gt_mobile(width: u32) -> bool {
        width >= MOBILE
    }

    /// Check if width is greater than tablet
    pub fn is_gt_tablet(width: u32) -> bool {
        width >= TABLET
    }

    /// Get current breakpoint
    pub fn current(width: u32) -> super::Breakpoint {
        if width >= TABLET {
            super::Breakpoint::GtTablet
        } else if width >= MOBILE {
            super::Breakpoint::GtMobile
        } else if width >= PHONE {
            super::Breakpoint::GtPhone
        } else {
            super::Breakpoint::Phone
        }
    }
}

// =============================================================================
// Animation Tokens
// =============================================================================

/// Animation duration tokens (in milliseconds)
pub mod duration {
    /// Instant (0ms)
    pub const INSTANT: u32 = 0;
    /// Extra fast (50ms)
    pub const EXTRA_FAST: u32 = 50;
    /// Fast (100ms)
    pub const FAST: u32 = 100;
    /// Normal (150ms)
    pub const NORMAL: u32 = 150;
    /// Moderate (200ms)
    pub const MODERATE: u32 = 200;
    /// Slow (300ms)
    pub const SLOW: u32 = 300;
    /// Extra slow (500ms)
    pub const EXTRA_SLOW: u32 = 500;
}

/// Easing functions
pub mod easing {
    /// Default easing curve (cubic-bezier)
    pub const DEFAULT: &str = "cubic-bezier(0.17, 0.73, 0.14, 1)";
    /// Linear
    pub const LINEAR: &str = "linear";
    /// Ease in
    pub const EASE_IN: &str = "cubic-bezier(0.4, 0, 1, 1)";
    /// Ease out
    pub const EASE_OUT: &str = "cubic-bezier(0, 0, 0.2, 1)";
    /// Ease in out
    pub const EASE_IN_OUT: &str = "cubic-bezier(0.4, 0, 0.2, 1)";
    /// Bounce
    pub const BOUNCE: &str = "cubic-bezier(0.34, 1.56, 0.64, 1)";
}

// =============================================================================
// Z-Index Tokens
// =============================================================================

/// Z-index layers
pub mod z_index {
    /// Default layer
    pub const DEFAULT: i32 = 0;
    /// Dropdown/select
    pub const DROPDOWN: i32 = 10;
    /// Sticky elements
    pub const STICKY: i32 = 20;
    /// Fixed elements (headers)
    pub const FIXED: i32 = 30;
    /// Modal backdrop
    pub const MODAL_BACKDROP: i32 = 40;
    /// Modal content
    pub const MODAL: i32 = 50;
    /// Popover
    pub const POPOVER: i32 = 60;
    /// Tooltip
    pub const TOOLTIP: i32 = 70;
    /// Toast notifications
    pub const TOAST: i32 = 80;
    /// Maximum (loading overlays, etc.)
    pub const MAX: i32 = 9999;
}

// =============================================================================
// Content Widths
// =============================================================================

/// Content width constraints
pub mod content_width {
    /// Narrow content (480px)
    pub const NARROW: f32 = 480.0;
    /// Normal content (600px)
    pub const NORMAL: f32 = 600.0;
    /// Wide content (800px)
    pub const WIDE: f32 = 800.0;
    /// Extra wide content (1000px)
    pub const EXTRA_WIDE: f32 = 1000.0;
    /// Full width (no constraint)
    pub const FULL: f32 = f32::MAX;
}

// =============================================================================
// Aspect Ratios
// =============================================================================

/// Common aspect ratios
pub mod aspect_ratio {
    /// Square (1:1)
    pub const SQUARE: f32 = 1.0;
    /// Profile card (1.5:1)
    pub const CARD: f32 = 1.5;
    /// Video/landscape (16:9)
    pub const VIDEO: f32 = 16.0 / 9.0;
    /// Portrait (3:4)
    pub const PORTRAIT: f32 = 3.0 / 4.0;
    /// Banner (3:1)
    pub const BANNER: f32 = 3.0;
}

// =============================================================================
// Scrollbar
// =============================================================================

/// Scrollbar dimensions
pub mod scrollbar {
    /// Scrollbar width (web)
    pub const WIDTH: f32 = 8.0;
    /// Scrollbar offset for layout
    pub const OFFSET: f32 = 8.0;
}

// =============================================================================
// Hit Target
// =============================================================================

/// Minimum touch/click target sizes (for accessibility)
pub mod hit_target {
    /// Minimum touch target (44px - iOS guideline)
    pub const MIN: f32 = 44.0;
    /// Comfortable touch target (48px - Material guideline)
    pub const COMFORTABLE: f32 = 48.0;
}

// =============================================================================
// Typography Tokens (moved from typography.rs for organization)
// =============================================================================

/// Letter spacing (tracking) in em units
pub mod tracking {
    /// Default letter spacing
    pub const DEFAULT: f32 = 0.0;
    /// Tight letter spacing (-0.025em)
    pub const TIGHT: f32 = -0.025;
    /// Wide letter spacing (0.025em)
    pub const WIDE: f32 = 0.025;
}

/// Line height multipliers
pub mod line_height {
    /// None (1.0)
    pub const NONE: f32 = 1.0;
    /// Tight (1.25)
    pub const TIGHT: f32 = 1.25;
    /// Snug (1.375)
    pub const SNUG: f32 = 1.375;
    /// Normal (1.5)
    pub const NORMAL: f32 = 1.5;
    /// Relaxed (1.625)
    pub const RELAXED: f32 = 1.625;
    /// Loose (2.0)
    pub const LOOSE: f32 = 2.0;
}

/// Font weight values
pub mod font_weight {
    /// Normal/Regular (400)
    pub const NORMAL: u16 = 400;
    /// Medium (500)
    pub const MEDIUM: u16 = 500;
    /// Semi-bold (600)
    pub const SEMI_BOLD: u16 = 600;
    /// Bold (700)
    pub const BOLD: u16 = 700;
    /// Heavy/Black (800)
    pub const HEAVY: u16 = 800;
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Spacing Tests
    // ==========================================================================

    #[test]
    fn test_spacing_values() {
        assert_eq!(spacing::SPACE_2XS, 2.0);
        assert_eq!(spacing::SPACE_XS, 4.0);
        assert_eq!(spacing::SPACE_SM, 8.0);
        assert_eq!(spacing::SPACE_MD, 12.0);
        assert_eq!(spacing::SPACE_LG, 16.0);
        assert_eq!(spacing::SPACE_XL, 20.0);
        assert_eq!(spacing::SPACE_2XL, 24.0);
        assert_eq!(spacing::SPACE_3XL, 32.0);
        assert_eq!(spacing::SPACE_4XL, 40.0);
        assert_eq!(spacing::SPACE_5XL, 48.0);
    }

    #[test]
    fn test_spacing_get() {
        assert_eq!(spacing::get("xs"), Some(4.0));
        assert_eq!(spacing::get("md"), Some(12.0));
        assert_eq!(spacing::get("invalid"), None);
    }

    // ==========================================================================
    // Sizing Tests
    // ==========================================================================

    #[test]
    fn test_icon_sizes() {
        assert!(sizing::icon::XS < sizing::icon::SM);
        assert!(sizing::icon::SM < sizing::icon::MD);
        assert!(sizing::icon::MD < sizing::icon::LG);
        assert!(sizing::icon::LG < sizing::icon::XL);
    }

    #[test]
    fn test_avatar_sizes() {
        assert!(sizing::avatar::XS < sizing::avatar::SM);
        assert!(sizing::avatar::SM < sizing::avatar::MD);
        assert!(sizing::avatar::MD < sizing::avatar::LG);
        assert!(sizing::avatar::LG < sizing::avatar::XL);
        assert!(sizing::avatar::XL < sizing::avatar::PROFILE);
    }

    #[test]
    fn test_button_sizes() {
        assert!(sizing::button::SM_HEIGHT < sizing::button::MD_HEIGHT);
        assert!(sizing::button::MD_HEIGHT < sizing::button::LG_HEIGHT);
    }

    // ==========================================================================
    // Border Radius Tests
    // ==========================================================================

    #[test]
    fn test_radius_scale() {
        assert_eq!(radius::NONE, 0.0);
        assert!(radius::SM < radius::MD);
        assert!(radius::MD < radius::LG);
        assert!(radius::LG < radius::XL);
        assert!(radius::XL < radius::XXL);
        assert!(radius::FULL > 1000.0);
    }

    // ==========================================================================
    // Shadow Tests
    // ==========================================================================

    #[test]
    fn test_shadow_new() {
        let shadow = Shadow::new(0.0, 4.0, 6.0, -1.0, "rgba(0,0,0,0.1)");
        assert_eq!(shadow.offset_y, 4.0);
        assert_eq!(shadow.blur, 6.0);
    }

    #[test]
    fn test_shadow_presets() {
        let none = shadows::none();
        assert_eq!(none.blur, 0.0);

        let sm = shadows::sm();
        let md = shadows::md();
        let lg = shadows::lg();

        assert!(sm.blur < md.blur);
        assert!(md.blur < lg.blur);
    }

    // ==========================================================================
    // Breakpoint Tests
    // ==========================================================================

    #[test]
    fn test_breakpoint_thresholds() {
        assert!(breakpoints::PHONE < breakpoints::MOBILE);
        assert!(breakpoints::MOBILE < breakpoints::TABLET);
    }

    #[test]
    fn test_breakpoint_checks() {
        assert!(!breakpoints::is_gt_phone(400));
        assert!(breakpoints::is_gt_phone(500));
        assert!(breakpoints::is_gt_phone(600));

        assert!(!breakpoints::is_gt_mobile(700));
        assert!(breakpoints::is_gt_mobile(800));

        assert!(!breakpoints::is_gt_tablet(1200));
        assert!(breakpoints::is_gt_tablet(1300));
    }

    #[test]
    fn test_breakpoint_current() {
        assert_eq!(breakpoints::current(400), Breakpoint::Phone);
        assert_eq!(breakpoints::current(600), Breakpoint::GtPhone);
        assert_eq!(breakpoints::current(900), Breakpoint::GtMobile);
        assert_eq!(breakpoints::current(1500), Breakpoint::GtTablet);
    }

    // ==========================================================================
    // Animation Tests
    // ==========================================================================

    #[test]
    fn test_duration_scale() {
        assert!(duration::INSTANT < duration::EXTRA_FAST);
        assert!(duration::EXTRA_FAST < duration::FAST);
        assert!(duration::FAST < duration::NORMAL);
        assert!(duration::NORMAL < duration::MODERATE);
        assert!(duration::MODERATE < duration::SLOW);
        assert!(duration::SLOW < duration::EXTRA_SLOW);
    }

    #[test]
    fn test_easing_valid_css() {
        // Just verify they're valid CSS-like strings
        assert!(easing::DEFAULT.contains("cubic-bezier"));
        assert!(easing::LINEAR.contains("linear"));
        assert!(easing::EASE_IN.contains("cubic-bezier"));
    }

    // ==========================================================================
    // Z-Index Tests
    // ==========================================================================

    #[test]
    fn test_z_index_ordering() {
        assert!(z_index::DEFAULT < z_index::DROPDOWN);
        assert!(z_index::DROPDOWN < z_index::STICKY);
        assert!(z_index::STICKY < z_index::FIXED);
        assert!(z_index::FIXED < z_index::MODAL_BACKDROP);
        assert!(z_index::MODAL_BACKDROP < z_index::MODAL);
        assert!(z_index::MODAL < z_index::POPOVER);
        assert!(z_index::POPOVER < z_index::TOOLTIP);
        assert!(z_index::TOOLTIP < z_index::TOAST);
        assert!(z_index::TOAST < z_index::MAX);
    }

    // ==========================================================================
    // Content Width Tests
    // ==========================================================================

    #[test]
    fn test_content_width_scale() {
        assert!(content_width::NARROW < content_width::NORMAL);
        assert!(content_width::NORMAL < content_width::WIDE);
        assert!(content_width::WIDE < content_width::EXTRA_WIDE);
    }

    // ==========================================================================
    // Aspect Ratio Tests
    // ==========================================================================

    #[test]
    fn test_aspect_ratios() {
        assert_eq!(aspect_ratio::SQUARE, 1.0);
        assert!(aspect_ratio::VIDEO > 1.0); // Landscape
        assert!(aspect_ratio::PORTRAIT < 1.0); // Portrait
        assert!(aspect_ratio::BANNER > aspect_ratio::VIDEO); // Very wide
    }

    // ==========================================================================
    // Hit Target Tests
    // ==========================================================================

    #[test]
    fn test_hit_targets() {
        // Minimum should be at least 44px (iOS guideline)
        assert!(hit_target::MIN >= 44.0);
        assert!(hit_target::COMFORTABLE >= hit_target::MIN);
    }

    // ==========================================================================
    // Typography Token Tests
    // ==========================================================================

    #[test]
    fn test_font_weights() {
        assert_eq!(font_weight::NORMAL, 400);
        assert!(font_weight::MEDIUM > font_weight::NORMAL);
        assert!(font_weight::SEMI_BOLD > font_weight::MEDIUM);
        assert!(font_weight::BOLD > font_weight::SEMI_BOLD);
        assert!(font_weight::HEAVY > font_weight::BOLD);
    }

    #[test]
    fn test_line_heights() {
        assert_eq!(line_height::NONE, 1.0);
        assert!(line_height::TIGHT > line_height::NONE);
        assert!(line_height::SNUG > line_height::TIGHT);
        assert!(line_height::NORMAL > line_height::SNUG);
        assert!(line_height::RELAXED > line_height::NORMAL);
        assert!(line_height::LOOSE > line_height::RELAXED);
    }

    // ==========================================================================
    // Serialization Tests
    // ==========================================================================

    #[test]
    fn test_shadow_serialization() {
        let shadow = shadows::md();
        let json = serde_json::to_string(&shadow).unwrap();
        let deserialized: Shadow = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.blur, shadow.blur);
    }

    #[test]
    fn test_breakpoint_serialization() {
        let bp = Breakpoint::GtMobile;
        let json = serde_json::to_string(&bp).unwrap();
        let deserialized: Breakpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, Breakpoint::GtMobile);
    }
}
