//! User interface for Aurora Compass
//!
//! This crate provides the UI layer, including components,
//! screens, navigation, theming, and design system primitives.
//!
//! # Design System
//!
//! The design system is built around Aurora borealis colors:
//! - Primary: Aurora purple (#9D4EDD)
//! - Secondary: Aurora cyan (#06FFA5)
//! - Accent: Compass gold (#FFB703)
//!
//! Three themes are supported:
//! - [`theme::ThemeName::Light`] - Bright theme with white background
//! - [`theme::ThemeName::Dark`] - Dark theme with near-black background
//! - [`theme::ThemeName::Dim`] - Softer dark theme
//!
//! # Modules
//!
//! - [`theme`] - Theme provider, color palettes, and gradients
//! - [`tokens`] - Design tokens (spacing, sizing, breakpoints, etc.)
//! - [`typography`] - Typography system and text styles
//! - [`components`] - UI component library
//! - [`screens`] - Application screens
//! - [`navigation`] - Navigation framework
//!
//! # Example
//!
//! ```rust
//! use app_ui::theme::{ThemeName, get_theme, ThemeState};
//! use app_ui::tokens::spacing;
//! use app_ui::typography::{Typography, TypographyVariant};
//!
//! // Get a theme
//! let theme = get_theme(ThemeName::Dark);
//! assert!(theme.is_dark());
//!
//! // Use design tokens
//! let padding = spacing::SPACE_MD;
//!
//! // Get typography styles
//! let typo = Typography::default();
//! let title_style = typo.get(TypographyVariant::Title);
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod components;
pub mod navigation;
pub mod screens;
pub mod theme;
pub mod tokens;
pub mod typography;

// Re-export commonly used types
pub use theme::{
    get_theme, all_themes, light_theme, dark_theme, dim_theme,
    Theme, ThemeName, ThemeState, ThemeColors, Palette,
    FontConfig, FontFamily, Gradient, Gradients,
};

pub use tokens::{
    spacing, sizing, radius, border, shadows, breakpoints,
    duration, easing, z_index, content_width, aspect_ratio,
    Shadow, Breakpoint,
};

pub use typography::{
    font_size, Typography, TypographyVariant, TextStyle,
    FontStack, TextTransform, TextDecoration,
};

pub use navigation::{
    Route, RouteParams, Router, SearchTab,
    NavigationTab, NavigationStack, NavigationState,
    NavigationAnimation, StackEntry, PendingNavigation,
};
