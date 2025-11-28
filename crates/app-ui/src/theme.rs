//! Design system and theme provider for Aurora Compass
//!
//! This module provides a comprehensive theming system that mirrors the structure
//! of Bluesky's ALF (Application Layout Framework) while using Aurora Compass's
//! custom aurora borealis color palette.
//!
//! # Themes
//!
//! Three themes are supported:
//! - Light: A bright theme with white background
//! - Dark: A dark theme with near-black background
//! - Dim: A dimmed dark theme with softer contrast
//!
//! # Usage
//!
//! ```rust
//! use app_ui::theme::{Theme, ThemeName, get_theme};
//!
//! let theme = get_theme(ThemeName::Dark);
//! let bg_color = &theme.colors.default.background;
//! let primary = &theme.palette.primary.primary_500;
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Color Types
// =============================================================================

/// A color represented as an RGBA hex string (e.g., "#FFFFFF" or "#FFFFFF80")
pub type Color = String;

/// Parse a hex color string to RGB components
pub fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() < 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

/// Convert RGB to hex string
pub fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

// =============================================================================
// Brand Colors (Aurora Borealis Theme)
// =============================================================================

/// Aurora Compass brand colors derived from the logo
pub mod brand {
    /// Primary brand color (Navy blue from compass)
    pub const PRIMARY: &str = "#1E3A5F";

    /// Secondary purple (Aurora purple)
    pub const SECONDARY_PURPLE: &str = "#9D4EDD";

    /// Secondary blue (Aurora blue)
    pub const SECONDARY_BLUE: &str = "#3A86FF";

    /// Secondary cyan (Aurora cyan/green)
    pub const SECONDARY_CYAN: &str = "#06FFA5";

    /// Accent gold (Compass detail)
    pub const ACCENT_GOLD: &str = "#FFB703";

    /// Pure white
    pub const WHITE: &str = "#FFFFFF";

    /// Pure black
    pub const BLACK: &str = "#000000";
}

// =============================================================================
// Color Scale (used across themes)
// =============================================================================

/// A complete color scale with 13 stops from lightest to darkest
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorScale {
    /// Lightest (25)
    pub s25: Color,
    /// Very light (50)
    pub s50: Color,
    /// Light (100)
    pub s100: Color,
    /// Light-medium (200)
    pub s200: Color,
    /// Medium-light (300)
    pub s300: Color,
    /// Medium (400)
    pub s400: Color,
    /// Base (500)
    pub s500: Color,
    /// Medium-dark (600)
    pub s600: Color,
    /// Dark-medium (700)
    pub s700: Color,
    /// Dark (800)
    pub s800: Color,
    /// Very dark (900)
    pub s900: Color,
    /// Darkest (950)
    pub s950: Color,
    /// Near black (975)
    pub s975: Color,
}

impl ColorScale {
    /// Get a color by its numeric stop (25, 50, 100, ..., 975)
    pub fn get(&self, stop: u16) -> Option<&Color> {
        match stop {
            25 => Some(&self.s25),
            50 => Some(&self.s50),
            100 => Some(&self.s100),
            200 => Some(&self.s200),
            300 => Some(&self.s300),
            400 => Some(&self.s400),
            500 => Some(&self.s500),
            600 => Some(&self.s600),
            700 => Some(&self.s700),
            800 => Some(&self.s800),
            900 => Some(&self.s900),
            950 => Some(&self.s950),
            975 => Some(&self.s975),
            _ => None,
        }
    }
}

// =============================================================================
// Theme Palette
// =============================================================================

/// Contrast scale for neutral colors (backgrounds, borders, text)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContrastPalette {
    /// Nearly transparent/white (for light themes) or nearly black (for dark)
    pub contrast_0: Color,
    /// Lightest visible contrast
    pub contrast_25: Color,
    /// Very light contrast
    pub contrast_50: Color,
    /// Light contrast
    pub contrast_100: Color,
    /// Light-medium contrast
    pub contrast_200: Color,
    /// Medium-light contrast
    pub contrast_300: Color,
    /// Medium contrast
    pub contrast_400: Color,
    /// Base contrast
    pub contrast_500: Color,
    /// Medium-dark contrast
    pub contrast_600: Color,
    /// Dark-medium contrast
    pub contrast_700: Color,
    /// Dark contrast
    pub contrast_800: Color,
    /// Very dark contrast
    pub contrast_900: Color,
    /// Darkest contrast
    pub contrast_950: Color,
    /// Near maximum contrast
    pub contrast_975: Color,
}

/// Primary color scale (Aurora purple/blue gradient)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrimaryPalette {
    pub primary_25: Color,
    pub primary_50: Color,
    pub primary_100: Color,
    pub primary_200: Color,
    pub primary_300: Color,
    pub primary_400: Color,
    pub primary_500: Color,
    pub primary_600: Color,
    pub primary_700: Color,
    pub primary_800: Color,
    pub primary_900: Color,
    pub primary_950: Color,
    pub primary_975: Color,
}

/// Positive/success color scale (Aurora cyan/green)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositivePalette {
    pub positive_25: Color,
    pub positive_50: Color,
    pub positive_100: Color,
    pub positive_200: Color,
    pub positive_300: Color,
    pub positive_400: Color,
    pub positive_500: Color,
    pub positive_600: Color,
    pub positive_700: Color,
    pub positive_800: Color,
    pub positive_900: Color,
    pub positive_950: Color,
    pub positive_975: Color,
}

/// Negative/error color scale (warm red/orange tones)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NegativePalette {
    pub negative_25: Color,
    pub negative_50: Color,
    pub negative_100: Color,
    pub negative_200: Color,
    pub negative_300: Color,
    pub negative_400: Color,
    pub negative_500: Color,
    pub negative_600: Color,
    pub negative_700: Color,
    pub negative_800: Color,
    pub negative_900: Color,
    pub negative_950: Color,
    pub negative_975: Color,
}

/// Complete color palette for a theme
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Palette {
    /// White color
    pub white: Color,
    /// Black color
    pub black: Color,
    /// Contrast/neutral colors
    #[serde(flatten)]
    pub contrast: ContrastPalette,
    /// Primary brand colors
    #[serde(flatten)]
    pub primary: PrimaryPalette,
    /// Positive/success colors
    #[serde(flatten)]
    pub positive: PositivePalette,
    /// Negative/error colors
    #[serde(flatten)]
    pub negative: NegativePalette,
}

// =============================================================================
// Semantic Colors
// =============================================================================

/// Semantic colors for specific UI purposes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticColors {
    /// Main background color
    pub background: Color,
    /// Lighter background (for cards, elevated surfaces)
    pub background_light: Color,
    /// Primary text color
    pub text: Color,
    /// Secondary/muted text color
    pub text_light: Color,
    /// Very muted text color
    pub text_very_light: Color,
    /// Text color on inverted backgrounds
    pub text_inverted: Color,
    /// Link color
    pub link: Color,
    /// Border color
    pub border: Color,
    /// Darker border color
    pub border_dark: Color,
    /// Border color on link hover
    pub border_link_hover: Color,
    /// Icon color
    pub icon: Color,
    /// Reply thread line color
    pub reply_line: Color,
    /// Reply thread line dot color
    pub reply_line_dot: Color,
    /// Unread notification background
    pub unread_notif_bg: Color,
    /// Unread notification border
    pub unread_notif_border: Color,
    /// Post action button color
    pub post_ctrl: Color,
    /// Brand text color
    pub brand_text: Color,
    /// Empty state icon color
    pub empty_state_icon: Color,
}

/// Color set for a specific purpose (primary, secondary, error)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorSet {
    /// Background color
    pub background: Color,
    /// Light background variant
    pub background_light: Color,
    /// Text color
    pub text: Color,
    /// Light text color
    pub text_light: Color,
    /// Inverted text color
    pub text_inverted: Color,
    /// Link color
    pub link: Color,
    /// Border color
    pub border: Color,
    /// Dark border color
    pub border_dark: Color,
    /// Icon color
    pub icon: Color,
}

/// All semantic color sets for a theme
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeColors {
    /// Default colors (for general UI)
    pub default: SemanticColors,
    /// Primary action colors (buttons, links, etc.)
    pub primary: ColorSet,
    /// Secondary colors
    pub secondary: ColorSet,
    /// Inverted colors (for contrast areas)
    pub inverted: ColorSet,
    /// Error/danger colors
    pub error: ColorSet,
}

// =============================================================================
// Gradients
// =============================================================================

/// A gradient stop with position and color
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GradientStop {
    /// Position from 0.0 to 1.0
    pub position: f32,
    /// Color at this position
    pub color: Color,
}

/// A gradient definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gradient {
    /// Gradient stops
    pub stops: Vec<GradientStop>,
    /// Color to use on hover
    pub hover_color: Color,
}

impl Gradient {
    /// Create a new gradient with stops
    pub fn new(stops: Vec<(f32, &str)>, hover: &str) -> Self {
        Self {
            stops: stops
                .into_iter()
                .map(|(pos, color)| GradientStop {
                    position: pos,
                    color: color.to_string(),
                })
                .collect(),
            hover_color: hover.to_string(),
        }
    }
}

/// Aurora-themed gradient presets
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gradients {
    /// Primary aurora gradient (purple to cyan)
    pub primary: Gradient,
    /// Aurora gradient variation
    pub aurora: Gradient,
    /// Midnight aurora (dark blue to purple)
    pub midnight: Gradient,
    /// Sunrise aurora (cyan to gold)
    pub sunrise: Gradient,
    /// Sunset aurora (purple to pink)
    pub sunset: Gradient,
    /// Northern lights (green dominant)
    pub northern: Gradient,
    /// Compass gradient (navy to gold)
    pub compass: Gradient,
}

impl Default for Gradients {
    fn default() -> Self {
        Self {
            primary: Gradient::new(
                vec![
                    (0.0, "#9D4EDD"),  // Aurora purple
                    (0.4, "#3A86FF"),  // Aurora blue
                    (0.7, "#3A86FF"),  // Aurora blue
                    (1.0, "#06FFA5"),  // Aurora cyan
                ],
                "#3A86FF",
            ),
            aurora: Gradient::new(
                vec![
                    (0.0, "#9D4EDD"),  // Purple
                    (0.5, "#3A86FF"),  // Blue
                    (1.0, "#06FFA5"),  // Cyan
                ],
                "#3A86FF",
            ),
            midnight: Gradient::new(
                vec![
                    (0.0, "#1E3A5F"),  // Navy
                    (0.5, "#4A2C7F"),  // Dark purple
                    (1.0, "#9D4EDD"),  // Purple
                ],
                "#4A2C7F",
            ),
            sunrise: Gradient::new(
                vec![
                    (0.0, "#06FFA5"),  // Cyan
                    (0.4, "#3A86FF"),  // Blue
                    (0.7, "#9D4EDD"),  // Purple
                    (1.0, "#FFB703"),  // Gold
                ],
                "#3A86FF",
            ),
            sunset: Gradient::new(
                vec![
                    (0.0, "#9D4EDD"),  // Purple
                    (0.5, "#D946EF"),  // Pink/magenta
                    (1.0, "#FF6B6B"),  // Coral
                ],
                "#D946EF",
            ),
            northern: Gradient::new(
                vec![
                    (0.0, "#1E3A5F"),  // Navy
                    (0.3, "#06FFA5"),  // Cyan
                    (0.7, "#00D68F"),  // Green
                    (1.0, "#3A86FF"),  // Blue
                ],
                "#06FFA5",
            ),
            compass: Gradient::new(
                vec![
                    (0.0, "#1E3A5F"),  // Navy
                    (0.6, "#3A86FF"),  // Blue
                    (1.0, "#FFB703"),  // Gold
                ],
                "#3A86FF",
            ),
        }
    }
}

// =============================================================================
// Theme Definition
// =============================================================================

/// Theme name enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeName {
    /// Light theme
    #[default]
    Light,
    /// Dark theme
    Dark,
    /// Dim theme (softer dark)
    Dim,
}

impl ThemeName {
    /// Get the color scheme name
    pub fn color_scheme(&self) -> &'static str {
        match self {
            ThemeName::Light => "light",
            ThemeName::Dark => "dark",
            ThemeName::Dim => "dim",
        }
    }
}

impl std::fmt::Display for ThemeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemeName::Light => write!(f, "Light"),
            ThemeName::Dark => write!(f, "Dark"),
            ThemeName::Dim => write!(f, "Dim"),
        }
    }
}

impl std::str::FromStr for ThemeName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "light" => Ok(ThemeName::Light),
            "dark" => Ok(ThemeName::Dark),
            "dim" => Ok(ThemeName::Dim),
            _ => Err(format!("Unknown theme: {}", s)),
        }
    }
}

/// Complete theme definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    /// Theme name
    pub name: ThemeName,
    /// Color scheme (light or dark)
    pub color_scheme: String,
    /// Color palette
    pub palette: Palette,
    /// Semantic theme colors
    pub colors: ThemeColors,
    /// Gradient definitions
    pub gradients: Gradients,
}

impl Theme {
    /// Check if this is a dark theme
    pub fn is_dark(&self) -> bool {
        matches!(self.name, ThemeName::Dark | ThemeName::Dim)
    }

    /// Get a contrast color by level (0-975)
    pub fn contrast(&self, level: u16) -> &str {
        match level {
            0 => &self.palette.contrast.contrast_0,
            25 => &self.palette.contrast.contrast_25,
            50 => &self.palette.contrast.contrast_50,
            100 => &self.palette.contrast.contrast_100,
            200 => &self.palette.contrast.contrast_200,
            300 => &self.palette.contrast.contrast_300,
            400 => &self.palette.contrast.contrast_400,
            500 => &self.palette.contrast.contrast_500,
            600 => &self.palette.contrast.contrast_600,
            700 => &self.palette.contrast.contrast_700,
            800 => &self.palette.contrast.contrast_800,
            900 => &self.palette.contrast.contrast_900,
            950 => &self.palette.contrast.contrast_950,
            975 => &self.palette.contrast.contrast_975,
            _ => &self.palette.contrast.contrast_500,
        }
    }

    /// Get a primary color by level (25-975)
    pub fn primary(&self, level: u16) -> &str {
        match level {
            25 => &self.palette.primary.primary_25,
            50 => &self.palette.primary.primary_50,
            100 => &self.palette.primary.primary_100,
            200 => &self.palette.primary.primary_200,
            300 => &self.palette.primary.primary_300,
            400 => &self.palette.primary.primary_400,
            500 => &self.palette.primary.primary_500,
            600 => &self.palette.primary.primary_600,
            700 => &self.palette.primary.primary_700,
            800 => &self.palette.primary.primary_800,
            900 => &self.palette.primary.primary_900,
            950 => &self.palette.primary.primary_950,
            975 => &self.palette.primary.primary_975,
            _ => &self.palette.primary.primary_500,
        }
    }
}

// =============================================================================
// Light Theme
// =============================================================================

/// Create the light theme
pub fn light_theme() -> Theme {
    Theme {
        name: ThemeName::Light,
        color_scheme: "light".to_string(),
        palette: Palette {
            white: "#FFFFFF".to_string(),
            black: "#000000".to_string(),
            contrast: ContrastPalette {
                contrast_0: "#FFFFFF".to_string(),
                contrast_25: "#F7F7F7".to_string(),
                contrast_50: "#F0F0F0".to_string(),
                contrast_100: "#E5E5E5".to_string(),
                contrast_200: "#D4D4D4".to_string(),
                contrast_300: "#B3B3B3".to_string(),
                contrast_400: "#8F8F8F".to_string(),
                contrast_500: "#6B6B6B".to_string(),
                contrast_600: "#525252".to_string(),
                contrast_700: "#3D3D3D".to_string(),
                contrast_800: "#2E2E2E".to_string(),
                contrast_900: "#1F1F1F".to_string(),
                contrast_950: "#141414".to_string(),
                contrast_975: "#0A0A0A".to_string(),
            },
            primary: PrimaryPalette {
                // Aurora purple/blue gradient - light variants
                primary_25: "#F5F0FF".to_string(),   // Very light purple
                primary_50: "#EDE4FF".to_string(),   // Light purple
                primary_100: "#D9C9FF".to_string(),  // Lighter purple
                primary_200: "#C4ADFF".to_string(),  // Light purple
                primary_300: "#B392FF".to_string(),  // Medium light purple
                primary_400: "#A77BFF".to_string(),  // Medium purple
                primary_500: "#9D4EDD".to_string(),  // Aurora purple (brand)
                primary_600: "#8A3DC7".to_string(),  // Darker purple
                primary_700: "#752FB0".to_string(),  // Dark purple
                primary_800: "#602499".to_string(),  // Darker purple
                primary_900: "#4A1A7A".to_string(),  // Very dark purple
                primary_950: "#35125A".to_string(),  // Near black purple
                primary_975: "#200A3A".to_string(),  // Deepest purple
            },
            positive: PositivePalette {
                // Aurora cyan/green - success colors
                positive_25: "#E6FFF5".to_string(),
                positive_50: "#CCFFEB".to_string(),
                positive_100: "#99FFD6".to_string(),
                positive_200: "#66FFC2".to_string(),
                positive_300: "#33FFAD".to_string(),
                positive_400: "#1AFF9F".to_string(),
                positive_500: "#06FFA5".to_string(),  // Aurora cyan (brand)
                positive_600: "#05D98D".to_string(),
                positive_700: "#04B376".to_string(),
                positive_800: "#038C5E".to_string(),
                positive_900: "#026647".to_string(),
                positive_950: "#014030".to_string(),
                positive_975: "#00261D".to_string(),
            },
            negative: NegativePalette {
                // Error/warning colors
                negative_25: "#FFF0F0".to_string(),
                negative_50: "#FFE0E0".to_string(),
                negative_100: "#FFC2C2".to_string(),
                negative_200: "#FFA3A3".to_string(),
                negative_300: "#FF8585".to_string(),
                negative_400: "#FF6B6B".to_string(),
                negative_500: "#EF4444".to_string(),
                negative_600: "#DC2626".to_string(),
                negative_700: "#B91C1C".to_string(),
                negative_800: "#991B1B".to_string(),
                negative_900: "#7F1D1D".to_string(),
                negative_950: "#5C1414".to_string(),
                negative_975: "#3B0D0D".to_string(),
            },
        },
        colors: ThemeColors {
            default: SemanticColors {
                background: "#FFFFFF".to_string(),
                background_light: "#F7F7F7".to_string(),
                text: "#000000".to_string(),
                text_light: "#3D3D3D".to_string(),
                text_very_light: "#8F8F8F".to_string(),
                text_inverted: "#FFFFFF".to_string(),
                link: "#9D4EDD".to_string(),  // Aurora purple
                border: "#E5E5E5".to_string(),
                border_dark: "#D4D4D4".to_string(),
                border_link_hover: "#B3B3B3".to_string(),
                icon: "#6B6B6B".to_string(),
                reply_line: "#E5E5E5".to_string(),
                reply_line_dot: "#D4D4D4".to_string(),
                unread_notif_bg: "#F5F0FF".to_string(),  // Light purple
                unread_notif_border: "#D9C9FF".to_string(),
                post_ctrl: "#6B6B6B".to_string(),
                brand_text: "#9D4EDD".to_string(),  // Aurora purple
                empty_state_icon: "#B3B3B3".to_string(),
            },
            primary: ColorSet {
                background: "#9D4EDD".to_string(),  // Aurora purple
                background_light: "#B392FF".to_string(),
                text: "#FFFFFF".to_string(),
                text_light: "#EDE4FF".to_string(),
                text_inverted: "#9D4EDD".to_string(),
                link: "#EDE4FF".to_string(),
                border: "#8A3DC7".to_string(),
                border_dark: "#752FB0".to_string(),
                icon: "#8A3DC7".to_string(),
            },
            secondary: ColorSet {
                background: "#06FFA5".to_string(),  // Aurora cyan
                background_light: "#33FFAD".to_string(),
                text: "#000000".to_string(),
                text_light: "#014030".to_string(),
                text_inverted: "#06FFA5".to_string(),
                link: "#014030".to_string(),
                border: "#05D98D".to_string(),
                border_dark: "#04B376".to_string(),
                icon: "#038C5E".to_string(),
            },
            inverted: ColorSet {
                background: "#0A0F1A".to_string(),  // Dark background
                background_light: "#1A2332".to_string(),
                text: "#FFFFFF".to_string(),
                text_light: "#B3B3B3".to_string(),
                text_inverted: "#000000".to_string(),
                link: "#B392FF".to_string(),
                border: "#3D3D3D".to_string(),
                border_dark: "#2E2E2E".to_string(),
                icon: "#6B6B6B".to_string(),
            },
            error: ColorSet {
                background: "#EF4444".to_string(),
                background_light: "#FFA3A3".to_string(),
                text: "#FFFFFF".to_string(),
                text_light: "#FFE0E0".to_string(),
                text_inverted: "#EF4444".to_string(),
                link: "#FFE0E0".to_string(),
                border: "#DC2626".to_string(),
                border_dark: "#B91C1C".to_string(),
                icon: "#DC2626".to_string(),
            },
        },
        gradients: Gradients::default(),
    }
}

// =============================================================================
// Dark Theme
// =============================================================================

/// Create the dark theme
pub fn dark_theme() -> Theme {
    Theme {
        name: ThemeName::Dark,
        color_scheme: "dark".to_string(),
        palette: Palette {
            white: "#FFFFFF".to_string(),
            black: "#000000".to_string(),
            contrast: ContrastPalette {
                contrast_0: "#0A0F1A".to_string(),   // Near black (aurora night sky)
                contrast_25: "#111827".to_string(),
                contrast_50: "#1A2332".to_string(),
                contrast_100: "#243044".to_string(),
                contrast_200: "#2E3D55".to_string(),
                contrast_300: "#3D4F6A".to_string(),
                contrast_400: "#4F6380".to_string(),
                contrast_500: "#647896".to_string(),
                contrast_600: "#7B8DAA".to_string(),
                contrast_700: "#96A5BC".to_string(),
                contrast_800: "#B3BFCE".to_string(),
                contrast_900: "#D1D9E3".to_string(),
                contrast_950: "#E8ECF1".to_string(),
                contrast_975: "#F5F7F9".to_string(),
            },
            primary: PrimaryPalette {
                // Aurora purple/blue - adjusted for dark theme
                primary_25: "#1A102A".to_string(),
                primary_50: "#251538".to_string(),
                primary_100: "#351F4F".to_string(),
                primary_200: "#4A2C6E".to_string(),
                primary_300: "#63398F".to_string(),
                primary_400: "#8042B8".to_string(),
                primary_500: "#9D4EDD".to_string(),  // Aurora purple (brand)
                primary_600: "#B06BE8".to_string(),
                primary_700: "#C38AF0".to_string(),
                primary_800: "#D4A9F5".to_string(),
                primary_900: "#E4C8FA".to_string(),
                primary_950: "#F0E0FC".to_string(),
                primary_975: "#F8F0FE".to_string(),
            },
            positive: PositivePalette {
                // Aurora cyan/green for dark
                positive_25: "#031F15".to_string(),
                positive_50: "#052E20".to_string(),
                positive_100: "#0A4D36".to_string(),
                positive_200: "#0F6C4B".to_string(),
                positive_300: "#148A61".to_string(),
                positive_400: "#19A876".to_string(),
                positive_500: "#06FFA5".to_string(),  // Aurora cyan
                positive_600: "#38FFB8".to_string(),
                positive_700: "#6AFFCB".to_string(),
                positive_800: "#9CFFDD".to_string(),
                positive_900: "#CEFFEE".to_string(),
                positive_950: "#E7FFF7".to_string(),
                positive_975: "#F3FFFB".to_string(),
            },
            negative: NegativePalette {
                // Error colors for dark
                negative_25: "#1F0A0A".to_string(),
                negative_50: "#2D0F0F".to_string(),
                negative_100: "#4A1919".to_string(),
                negative_200: "#662323".to_string(),
                negative_300: "#832D2D".to_string(),
                negative_400: "#A03737".to_string(),
                negative_500: "#EF4444".to_string(),
                negative_600: "#F26969".to_string(),
                negative_700: "#F58D8D".to_string(),
                negative_800: "#F8B2B2".to_string(),
                negative_900: "#FBD6D6".to_string(),
                negative_950: "#FDEBEB".to_string(),
                negative_975: "#FEF5F5".to_string(),
            },
        },
        colors: ThemeColors {
            default: SemanticColors {
                background: "#0A0F1A".to_string(),
                background_light: "#111827".to_string(),
                text: "#FFFFFF".to_string(),
                text_light: "#96A5BC".to_string(),
                text_very_light: "#647896".to_string(),
                text_inverted: "#000000".to_string(),
                link: "#B06BE8".to_string(),  // Lighter purple for dark
                border: "#243044".to_string(),
                border_dark: "#2E3D55".to_string(),
                border_link_hover: "#3D4F6A".to_string(),
                icon: "#647896".to_string(),
                reply_line: "#2E3D55".to_string(),
                reply_line_dot: "#2E3D55".to_string(),
                unread_notif_bg: "#1A102A".to_string(),
                unread_notif_border: "#351F4F".to_string(),
                post_ctrl: "#647896".to_string(),
                brand_text: "#B06BE8".to_string(),
                empty_state_icon: "#3D4F6A".to_string(),
            },
            primary: ColorSet {
                background: "#9D4EDD".to_string(),
                background_light: "#B06BE8".to_string(),
                text: "#FFFFFF".to_string(),
                text_light: "#E4C8FA".to_string(),
                text_inverted: "#B06BE8".to_string(),
                link: "#E4C8FA".to_string(),
                border: "#8042B8".to_string(),
                border_dark: "#63398F".to_string(),
                icon: "#8042B8".to_string(),
            },
            secondary: ColorSet {
                background: "#06FFA5".to_string(),
                background_light: "#38FFB8".to_string(),
                text: "#000000".to_string(),
                text_light: "#031F15".to_string(),
                text_inverted: "#38FFB8".to_string(),
                link: "#031F15".to_string(),
                border: "#19A876".to_string(),
                border_dark: "#148A61".to_string(),
                icon: "#0F6C4B".to_string(),
            },
            inverted: ColorSet {
                background: "#FFFFFF".to_string(),
                background_light: "#F5F7F9".to_string(),
                text: "#000000".to_string(),
                text_light: "#3D4F6A".to_string(),
                text_inverted: "#FFFFFF".to_string(),
                link: "#9D4EDD".to_string(),
                border: "#E8ECF1".to_string(),
                border_dark: "#D1D9E3".to_string(),
                icon: "#647896".to_string(),
            },
            error: ColorSet {
                background: "#EF4444".to_string(),
                background_light: "#F26969".to_string(),
                text: "#FFFFFF".to_string(),
                text_light: "#FBD6D6".to_string(),
                text_inverted: "#EF4444".to_string(),
                link: "#FBD6D6".to_string(),
                border: "#A03737".to_string(),
                border_dark: "#832D2D".to_string(),
                icon: "#A03737".to_string(),
            },
        },
        gradients: Gradients::default(),
    }
}

// =============================================================================
// Dim Theme
// =============================================================================

/// Create the dim theme (softer dark theme)
pub fn dim_theme() -> Theme {
    Theme {
        name: ThemeName::Dim,
        color_scheme: "dark".to_string(),
        palette: Palette {
            white: "#FFFFFF".to_string(),
            black: "#000000".to_string(),
            contrast: ContrastPalette {
                contrast_0: "#1A2332".to_string(),   // Dim background (softer than dark)
                contrast_25: "#212D3F".to_string(),
                contrast_50: "#2A384C".to_string(),
                contrast_100: "#344459".to_string(),
                contrast_200: "#3F5167".to_string(),
                contrast_300: "#4D6078".to_string(),
                contrast_400: "#5D7189".to_string(),
                contrast_500: "#6E829A".to_string(),
                contrast_600: "#8295AB".to_string(),
                contrast_700: "#99A9BA".to_string(),
                contrast_800: "#B3BFCE".to_string(),
                contrast_900: "#CDD5DE".to_string(),
                contrast_950: "#E5EAEF".to_string(),
                contrast_975: "#F2F5F7".to_string(),
            },
            primary: PrimaryPalette {
                // Aurora purple adjusted for dim
                primary_25: "#1E1430".to_string(),
                primary_50: "#2A1B40".to_string(),
                primary_100: "#3A2558".to_string(),
                primary_200: "#503376".to_string(),
                primary_300: "#684296".to_string(),
                primary_400: "#8348B5".to_string(),
                primary_500: "#9D4EDD".to_string(),
                primary_600: "#AD68E5".to_string(),
                primary_700: "#BE85EC".to_string(),
                primary_800: "#CEA3F2".to_string(),
                primary_900: "#DFC1F8".to_string(),
                primary_950: "#EFE0FB".to_string(),
                primary_975: "#F7F0FD".to_string(),
            },
            positive: PositivePalette {
                positive_25: "#0A2519".to_string(),
                positive_50: "#0F3322".to_string(),
                positive_100: "#17523A".to_string(),
                positive_200: "#1F7052".to_string(),
                positive_300: "#288F6A".to_string(),
                positive_400: "#30AD82".to_string(),
                positive_500: "#06FFA5".to_string(),
                positive_600: "#3DFFB5".to_string(),
                positive_700: "#6DFFC5".to_string(),
                positive_800: "#9EFFD5".to_string(),
                positive_900: "#CEFFE5".to_string(),
                positive_950: "#E7FFF2".to_string(),
                positive_975: "#F3FFF9".to_string(),
            },
            negative: NegativePalette {
                negative_25: "#250F0F".to_string(),
                negative_50: "#331414".to_string(),
                negative_100: "#4F1E1E".to_string(),
                negative_200: "#6B2828".to_string(),
                negative_300: "#873232".to_string(),
                negative_400: "#A33C3C".to_string(),
                negative_500: "#EF4444".to_string(),
                negative_600: "#F16C6C".to_string(),
                negative_700: "#F39393".to_string(),
                negative_800: "#F6BBBB".to_string(),
                negative_900: "#F9E2E2".to_string(),
                negative_950: "#FCF0F0".to_string(),
                negative_975: "#FEF8F8".to_string(),
            },
        },
        colors: ThemeColors {
            default: SemanticColors {
                background: "#1A2332".to_string(),
                background_light: "#212D3F".to_string(),
                text: "#FFFFFF".to_string(),
                text_light: "#99A9BA".to_string(),
                text_very_light: "#6E829A".to_string(),
                text_inverted: "#000000".to_string(),
                link: "#AD68E5".to_string(),
                border: "#344459".to_string(),
                border_dark: "#3F5167".to_string(),
                border_link_hover: "#4D6078".to_string(),
                icon: "#6E829A".to_string(),
                reply_line: "#3F5167".to_string(),
                reply_line_dot: "#3F5167".to_string(),
                unread_notif_bg: "#1E1430".to_string(),
                unread_notif_border: "#3A2558".to_string(),
                post_ctrl: "#6E829A".to_string(),
                brand_text: "#AD68E5".to_string(),
                empty_state_icon: "#4D6078".to_string(),
            },
            primary: ColorSet {
                background: "#9D4EDD".to_string(),
                background_light: "#AD68E5".to_string(),
                text: "#FFFFFF".to_string(),
                text_light: "#DFC1F8".to_string(),
                text_inverted: "#AD68E5".to_string(),
                link: "#DFC1F8".to_string(),
                border: "#8348B5".to_string(),
                border_dark: "#684296".to_string(),
                icon: "#8348B5".to_string(),
            },
            secondary: ColorSet {
                background: "#06FFA5".to_string(),
                background_light: "#3DFFB5".to_string(),
                text: "#000000".to_string(),
                text_light: "#0A2519".to_string(),
                text_inverted: "#3DFFB5".to_string(),
                link: "#0A2519".to_string(),
                border: "#30AD82".to_string(),
                border_dark: "#288F6A".to_string(),
                icon: "#1F7052".to_string(),
            },
            inverted: ColorSet {
                background: "#FFFFFF".to_string(),
                background_light: "#F2F5F7".to_string(),
                text: "#000000".to_string(),
                text_light: "#4D6078".to_string(),
                text_inverted: "#FFFFFF".to_string(),
                link: "#9D4EDD".to_string(),
                border: "#E5EAEF".to_string(),
                border_dark: "#CDD5DE".to_string(),
                icon: "#6E829A".to_string(),
            },
            error: ColorSet {
                background: "#EF4444".to_string(),
                background_light: "#F16C6C".to_string(),
                text: "#FFFFFF".to_string(),
                text_light: "#F9E2E2".to_string(),
                text_inverted: "#EF4444".to_string(),
                link: "#F9E2E2".to_string(),
                border: "#A33C3C".to_string(),
                border_dark: "#873232".to_string(),
                icon: "#A33C3C".to_string(),
            },
        },
        gradients: Gradients::default(),
    }
}

// =============================================================================
// Theme Provider
// =============================================================================

/// Get a theme by name
pub fn get_theme(name: ThemeName) -> Theme {
    match name {
        ThemeName::Light => light_theme(),
        ThemeName::Dark => dark_theme(),
        ThemeName::Dim => dim_theme(),
    }
}

/// All available themes
pub fn all_themes() -> HashMap<ThemeName, Theme> {
    let mut themes = HashMap::new();
    themes.insert(ThemeName::Light, light_theme());
    themes.insert(ThemeName::Dark, dark_theme());
    themes.insert(ThemeName::Dim, dim_theme());
    themes
}

/// Theme configuration for font preferences
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FontConfig {
    /// Font scale multiplier (0.8 - 1.4)
    pub scale: f32,
    /// Font family preference
    pub family: FontFamily,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            scale: 1.0,
            family: FontFamily::System,
        }
    }
}

/// Available font families
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FontFamily {
    /// System default font
    #[default]
    System,
    /// Inter font
    Inter,
    /// SF Pro (iOS/macOS)
    SfPro,
    /// Roboto (Android)
    Roboto,
}

/// Theme provider state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeState {
    /// Current theme name
    pub theme_name: ThemeName,
    /// Current theme (regenerated on deserialization)
    #[serde(skip, default = "light_theme")]
    pub theme: Theme,
    /// Font configuration
    pub fonts: FontConfig,
}

impl Default for ThemeState {
    fn default() -> Self {
        Self {
            theme_name: ThemeName::Light,
            theme: light_theme(),
            fonts: FontConfig::default(),
        }
    }
}

impl ThemeState {
    /// Create a new theme state with the given theme
    pub fn new(theme_name: ThemeName) -> Self {
        Self {
            theme_name,
            theme: get_theme(theme_name),
            fonts: FontConfig::default(),
        }
    }

    /// Set the current theme
    pub fn set_theme(&mut self, theme_name: ThemeName) {
        self.theme_name = theme_name;
        self.theme = get_theme(theme_name);
    }

    /// Set font scale
    pub fn set_font_scale(&mut self, scale: f32) {
        self.fonts.scale = scale.clamp(0.8, 1.4);
    }

    /// Set font family
    pub fn set_font_family(&mut self, family: FontFamily) {
        self.fonts.family = family;
    }

    /// Get the current theme
    pub fn current_theme(&self) -> &Theme {
        &self.theme
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Color Utility Tests
    // ==========================================================================

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("#FFFFFF"), Some((255, 255, 255)));
        assert_eq!(parse_hex_color("#000000"), Some((0, 0, 0)));
        assert_eq!(parse_hex_color("#9D4EDD"), Some((157, 78, 221)));
        assert_eq!(parse_hex_color("#06FFA5"), Some((6, 255, 165)));
        assert_eq!(parse_hex_color("FFFFFF"), Some((255, 255, 255)));
        assert_eq!(parse_hex_color("#FF"), None); // Too short
    }

    #[test]
    fn test_rgb_to_hex() {
        assert_eq!(rgb_to_hex(255, 255, 255), "#FFFFFF");
        assert_eq!(rgb_to_hex(0, 0, 0), "#000000");
        assert_eq!(rgb_to_hex(157, 78, 221), "#9D4EDD");
    }

    // ==========================================================================
    // Theme Name Tests
    // ==========================================================================

    #[test]
    fn test_theme_name_display() {
        assert_eq!(ThemeName::Light.to_string(), "Light");
        assert_eq!(ThemeName::Dark.to_string(), "Dark");
        assert_eq!(ThemeName::Dim.to_string(), "Dim");
    }

    #[test]
    fn test_theme_name_from_str() {
        assert_eq!("light".parse::<ThemeName>().unwrap(), ThemeName::Light);
        assert_eq!("dark".parse::<ThemeName>().unwrap(), ThemeName::Dark);
        assert_eq!("dim".parse::<ThemeName>().unwrap(), ThemeName::Dim);
        assert_eq!("LIGHT".parse::<ThemeName>().unwrap(), ThemeName::Light);
        assert!("invalid".parse::<ThemeName>().is_err());
    }

    #[test]
    fn test_theme_name_color_scheme() {
        assert_eq!(ThemeName::Light.color_scheme(), "light");
        assert_eq!(ThemeName::Dark.color_scheme(), "dark");
        assert_eq!(ThemeName::Dim.color_scheme(), "dim");
    }

    // ==========================================================================
    // Light Theme Tests
    // ==========================================================================

    #[test]
    fn test_light_theme_basics() {
        let theme = light_theme();
        assert_eq!(theme.name, ThemeName::Light);
        assert_eq!(theme.color_scheme, "light");
        assert!(!theme.is_dark());
    }

    #[test]
    fn test_light_theme_palette() {
        let theme = light_theme();
        assert_eq!(theme.palette.white, "#FFFFFF");
        assert_eq!(theme.palette.black, "#000000");
        assert_eq!(theme.palette.primary.primary_500, "#9D4EDD"); // Aurora purple
        assert_eq!(theme.palette.positive.positive_500, "#06FFA5"); // Aurora cyan
    }

    #[test]
    fn test_light_theme_colors() {
        let theme = light_theme();
        assert_eq!(theme.colors.default.background, "#FFFFFF");
        assert_eq!(theme.colors.default.text, "#000000");
        assert_eq!(theme.colors.default.link, "#9D4EDD"); // Aurora purple
        assert_eq!(theme.colors.primary.background, "#9D4EDD");
        assert_eq!(theme.colors.secondary.background, "#06FFA5");
    }

    #[test]
    fn test_light_theme_contrast() {
        let theme = light_theme();
        assert_eq!(theme.contrast(0), "#FFFFFF");
        assert_eq!(theme.contrast(500), "#6B6B6B");
        assert_eq!(theme.contrast(975), "#0A0A0A");
    }

    // ==========================================================================
    // Dark Theme Tests
    // ==========================================================================

    #[test]
    fn test_dark_theme_basics() {
        let theme = dark_theme();
        assert_eq!(theme.name, ThemeName::Dark);
        assert_eq!(theme.color_scheme, "dark");
        assert!(theme.is_dark());
    }

    #[test]
    fn test_dark_theme_palette() {
        let theme = dark_theme();
        assert_eq!(theme.palette.contrast.contrast_0, "#0A0F1A"); // Near black
        assert_eq!(theme.palette.primary.primary_500, "#9D4EDD"); // Aurora purple
    }

    #[test]
    fn test_dark_theme_colors() {
        let theme = dark_theme();
        assert_eq!(theme.colors.default.background, "#0A0F1A");
        assert_eq!(theme.colors.default.text, "#FFFFFF");
        // Inverted colors should be light in dark theme
        assert_eq!(theme.colors.inverted.background, "#FFFFFF");
        assert_eq!(theme.colors.inverted.text, "#000000");
    }

    // ==========================================================================
    // Dim Theme Tests
    // ==========================================================================

    #[test]
    fn test_dim_theme_basics() {
        let theme = dim_theme();
        assert_eq!(theme.name, ThemeName::Dim);
        assert_eq!(theme.color_scheme, "dark"); // Dim is still a dark scheme
        assert!(theme.is_dark());
    }

    #[test]
    fn test_dim_theme_softer_than_dark() {
        let dark = dark_theme();
        let dim = dim_theme();

        // Dim should have a lighter background than dark
        let dark_bg = parse_hex_color(&dark.colors.default.background).unwrap();
        let dim_bg = parse_hex_color(&dim.colors.default.background).unwrap();

        // Dim background should be lighter (higher RGB values)
        assert!(dim_bg.0 > dark_bg.0 || dim_bg.1 > dark_bg.1 || dim_bg.2 > dark_bg.2);
    }

    // ==========================================================================
    // Gradient Tests
    // ==========================================================================

    #[test]
    fn test_gradients_default() {
        let gradients = Gradients::default();

        // Primary gradient should use aurora colors
        assert_eq!(gradients.primary.stops.len(), 4);
        assert_eq!(gradients.primary.stops[0].color, "#9D4EDD"); // Purple
        assert_eq!(gradients.primary.stops[3].color, "#06FFA5"); // Cyan

        // Aurora gradient
        assert_eq!(gradients.aurora.stops.len(), 3);

        // Compass gradient should use navy and gold
        assert!(gradients.compass.stops[0].color.contains("1E3A5F")); // Navy
        assert!(gradients.compass.stops[2].color.contains("FFB703")); // Gold
    }

    #[test]
    fn test_gradient_stops_valid_positions() {
        let gradients = Gradients::default();

        for gradient in [
            &gradients.primary,
            &gradients.aurora,
            &gradients.midnight,
            &gradients.sunrise,
            &gradients.sunset,
            &gradients.northern,
            &gradients.compass,
        ] {
            for stop in &gradient.stops {
                assert!(stop.position >= 0.0 && stop.position <= 1.0);
            }
        }
    }

    // ==========================================================================
    // Theme Provider Tests
    // ==========================================================================

    #[test]
    fn test_get_theme() {
        let light = get_theme(ThemeName::Light);
        assert_eq!(light.name, ThemeName::Light);

        let dark = get_theme(ThemeName::Dark);
        assert_eq!(dark.name, ThemeName::Dark);

        let dim = get_theme(ThemeName::Dim);
        assert_eq!(dim.name, ThemeName::Dim);
    }

    #[test]
    fn test_all_themes() {
        let themes = all_themes();
        assert_eq!(themes.len(), 3);
        assert!(themes.contains_key(&ThemeName::Light));
        assert!(themes.contains_key(&ThemeName::Dark));
        assert!(themes.contains_key(&ThemeName::Dim));
    }

    // ==========================================================================
    // Theme State Tests
    // ==========================================================================

    #[test]
    fn test_theme_state_default() {
        let state = ThemeState::default();
        assert_eq!(state.theme_name, ThemeName::Light);
        assert_eq!(state.fonts.scale, 1.0);
        assert_eq!(state.fonts.family, FontFamily::System);
    }

    #[test]
    fn test_theme_state_set_theme() {
        let mut state = ThemeState::default();
        state.set_theme(ThemeName::Dark);
        assert_eq!(state.theme_name, ThemeName::Dark);
        assert!(state.theme.is_dark());
    }

    #[test]
    fn test_theme_state_font_scale() {
        let mut state = ThemeState::default();

        state.set_font_scale(1.2);
        assert_eq!(state.fonts.scale, 1.2);

        // Test clamping
        state.set_font_scale(0.5);
        assert_eq!(state.fonts.scale, 0.8);

        state.set_font_scale(2.0);
        assert_eq!(state.fonts.scale, 1.4);
    }

    #[test]
    fn test_theme_state_font_family() {
        let mut state = ThemeState::default();
        state.set_font_family(FontFamily::Inter);
        assert_eq!(state.fonts.family, FontFamily::Inter);
    }

    // ==========================================================================
    // Serialization Tests
    // ==========================================================================

    #[test]
    fn test_theme_name_serialization() {
        let name = ThemeName::Dark;
        let json = serde_json::to_string(&name).unwrap();
        assert_eq!(json, "\"dark\"");

        let deserialized: ThemeName = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ThemeName::Dark);
    }

    #[test]
    fn test_font_config_serialization() {
        let config = FontConfig {
            scale: 1.2,
            family: FontFamily::Inter,
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: FontConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.scale, 1.2);
        assert_eq!(deserialized.family, FontFamily::Inter);
    }

    #[test]
    fn test_gradient_serialization() {
        let gradient = Gradient::new(
            vec![(0.0, "#FF0000"), (1.0, "#00FF00")],
            "#FFFF00",
        );

        let json = serde_json::to_string(&gradient).unwrap();
        let deserialized: Gradient = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.stops.len(), 2);
        assert_eq!(deserialized.hover_color, "#FFFF00");
    }

    // ==========================================================================
    // Color Consistency Tests
    // ==========================================================================

    #[test]
    fn test_all_colors_are_valid_hex() {
        for (name, theme) in all_themes() {
            // Test palette colors
            assert!(
                parse_hex_color(&theme.palette.white).is_some(),
                "Invalid white color in {:?} theme",
                name
            );
            assert!(
                parse_hex_color(&theme.palette.black).is_some(),
                "Invalid black color in {:?} theme",
                name
            );

            // Test contrast colors
            for level in [0, 25, 50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950, 975] {
                assert!(
                    parse_hex_color(theme.contrast(level)).is_some(),
                    "Invalid contrast_{} color in {:?} theme",
                    level,
                    name
                );
            }

            // Test semantic colors
            assert!(
                parse_hex_color(&theme.colors.default.background).is_some(),
                "Invalid default background in {:?} theme",
                name
            );
            assert!(
                parse_hex_color(&theme.colors.default.text).is_some(),
                "Invalid default text in {:?} theme",
                name
            );
            assert!(
                parse_hex_color(&theme.colors.default.link).is_some(),
                "Invalid default link in {:?} theme",
                name
            );
        }
    }

    #[test]
    fn test_brand_colors_consistency() {
        // All themes should use the same brand colors for primary/secondary
        let light = light_theme();
        let dark = dark_theme();
        let dim = dim_theme();

        // Primary_500 should always be Aurora purple
        assert_eq!(light.palette.primary.primary_500, "#9D4EDD");
        assert_eq!(dark.palette.primary.primary_500, "#9D4EDD");
        assert_eq!(dim.palette.primary.primary_500, "#9D4EDD");

        // Positive_500 should always be Aurora cyan
        assert_eq!(light.palette.positive.positive_500, "#06FFA5");
        assert_eq!(dark.palette.positive.positive_500, "#06FFA5");
        assert_eq!(dim.palette.positive.positive_500, "#06FFA5");
    }

    // ==========================================================================
    // Accessibility Contrast Tests
    // ==========================================================================

    #[test]
    fn test_text_background_contrast() {
        // Basic check that text is readable against background
        for (name, theme) in all_themes() {
            let bg = parse_hex_color(&theme.colors.default.background).unwrap();
            let text = parse_hex_color(&theme.colors.default.text).unwrap();

            // Calculate rough luminance difference
            let bg_lum = (bg.0 as u32 + bg.1 as u32 + bg.2 as u32) / 3;
            let text_lum = (text.0 as u32 + text.1 as u32 + text.2 as u32) / 3;

            let diff = if bg_lum > text_lum {
                bg_lum - text_lum
            } else {
                text_lum - bg_lum
            };

            // Ensure minimum contrast
            assert!(
                diff > 100,
                "{:?} theme has insufficient text contrast: bg_lum={}, text_lum={}, diff={}",
                name,
                bg_lum,
                text_lum,
                diff
            );
        }
    }
}
