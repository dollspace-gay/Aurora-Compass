//! Typography system for Aurora Compass
//!
//! This module provides a comprehensive typography system matching
//! the Bluesky client's text styles and font handling.

use crate::tokens::{font_weight, line_height, tracking};
use serde::{Deserialize, Serialize};

// =============================================================================
// Font Size Scale
// =============================================================================

/// Font size scale in pixels
pub mod font_size {
    /// Extra small (13px)
    pub const XS: f32 = 13.0;
    /// Small (14px)
    pub const SM: f32 = 14.0;
    /// Medium (15px)
    pub const MD: f32 = 15.0;
    /// Large (16px)
    pub const LG: f32 = 16.0;
    /// Extra large (17px)
    pub const XL: f32 = 17.0;
    /// 2x large (18px)
    pub const XXL: f32 = 18.0;

    /// Title sizes
    pub mod title {
        /// Small title (17px)
        pub const SM: f32 = 17.0;
        /// Base title (20px)
        pub const BASE: f32 = 20.0;
        /// Large title (22px)
        pub const LG: f32 = 22.0;
        /// Extra large title (28px)
        pub const XL: f32 = 28.0;
        /// 2x large title (34px)
        pub const XXL: f32 = 34.0;
    }

    /// Post text sizes
    pub mod post {
        /// Normal post text (16px)
        pub const NORMAL: f32 = 16.0;
        /// Large post text (20px)
        pub const LARGE: f32 = 20.0;
    }

    /// Button text sizes
    pub mod button {
        /// Normal button (14px)
        pub const NORMAL: f32 = 14.0;
        /// Large button (18px)
        pub const LARGE: f32 = 18.0;
    }

    /// Monospace text size (14px)
    pub const MONO: f32 = 14.0;
}

// =============================================================================
// Typography Style
// =============================================================================

/// A typography style definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextStyle {
    /// Font size in pixels
    pub font_size: f32,
    /// Font weight (400, 500, 600, 700, 800)
    pub font_weight: u16,
    /// Line height multiplier
    pub line_height: f32,
    /// Letter spacing in em
    pub letter_spacing: f32,
    /// Font family override (None = system default)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
}

impl TextStyle {
    /// Create a new text style
    pub fn new(font_size: f32, font_weight: u16) -> Self {
        Self {
            font_size,
            font_weight,
            line_height: line_height::NORMAL,
            letter_spacing: tracking::DEFAULT,
            font_family: None,
        }
    }

    /// Set line height
    pub fn with_line_height(mut self, lh: f32) -> Self {
        self.line_height = lh;
        self
    }

    /// Set letter spacing
    pub fn with_letter_spacing(mut self, ls: f32) -> Self {
        self.letter_spacing = ls;
        self
    }

    /// Set font family
    pub fn with_font_family(mut self, family: impl Into<String>) -> Self {
        self.font_family = Some(family.into());
        self
    }

    /// Calculate the actual line height in pixels
    pub fn line_height_px(&self) -> f32 {
        self.font_size * self.line_height
    }

    /// Scale the font size by a multiplier
    pub fn scale(&self, multiplier: f32) -> Self {
        Self {
            font_size: self.font_size * multiplier,
            font_weight: self.font_weight,
            line_height: self.line_height,
            letter_spacing: self.letter_spacing,
            font_family: self.font_family.clone(),
        }
    }
}

// =============================================================================
// Typography Variants
// =============================================================================

/// Typography variant identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TypographyVariant {
    // Regular text variants
    /// Extra small thin
    Xs,
    /// Extra small thin
    XsThin,
    /// Extra small medium
    XsMedium,
    /// Extra small bold
    XsBold,
    /// Extra small heavy
    XsHeavy,

    /// Small
    Sm,
    /// Small thin
    SmThin,
    /// Small medium
    SmMedium,
    /// Small bold
    SmBold,
    /// Small heavy
    SmHeavy,

    /// Medium (base)
    #[default]
    Md,
    /// Medium thin
    MdThin,
    /// Medium medium weight
    MdMedium,
    /// Medium bold
    MdBold,
    /// Medium heavy
    MdHeavy,

    /// Large
    Lg,
    /// Large thin
    LgThin,
    /// Large medium
    LgMedium,
    /// Large bold
    LgBold,
    /// Large heavy
    LgHeavy,

    /// Extra large
    Xl,
    /// Extra large thin
    XlThin,
    /// Extra large medium
    XlMedium,
    /// Extra large bold
    XlBold,
    /// Extra large heavy
    XlHeavy,

    /// 2x large
    Xxl,
    /// 2x large thin
    XxlThin,
    /// 2x large medium
    XxlMedium,
    /// 2x large bold
    XxlBold,
    /// 2x large heavy
    XxlHeavy,

    // Title variants
    /// Title (default)
    Title,
    /// Small title
    TitleSm,
    /// Large title
    TitleLg,
    /// Extra large title
    TitleXl,
    /// 2x large title
    Title2xl,

    // Special variants
    /// Post text
    PostText,
    /// Large post text
    PostTextLg,
    /// Button text
    Button,
    /// Large button text
    ButtonLg,
    /// Monospace text
    Mono,
}

impl TypographyVariant {
    /// Get the text style for this variant
    pub fn style(&self) -> TextStyle {
        match self {
            // XS variants (13px)
            Self::Xs | Self::XsThin => {
                TextStyle::new(font_size::XS, font_weight::NORMAL)
            }
            Self::XsMedium => {
                TextStyle::new(font_size::XS, font_weight::SEMI_BOLD)
            }
            Self::XsBold => {
                TextStyle::new(font_size::XS, font_weight::SEMI_BOLD)
            }
            Self::XsHeavy => {
                TextStyle::new(font_size::XS, font_weight::BOLD)
            }

            // SM variants (14px)
            Self::Sm | Self::SmThin => {
                TextStyle::new(font_size::SM, font_weight::NORMAL)
            }
            Self::SmMedium => {
                TextStyle::new(font_size::SM, font_weight::SEMI_BOLD)
            }
            Self::SmBold => {
                TextStyle::new(font_size::SM, font_weight::SEMI_BOLD)
            }
            Self::SmHeavy => {
                TextStyle::new(font_size::SM, font_weight::BOLD)
            }

            // MD variants (15px)
            Self::Md | Self::MdThin => {
                TextStyle::new(font_size::MD, font_weight::NORMAL)
            }
            Self::MdMedium => {
                TextStyle::new(font_size::MD, font_weight::SEMI_BOLD)
            }
            Self::MdBold => {
                TextStyle::new(font_size::MD, font_weight::SEMI_BOLD)
            }
            Self::MdHeavy => {
                TextStyle::new(font_size::MD, font_weight::BOLD)
            }

            // LG variants (16px)
            Self::Lg | Self::LgThin => {
                TextStyle::new(font_size::LG, font_weight::NORMAL)
            }
            Self::LgMedium => {
                TextStyle::new(font_size::LG, font_weight::SEMI_BOLD)
            }
            Self::LgBold => {
                TextStyle::new(font_size::LG, font_weight::SEMI_BOLD)
            }
            Self::LgHeavy => {
                TextStyle::new(font_size::LG, font_weight::BOLD)
            }

            // XL variants (17px)
            Self::Xl | Self::XlThin => {
                TextStyle::new(font_size::XL, font_weight::NORMAL)
            }
            Self::XlMedium => {
                TextStyle::new(font_size::XL, font_weight::SEMI_BOLD)
            }
            Self::XlBold => {
                TextStyle::new(font_size::XL, font_weight::SEMI_BOLD)
            }
            Self::XlHeavy => {
                TextStyle::new(font_size::XL, font_weight::BOLD)
            }

            // XXL variants (18px)
            Self::Xxl | Self::XxlThin => {
                TextStyle::new(font_size::XXL, font_weight::NORMAL)
            }
            Self::XxlMedium => {
                TextStyle::new(font_size::XXL, font_weight::SEMI_BOLD)
            }
            Self::XxlBold => {
                TextStyle::new(font_size::XXL, font_weight::SEMI_BOLD)
            }
            Self::XxlHeavy => {
                TextStyle::new(font_size::XXL, font_weight::BOLD)
            }

            // Title variants
            Self::TitleSm => {
                TextStyle::new(font_size::title::SM, font_weight::SEMI_BOLD)
            }
            Self::Title => {
                TextStyle::new(font_size::title::BASE, font_weight::SEMI_BOLD)
            }
            Self::TitleLg => {
                TextStyle::new(font_size::title::LG, font_weight::SEMI_BOLD)
            }
            Self::TitleXl => {
                TextStyle::new(font_size::title::XL, font_weight::SEMI_BOLD)
            }
            Self::Title2xl => {
                TextStyle::new(font_size::title::XXL, font_weight::SEMI_BOLD)
            }

            // Special variants
            Self::PostText => {
                TextStyle::new(font_size::post::NORMAL, font_weight::NORMAL)
            }
            Self::PostTextLg => {
                TextStyle::new(font_size::post::LARGE, font_weight::NORMAL)
            }
            Self::Button => {
                TextStyle::new(font_size::button::NORMAL, font_weight::SEMI_BOLD)
            }
            Self::ButtonLg => {
                TextStyle::new(font_size::button::LARGE, font_weight::SEMI_BOLD)
            }
            Self::Mono => {
                TextStyle::new(font_size::MONO, font_weight::NORMAL)
                    .with_font_family("monospace")
            }
        }
    }
}

// =============================================================================
// Typography System
// =============================================================================

/// Complete typography system with all variants
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Typography {
    /// Font scale multiplier (for accessibility)
    pub scale: f32,
    /// All text styles
    pub styles: std::collections::HashMap<TypographyVariant, TextStyle>,
}

impl Default for Typography {
    fn default() -> Self {
        Self::new(1.0)
    }
}

impl Typography {
    /// Create a new typography system with the given scale
    pub fn new(scale: f32) -> Self {
        use TypographyVariant::*;

        let variants = [
            Xs, XsThin, XsMedium, XsBold, XsHeavy,
            Sm, SmThin, SmMedium, SmBold, SmHeavy,
            Md, MdThin, MdMedium, MdBold, MdHeavy,
            Lg, LgThin, LgMedium, LgBold, LgHeavy,
            Xl, XlThin, XlMedium, XlBold, XlHeavy,
            Xxl, XxlThin, XxlMedium, XxlBold, XxlHeavy,
            Title, TitleSm, TitleLg, TitleXl, Title2xl,
            PostText, PostTextLg,
            Button, ButtonLg,
            Mono,
        ];

        let styles = variants
            .iter()
            .map(|v| (*v, v.style().scale(scale)))
            .collect();

        Self { scale, styles }
    }

    /// Get a text style by variant
    pub fn get(&self, variant: TypographyVariant) -> Option<&TextStyle> {
        self.styles.get(&variant)
    }

    /// Set the font scale and recalculate all styles
    pub fn set_scale(&mut self, scale: f32) {
        let clamped = scale.clamp(0.8, 1.4);
        if (clamped - self.scale).abs() > f32::EPSILON {
            *self = Self::new(clamped);
        }
    }

    /// Get the current scale
    pub fn current_scale(&self) -> f32 {
        self.scale
    }
}

// =============================================================================
// Font Family Configuration
// =============================================================================

/// Platform font stack configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FontStack {
    /// Primary font family
    pub primary: String,
    /// Fallback fonts
    pub fallbacks: Vec<String>,
}

impl FontStack {
    /// Create a new font stack
    pub fn new(primary: impl Into<String>) -> Self {
        Self {
            primary: primary.into(),
            fallbacks: vec![
                "-apple-system".to_string(),
                "BlinkMacSystemFont".to_string(),
                "Segoe UI".to_string(),
                "Roboto".to_string(),
                "Helvetica Neue".to_string(),
                "Arial".to_string(),
                "sans-serif".to_string(),
            ],
        }
    }

    /// Create the system default font stack
    pub fn system() -> Self {
        Self {
            primary: "system-ui".to_string(),
            fallbacks: vec![
                "-apple-system".to_string(),
                "BlinkMacSystemFont".to_string(),
                "Segoe UI".to_string(),
                "Roboto".to_string(),
                "Helvetica Neue".to_string(),
                "Arial".to_string(),
                "sans-serif".to_string(),
            ],
        }
    }

    /// Create an Inter font stack
    pub fn inter() -> Self {
        Self::new("Inter")
    }

    /// Create a monospace font stack
    pub fn monospace() -> Self {
        Self {
            primary: "ui-monospace".to_string(),
            fallbacks: vec![
                "SF Mono".to_string(),
                "SFMono-Regular".to_string(),
                "Menlo".to_string(),
                "Monaco".to_string(),
                "Consolas".to_string(),
                "Liberation Mono".to_string(),
                "Courier New".to_string(),
                "monospace".to_string(),
            ],
        }
    }

    /// Get the CSS font-family string
    pub fn to_css(&self) -> String {
        let mut fonts = vec![format!("\"{}\"", self.primary)];
        fonts.extend(self.fallbacks.iter().map(|f| {
            if f.contains(' ') && !f.starts_with('-') {
                format!("\"{}\"", f)
            } else {
                f.clone()
            }
        }));
        fonts.join(", ")
    }
}

impl Default for FontStack {
    fn default() -> Self {
        Self::system()
    }
}

// =============================================================================
// Text Transform
// =============================================================================

/// Text transform options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TextTransform {
    /// No transform
    #[default]
    None,
    /// UPPERCASE
    Uppercase,
    /// lowercase
    Lowercase,
    /// Capitalize Each Word
    Capitalize,
}

impl TextTransform {
    /// Apply the transform to text
    pub fn apply(&self, text: &str) -> String {
        match self {
            Self::None => text.to_string(),
            Self::Uppercase => text.to_uppercase(),
            Self::Lowercase => text.to_lowercase(),
            Self::Capitalize => {
                text.split_whitespace()
                    .map(|word| {
                        let mut chars = word.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(first) => {
                                first.to_uppercase().chain(chars.map(|c| c.to_ascii_lowercase())).collect()
                            }
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        }
    }
}

// =============================================================================
// Text Decoration
// =============================================================================

/// Text decoration options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TextDecoration {
    /// No decoration
    #[default]
    None,
    /// Underline
    Underline,
    /// Line through (strikethrough)
    LineThrough,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Font Size Tests
    // ==========================================================================

    #[test]
    fn test_font_size_scale() {
        assert!(font_size::XS < font_size::SM);
        assert!(font_size::SM < font_size::MD);
        assert!(font_size::MD < font_size::LG);
        assert!(font_size::LG < font_size::XL);
        assert!(font_size::XL < font_size::XXL);
    }

    #[test]
    fn test_title_sizes() {
        assert!(font_size::title::SM < font_size::title::BASE);
        assert!(font_size::title::BASE < font_size::title::LG);
        assert!(font_size::title::LG < font_size::title::XL);
        assert!(font_size::title::XL < font_size::title::XXL);
    }

    // ==========================================================================
    // TextStyle Tests
    // ==========================================================================

    #[test]
    fn test_text_style_new() {
        let style = TextStyle::new(16.0, 400);
        assert_eq!(style.font_size, 16.0);
        assert_eq!(style.font_weight, 400);
        assert!(style.font_family.is_none());
    }

    #[test]
    fn test_text_style_builder() {
        let style = TextStyle::new(16.0, 400)
            .with_line_height(1.5)
            .with_letter_spacing(0.02)
            .with_font_family("Inter");

        assert_eq!(style.line_height, 1.5);
        assert_eq!(style.letter_spacing, 0.02);
        assert_eq!(style.font_family, Some("Inter".to_string()));
    }

    #[test]
    fn test_text_style_line_height_px() {
        let style = TextStyle::new(16.0, 400).with_line_height(1.5);
        assert_eq!(style.line_height_px(), 24.0);
    }

    #[test]
    fn test_text_style_scale() {
        let style = TextStyle::new(16.0, 400);
        let scaled = style.scale(1.25);
        assert_eq!(scaled.font_size, 20.0);
        assert_eq!(scaled.font_weight, 400); // Weight unchanged
    }

    // ==========================================================================
    // Typography Variant Tests
    // ==========================================================================

    #[test]
    fn test_typography_variant_style() {
        let xs = TypographyVariant::Xs.style();
        assert_eq!(xs.font_size, 13.0);
        assert_eq!(xs.font_weight, 400);

        let title = TypographyVariant::Title.style();
        assert_eq!(title.font_size, 20.0);
        assert_eq!(title.font_weight, 600);

        let mono = TypographyVariant::Mono.style();
        assert!(mono.font_family.is_some());
    }

    #[test]
    fn test_weight_variants() {
        let thin = TypographyVariant::SmThin.style();
        let medium = TypographyVariant::SmMedium.style();
        let bold = TypographyVariant::SmBold.style();
        let heavy = TypographyVariant::SmHeavy.style();

        // Same size, different weights
        assert_eq!(thin.font_size, medium.font_size);
        assert!(thin.font_weight < medium.font_weight);
        assert!(medium.font_weight <= bold.font_weight);
        assert!(bold.font_weight <= heavy.font_weight);
    }

    // ==========================================================================
    // Typography System Tests
    // ==========================================================================

    #[test]
    fn test_typography_default() {
        let typo = Typography::default();
        assert_eq!(typo.scale, 1.0);
        assert!(!typo.styles.is_empty());
    }

    #[test]
    fn test_typography_get() {
        let typo = Typography::default();

        let md = typo.get(TypographyVariant::Md);
        assert!(md.is_some());
        assert_eq!(md.unwrap().font_size, 15.0);

        let title = typo.get(TypographyVariant::Title);
        assert!(title.is_some());
        assert_eq!(title.unwrap().font_size, 20.0);
    }

    #[test]
    fn test_typography_scale() {
        let mut typo = Typography::new(1.0);
        typo.set_scale(1.2);

        assert_eq!(typo.current_scale(), 1.2);

        // Font sizes should be scaled
        let md = typo.get(TypographyVariant::Md).unwrap();
        assert_eq!(md.font_size, 15.0 * 1.2);
    }

    #[test]
    fn test_typography_scale_clamping() {
        let mut typo = Typography::default();

        typo.set_scale(0.5);
        assert_eq!(typo.current_scale(), 0.8); // Clamped to min

        typo.set_scale(2.0);
        assert_eq!(typo.current_scale(), 1.4); // Clamped to max
    }

    // ==========================================================================
    // Font Stack Tests
    // ==========================================================================

    #[test]
    fn test_font_stack_system() {
        let stack = FontStack::system();
        assert!(stack.primary.contains("system"));
        assert!(!stack.fallbacks.is_empty());
    }

    #[test]
    fn test_font_stack_inter() {
        let stack = FontStack::inter();
        assert_eq!(stack.primary, "Inter");
    }

    #[test]
    fn test_font_stack_monospace() {
        let stack = FontStack::monospace();
        assert!(stack.fallbacks.iter().any(|f| f.contains("mono")));
    }

    #[test]
    fn test_font_stack_to_css() {
        let stack = FontStack::new("Inter");
        let css = stack.to_css();

        assert!(css.starts_with("\"Inter\""));
        assert!(css.contains("sans-serif"));
    }

    // ==========================================================================
    // Text Transform Tests
    // ==========================================================================

    #[test]
    fn test_text_transform_none() {
        let transform = TextTransform::None;
        assert_eq!(transform.apply("Hello World"), "Hello World");
    }

    #[test]
    fn test_text_transform_uppercase() {
        let transform = TextTransform::Uppercase;
        assert_eq!(transform.apply("Hello World"), "HELLO WORLD");
    }

    #[test]
    fn test_text_transform_lowercase() {
        let transform = TextTransform::Lowercase;
        assert_eq!(transform.apply("Hello World"), "hello world");
    }

    #[test]
    fn test_text_transform_capitalize() {
        let transform = TextTransform::Capitalize;
        assert_eq!(transform.apply("hello world"), "Hello World");
        assert_eq!(transform.apply("HELLO WORLD"), "Hello World");
    }

    // ==========================================================================
    // Serialization Tests
    // ==========================================================================

    #[test]
    fn test_text_style_serialization() {
        let style = TextStyle::new(16.0, 600)
            .with_line_height(1.5);

        let json = serde_json::to_string(&style).unwrap();
        let deserialized: TextStyle = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.font_size, 16.0);
        assert_eq!(deserialized.font_weight, 600);
        assert_eq!(deserialized.line_height, 1.5);
    }

    #[test]
    fn test_typography_variant_serialization() {
        let variant = TypographyVariant::TitleLg;
        let json = serde_json::to_string(&variant).unwrap();
        assert_eq!(json, "\"title-lg\"");

        let deserialized: TypographyVariant = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, TypographyVariant::TitleLg);
    }

    #[test]
    fn test_text_transform_serialization() {
        let transform = TextTransform::Uppercase;
        let json = serde_json::to_string(&transform).unwrap();
        assert_eq!(json, "\"uppercase\"");
    }

    #[test]
    fn test_font_stack_serialization() {
        let stack = FontStack::inter();
        let json = serde_json::to_string(&stack).unwrap();
        let deserialized: FontStack = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.primary, "Inter");
    }
}
