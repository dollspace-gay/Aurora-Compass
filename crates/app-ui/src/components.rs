//! UI component library for Aurora Compass
//!
//! This module provides foundational UI components that mirror
//! the Bluesky client's component patterns, adapted for Rust/Tauri.
//!
//! # Component Design
//!
//! Components are defined as Rust structs with serializable properties
//! that can be rendered by the frontend (Tauri webview). Each component
//! provides:
//!
//! - Type-safe props with builder patterns
//! - Theme-aware styling through the theme system
//! - Accessibility attributes
//! - Event handling hooks
//!
//! # Available Components
//!
//! - [`Button`] - Interactive button with multiple variants
//! - [`Text`] - Typography component with semantic variants
//! - [`Container`] - Layout container with flex properties
//! - [`Input`] - Text input with validation support
//! - [`Icon`] - SVG icon component

use crate::theme::{Color, Theme};
use crate::tokens::{radius, sizing};
use crate::typography::TypographyVariant;
use serde::{Deserialize, Serialize};

// =============================================================================
// Common Types
// =============================================================================

/// Component identifier
pub type ComponentId = String;

/// Event handler callback type (represented as a string identifier)
pub type EventHandler = String;

/// Style properties that can be applied to any component
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StyleProps {
    /// Margin around the component
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin: Option<Spacing>,
    /// Padding inside the component
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding: Option<Spacing>,
    /// Width constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<Dimension>,
    /// Height constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<Dimension>,
    /// Minimum width
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_width: Option<Dimension>,
    /// Minimum height
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_height: Option<Dimension>,
    /// Maximum width
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_width: Option<Dimension>,
    /// Maximum height
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_height: Option<Dimension>,
    /// Background color
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<Color>,
    /// Border radius
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_radius: Option<f32>,
    /// Border width
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_width: Option<f32>,
    /// Border color
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_color: Option<Color>,
    /// Opacity (0.0 - 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f32>,
    /// Flex grow factor
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flex_grow: Option<f32>,
    /// Flex shrink factor
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flex_shrink: Option<f32>,
    /// Flex basis
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flex_basis: Option<Dimension>,
    /// Align self
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align_self: Option<Alignment>,
    /// Custom CSS class
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
}

/// Spacing values (margin, padding)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Spacing {
    /// Uniform spacing on all sides
    Uniform(f32),
    /// Vertical and horizontal spacing
    Symmetric {
        /// Vertical spacing
        vertical: f32,
        /// Horizontal spacing
        horizontal: f32,
    },
    /// Individual spacing per side
    Individual {
        /// Top spacing
        top: f32,
        /// Right spacing
        right: f32,
        /// Bottom spacing
        bottom: f32,
        /// Left spacing
        left: f32,
    },
}

impl Default for Spacing {
    fn default() -> Self {
        Spacing::Uniform(0.0)
    }
}

impl Spacing {
    /// Create uniform spacing
    pub fn uniform(value: f32) -> Self {
        Spacing::Uniform(value)
    }

    /// Create symmetric spacing
    pub fn symmetric(vertical: f32, horizontal: f32) -> Self {
        Spacing::Symmetric {
            vertical,
            horizontal,
        }
    }

    /// Create individual spacing
    pub fn individual(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Spacing::Individual {
            top,
            right,
            bottom,
            left,
        }
    }
}

/// Dimension value (pixels, percentage, auto)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(untagged)]
pub enum Dimension {
    /// Fixed pixel value
    Pixels(f32),
    /// Percentage of parent
    Percent(String),
    /// Auto-size
    #[default]
    Auto,
}

impl Dimension {
    /// Create a pixel dimension
    pub fn px(value: f32) -> Self {
        Dimension::Pixels(value)
    }

    /// Create a percentage dimension
    pub fn percent(value: f32) -> Self {
        Dimension::Percent(format!("{}%", value))
    }

    /// Create an auto dimension
    pub fn auto() -> Self {
        Dimension::Auto
    }
}

/// Alignment options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Alignment {
    /// Stretch to fill
    #[default]
    Stretch,
    /// Align to start
    Start,
    /// Align to center
    Center,
    /// Align to end
    End,
    /// Baseline alignment
    Baseline,
}

/// Justify content options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JustifyContent {
    /// Start (default)
    #[default]
    Start,
    /// Center
    Center,
    /// End
    End,
    /// Space between
    SpaceBetween,
    /// Space around
    SpaceAround,
    /// Space evenly
    SpaceEvenly,
}

/// Flex direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlexDirection {
    /// Row (horizontal)
    #[default]
    Row,
    /// Column (vertical)
    Column,
    /// Row reversed
    RowReverse,
    /// Column reversed
    ColumnReverse,
}

/// Accessibility properties
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AccessibilityProps {
    /// Accessible label for screen readers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Accessible hint/description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// ARIA role
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Whether the element is disabled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    /// Whether the element is hidden from accessibility tree
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
}

// =============================================================================
// Button Component
// =============================================================================

/// Button style variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ButtonVariant {
    /// Solid background button
    #[default]
    Solid,
    /// Outlined button with border
    Outline,
    /// Ghost button with no background
    Ghost,
}

/// Button color schemes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ButtonColor {
    /// Primary brand color (aurora purple)
    #[default]
    Primary,
    /// Secondary/neutral color
    Secondary,
    /// Inverted secondary
    SecondaryInverted,
    /// Negative/destructive action
    Negative,
    /// Subtle primary
    PrimarySubtle,
    /// Subtle negative
    NegativeSubtle,
}

/// Button sizes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ButtonSize {
    /// Tiny button (25-33px)
    Tiny,
    /// Small button (33-40px)
    Small,
    /// Large button (44px+)
    #[default]
    Large,
}

/// Button shape
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ButtonShape {
    /// Default rounded rectangle
    #[default]
    Default,
    /// Fully round (circular if square dimensions)
    Round,
    /// Square with small radius
    Square,
}

/// Button component properties
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Button {
    /// Unique component ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<ComponentId>,
    /// Accessible label (required for accessibility)
    pub label: String,
    /// Button style variant
    #[serde(default)]
    pub variant: ButtonVariant,
    /// Button color scheme
    #[serde(default)]
    pub color: ButtonColor,
    /// Button size
    #[serde(default)]
    pub size: ButtonSize,
    /// Button shape
    #[serde(default)]
    pub shape: ButtonShape,
    /// Whether the button is disabled
    #[serde(default)]
    pub disabled: bool,
    /// Whether the button is loading
    #[serde(default)]
    pub loading: bool,
    /// On press event handler
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_press: Option<EventHandler>,
    /// On long press event handler
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_long_press: Option<EventHandler>,
    /// Icon name to display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Icon position
    #[serde(default)]
    pub icon_position: IconPosition,
    /// Child text content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Additional style props
    #[serde(default, skip_serializing_if = "is_default_style")]
    pub style: StyleProps,
    /// Test ID for testing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_id: Option<String>,
}

fn is_default_style(style: &StyleProps) -> bool {
    style == &StyleProps::default()
}

/// Icon position in button
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IconPosition {
    /// Icon on the left
    #[default]
    Left,
    /// Icon on the right
    Right,
    /// Icon only (no text)
    Only,
}

impl Button {
    /// Create a new button with the given label
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            id: None,
            label: label.into(),
            variant: ButtonVariant::default(),
            color: ButtonColor::default(),
            size: ButtonSize::default(),
            shape: ButtonShape::default(),
            disabled: false,
            loading: false,
            on_press: None,
            on_long_press: None,
            icon: None,
            icon_position: IconPosition::default(),
            text: None,
            style: StyleProps::default(),
            test_id: None,
        }
    }

    /// Set the button ID
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the button variant
    pub fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the button color
    pub fn with_color(mut self, color: ButtonColor) -> Self {
        self.color = color;
        self
    }

    /// Set the button size
    pub fn with_size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Set the button shape
    pub fn with_shape(mut self, shape: ButtonShape) -> Self {
        self.shape = shape;
        self
    }

    /// Set disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set loading state
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Set on press handler
    pub fn on_press(mut self, handler: impl Into<String>) -> Self {
        self.on_press = Some(handler.into());
        self
    }

    /// Set icon
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set icon position
    pub fn with_icon_position(mut self, position: IconPosition) -> Self {
        self.icon_position = position;
        self
    }

    /// Set button text
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Set custom style
    pub fn with_style(mut self, style: StyleProps) -> Self {
        self.style = style;
        self
    }

    /// Get the computed styles for this button based on theme
    pub fn computed_styles(&self, theme: &Theme) -> ButtonStyles {
        let (background, background_hover, text_color) = match (self.variant, self.color) {
            (ButtonVariant::Solid, ButtonColor::Primary) => {
                if self.disabled {
                    (
                        theme.palette.primary.primary_200.clone(),
                        theme.palette.primary.primary_200.clone(),
                        theme.palette.white.clone(),
                    )
                } else {
                    (
                        theme.palette.primary.primary_500.clone(),
                        theme.palette.primary.primary_600.clone(),
                        theme.palette.white.clone(),
                    )
                }
            }
            (ButtonVariant::Solid, ButtonColor::Secondary) => {
                if self.disabled {
                    (
                        theme.palette.contrast.contrast_50.clone(),
                        theme.palette.contrast.contrast_50.clone(),
                        theme.palette.contrast.contrast_300.clone(),
                    )
                } else {
                    (
                        theme.palette.contrast.contrast_50.clone(),
                        theme.palette.contrast.contrast_100.clone(),
                        theme.palette.contrast.contrast_600.clone(),
                    )
                }
            }
            (ButtonVariant::Solid, ButtonColor::Negative) => {
                if self.disabled {
                    (
                        theme.palette.negative.negative_700.clone(),
                        theme.palette.negative.negative_700.clone(),
                        theme.palette.negative.negative_300.clone(),
                    )
                } else {
                    (
                        theme.palette.negative.negative_500.clone(),
                        theme.palette.negative.negative_600.clone(),
                        theme.palette.white.clone(),
                    )
                }
            }
            (ButtonVariant::Outline, ButtonColor::Primary) => (
                "transparent".to_string(),
                theme.palette.primary.primary_50.clone(),
                theme.palette.primary.primary_600.clone(),
            ),
            (ButtonVariant::Ghost, _) => (
                "transparent".to_string(),
                theme.palette.contrast.contrast_50.clone(),
                theme.colors.default.text.clone(),
            ),
            _ => {
                // Default fallback
                (
                    theme.palette.contrast.contrast_100.clone(),
                    theme.palette.contrast.contrast_200.clone(),
                    theme.colors.default.text.clone(),
                )
            }
        };

        let (padding_v, padding_h, border_radius_val, gap) = match self.size {
            ButtonSize::Large => (12.0, 25.0, 10.0, 3.0),
            ButtonSize::Small => (8.0, 13.0, 8.0, 3.0),
            ButtonSize::Tiny => (5.0, 9.0, 6.0, 2.0),
        };

        let (width, height) = if matches!(self.shape, ButtonShape::Round | ButtonShape::Square) {
            let size = match self.size {
                ButtonSize::Large => 44.0,
                ButtonSize::Small => 33.0,
                ButtonSize::Tiny => 25.0,
            };
            (Some(size), Some(size))
        } else {
            (None, None)
        };

        let border_radius_final = if self.shape == ButtonShape::Round {
            radius::FULL
        } else {
            border_radius_val
        };

        ButtonStyles {
            background,
            background_hover,
            text_color,
            border_color: if self.variant == ButtonVariant::Outline {
                Some(theme.palette.primary.primary_500.clone())
            } else {
                None
            },
            border_width: if self.variant == ButtonVariant::Outline {
                1.0
            } else {
                0.0
            },
            padding_vertical: padding_v,
            padding_horizontal: padding_h,
            border_radius: border_radius_final,
            gap,
            width,
            height,
            opacity: if self.disabled { 0.7 } else { 1.0 },
        }
    }
}

/// Computed button styles
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ButtonStyles {
    /// Background color
    pub background: Color,
    /// Background color on hover
    pub background_hover: Color,
    /// Text color
    pub text_color: Color,
    /// Border color
    pub border_color: Option<Color>,
    /// Border width
    pub border_width: f32,
    /// Vertical padding
    pub padding_vertical: f32,
    /// Horizontal padding
    pub padding_horizontal: f32,
    /// Border radius
    pub border_radius: f32,
    /// Gap between icon and text
    pub gap: f32,
    /// Fixed width (for round/square shapes)
    pub width: Option<f32>,
    /// Fixed height (for round/square shapes)
    pub height: Option<f32>,
    /// Opacity
    pub opacity: f32,
}

// =============================================================================
// Text Component
// =============================================================================

/// Text semantic roles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextRole {
    /// Regular paragraph text
    #[default]
    Paragraph,
    /// Heading level 1
    H1,
    /// Heading level 2
    H2,
    /// Heading level 3
    H3,
    /// Heading level 4
    H4,
    /// Label text
    Label,
    /// Caption/help text
    Caption,
    /// Code/monospace text
    Code,
}

/// Text component properties
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Text {
    /// Unique component ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<ComponentId>,
    /// Text content
    pub content: String,
    /// Typography variant to use
    #[serde(default)]
    pub variant: TypographyVariant,
    /// Semantic role
    #[serde(default)]
    pub role: TextRole,
    /// Text color override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    /// Whether text is selectable
    #[serde(default)]
    pub selectable: bool,
    /// Number of lines (0 = unlimited)
    #[serde(default)]
    pub lines: u32,
    /// Text alignment
    #[serde(default)]
    pub align: TextAlign,
    /// Additional style props
    #[serde(default, skip_serializing_if = "is_default_style")]
    pub style: StyleProps,
}

/// Text alignment options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    /// Left aligned (default)
    #[default]
    Left,
    /// Center aligned
    Center,
    /// Right aligned
    Right,
    /// Justified
    Justify,
}

impl Text {
    /// Create new text component
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            id: None,
            content: content.into(),
            variant: TypographyVariant::Md,
            role: TextRole::Paragraph,
            color: None,
            selectable: false,
            lines: 0,
            align: TextAlign::Left,
            style: StyleProps::default(),
        }
    }

    /// Create a heading
    pub fn heading(content: impl Into<String>, level: u8) -> Self {
        let (variant, role) = match level {
            1 => (TypographyVariant::Title2xl, TextRole::H1),
            2 => (TypographyVariant::TitleXl, TextRole::H2),
            3 => (TypographyVariant::TitleLg, TextRole::H3),
            _ => (TypographyVariant::Title, TextRole::H4),
        };

        Self {
            id: None,
            content: content.into(),
            variant,
            role,
            color: None,
            selectable: false,
            lines: 0,
            align: TextAlign::Left,
            style: StyleProps::default(),
        }
    }

    /// Create caption text
    pub fn caption(content: impl Into<String>) -> Self {
        Self {
            id: None,
            content: content.into(),
            variant: TypographyVariant::Xs,
            role: TextRole::Caption,
            color: None,
            selectable: false,
            lines: 0,
            align: TextAlign::Left,
            style: StyleProps::default(),
        }
    }

    /// Set typography variant
    pub fn with_variant(mut self, variant: TypographyVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set text color
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set maximum lines
    pub fn with_lines(mut self, lines: u32) -> Self {
        self.lines = lines;
        self
    }

    /// Set text alignment
    pub fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// Make text selectable
    pub fn selectable(mut self) -> Self {
        self.selectable = true;
        self
    }
}

// =============================================================================
// Container Component
// =============================================================================

/// Container/View component for layout
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Container {
    /// Unique component ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<ComponentId>,
    /// Flex direction
    #[serde(default)]
    pub direction: FlexDirection,
    /// Justify content (main axis alignment)
    #[serde(default)]
    pub justify: JustifyContent,
    /// Align items (cross axis alignment)
    #[serde(default)]
    pub align: Alignment,
    /// Gap between children
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap: Option<f32>,
    /// Whether to wrap children
    #[serde(default)]
    pub wrap: bool,
    /// Style props
    #[serde(default, skip_serializing_if = "is_default_style")]
    pub style: StyleProps,
    /// Accessibility props
    #[serde(default, skip_serializing_if = "is_default_a11y")]
    pub accessibility: AccessibilityProps,
}

fn is_default_a11y(a11y: &AccessibilityProps) -> bool {
    a11y == &AccessibilityProps::default()
}

impl Container {
    /// Create a new container
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a row container
    pub fn row() -> Self {
        Self {
            direction: FlexDirection::Row,
            ..Default::default()
        }
    }

    /// Create a column container
    pub fn column() -> Self {
        Self {
            direction: FlexDirection::Column,
            ..Default::default()
        }
    }

    /// Set flex direction
    pub fn with_direction(mut self, direction: FlexDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Set justify content
    pub fn with_justify(mut self, justify: JustifyContent) -> Self {
        self.justify = justify;
        self
    }

    /// Set align items
    pub fn with_align(mut self, align: Alignment) -> Self {
        self.align = align;
        self
    }

    /// Set gap
    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = Some(gap);
        self
    }

    /// Enable wrapping
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    /// Set style
    pub fn with_style(mut self, style: StyleProps) -> Self {
        self.style = style;
        self
    }

    /// Set padding
    pub fn with_padding(mut self, padding: Spacing) -> Self {
        self.style.padding = Some(padding);
        self
    }

    /// Set background color
    pub fn with_background(mut self, color: impl Into<String>) -> Self {
        self.style.background_color = Some(color.into());
        self
    }
}

// =============================================================================
// Input Component
// =============================================================================

/// Input type variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputType {
    /// Text input
    #[default]
    Text,
    /// Password input (masked)
    Password,
    /// Email input
    Email,
    /// Numeric input
    Number,
    /// Search input
    Search,
    /// URL input
    Url,
    /// Phone number input
    Tel,
    /// Multi-line text area
    Textarea,
}

/// Input size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputSize {
    /// Small input
    Small,
    /// Medium input (default)
    #[default]
    Medium,
    /// Large input
    Large,
}

/// Input validation state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationState {
    /// Not validated
    #[default]
    None,
    /// Valid input
    Valid,
    /// Invalid input
    Invalid,
    /// Currently validating
    Validating,
}

/// Input component properties
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Input {
    /// Unique component ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<ComponentId>,
    /// Input type
    #[serde(default)]
    pub input_type: InputType,
    /// Input size
    #[serde(default)]
    pub size: InputSize,
    /// Placeholder text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    /// Current value
    #[serde(default)]
    pub value: String,
    /// Label text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Helper/hint text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub helper_text: Option<String>,
    /// Error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Validation state
    #[serde(default)]
    pub validation: ValidationState,
    /// Whether the input is disabled
    #[serde(default)]
    pub disabled: bool,
    /// Whether the input is read-only
    #[serde(default)]
    pub readonly: bool,
    /// Whether the input is required
    #[serde(default)]
    pub required: bool,
    /// Maximum length
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
    /// Minimum length
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,
    /// Pattern for validation (regex)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// Auto-complete hint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autocomplete: Option<String>,
    /// On change handler
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_change: Option<EventHandler>,
    /// On focus handler
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_focus: Option<EventHandler>,
    /// On blur handler
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_blur: Option<EventHandler>,
    /// On submit handler (for single-line inputs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_submit: Option<EventHandler>,
    /// Leading icon
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leading_icon: Option<String>,
    /// Trailing icon
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trailing_icon: Option<String>,
    /// Style props
    #[serde(default, skip_serializing_if = "is_default_style")]
    pub style: StyleProps,
}

impl Input {
    /// Create a new text input
    pub fn new() -> Self {
        Self {
            id: None,
            input_type: InputType::Text,
            size: InputSize::Medium,
            placeholder: None,
            value: String::new(),
            label: None,
            helper_text: None,
            error: None,
            validation: ValidationState::None,
            disabled: false,
            readonly: false,
            required: false,
            max_length: None,
            min_length: None,
            pattern: None,
            autocomplete: None,
            on_change: None,
            on_focus: None,
            on_blur: None,
            on_submit: None,
            leading_icon: None,
            trailing_icon: None,
            style: StyleProps::default(),
        }
    }

    /// Create a password input
    pub fn password() -> Self {
        Self {
            input_type: InputType::Password,
            ..Self::new()
        }
    }

    /// Create an email input
    pub fn email() -> Self {
        Self {
            input_type: InputType::Email,
            autocomplete: Some("email".to_string()),
            ..Self::new()
        }
    }

    /// Create a search input
    pub fn search() -> Self {
        Self {
            input_type: InputType::Search,
            leading_icon: Some("search".to_string()),
            ..Self::new()
        }
    }

    /// Create a textarea
    pub fn textarea() -> Self {
        Self {
            input_type: InputType::Textarea,
            ..Self::new()
        }
    }

    /// Set placeholder text
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set label
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set helper text
    pub fn with_helper(mut self, helper: impl Into<String>) -> Self {
        self.helper_text = Some(helper.into());
        self
    }

    /// Set error message
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self.validation = ValidationState::Invalid;
        self
    }

    /// Set required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Set disabled
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Set max length
    pub fn with_max_length(mut self, max: u32) -> Self {
        self.max_length = Some(max);
        self
    }

    /// Set on change handler
    pub fn on_change(mut self, handler: impl Into<String>) -> Self {
        self.on_change = Some(handler.into());
        self
    }

    /// Get computed input height based on size
    pub fn computed_height(&self) -> f32 {
        match self.size {
            InputSize::Small => sizing::input::SM_HEIGHT,
            InputSize::Medium => sizing::input::MD_HEIGHT,
            InputSize::Large => sizing::input::LG_HEIGHT,
        }
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Icon Component
// =============================================================================

/// Icon size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IconSize {
    /// Extra small (12px)
    Xs,
    /// Small (16px)
    Sm,
    /// Medium (20px)
    #[default]
    Md,
    /// Large (24px)
    Lg,
    /// Extra large (32px)
    Xl,
    /// 2x large (40px)
    Xxl,
}

impl IconSize {
    /// Get the pixel size
    pub fn pixels(&self) -> f32 {
        match self {
            IconSize::Xs => sizing::icon::XS,
            IconSize::Sm => sizing::icon::SM,
            IconSize::Md => sizing::icon::MD,
            IconSize::Lg => sizing::icon::LG,
            IconSize::Xl => sizing::icon::XL,
            IconSize::Xxl => sizing::icon::XXL,
        }
    }
}

/// Icon component properties
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Icon {
    /// Icon name (from icon set)
    pub name: String,
    /// Icon size
    #[serde(default)]
    pub size: IconSize,
    /// Icon color override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    /// Accessible label
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Style props
    #[serde(default, skip_serializing_if = "is_default_style")]
    pub style: StyleProps,
}

impl Icon {
    /// Create a new icon
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            size: IconSize::Md,
            color: None,
            label: None,
            style: StyleProps::default(),
        }
    }

    /// Set icon size
    pub fn with_size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }

    /// Set icon color
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set accessible label
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Get the pixel size
    pub fn pixel_size(&self) -> f32 {
        self.size.pixels()
    }
}

// =============================================================================
// Divider Component
// =============================================================================

/// Divider orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DividerOrientation {
    /// Horizontal divider
    #[default]
    Horizontal,
    /// Vertical divider
    Vertical,
}

/// Divider component
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Divider {
    /// Orientation
    #[serde(default)]
    pub orientation: DividerOrientation,
    /// Color override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    /// Thickness
    #[serde(default = "default_divider_thickness")]
    pub thickness: f32,
    /// Style props
    #[serde(default, skip_serializing_if = "is_default_style")]
    pub style: StyleProps,
}

fn default_divider_thickness() -> f32 {
    1.0
}

impl Default for Divider {
    fn default() -> Self {
        Self {
            orientation: DividerOrientation::Horizontal,
            color: None,
            thickness: 1.0,
            style: StyleProps::default(),
        }
    }
}

impl Divider {
    /// Create a horizontal divider
    pub fn horizontal() -> Self {
        Self {
            orientation: DividerOrientation::Horizontal,
            ..Default::default()
        }
    }

    /// Create a vertical divider
    pub fn vertical() -> Self {
        Self {
            orientation: DividerOrientation::Vertical,
            ..Default::default()
        }
    }

    /// Set color
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set thickness
    pub fn with_thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }
}

// =============================================================================
// Tab Bar Component
// =============================================================================

/// Tab bar item representing a navigation tab
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabBarItem {
    /// Tab identifier matching NavigationTab
    pub id: String,
    /// Icon name for inactive state
    pub icon: String,
    /// Icon name for active state (filled variant)
    pub icon_active: String,
    /// Label text
    pub label: String,
    /// Whether this tab is currently active
    pub is_active: bool,
    /// Badge count (e.g., unread notifications)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge_count: Option<u32>,
    /// Has new indicator (dot badge)
    #[serde(default)]
    pub has_new: bool,
    /// Accessibility label
    pub accessibility_label: String,
    /// Accessibility hint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility_hint: Option<String>,
}

impl TabBarItem {
    /// Create a new tab bar item
    pub fn new(
        id: impl Into<String>,
        icon: impl Into<String>,
        icon_active: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        let label_str = label.into();
        Self {
            id: id.into(),
            icon: icon.into(),
            icon_active: icon_active.into(),
            accessibility_label: label_str.clone(),
            label: label_str,
            is_active: false,
            badge_count: None,
            has_new: false,
            accessibility_hint: None,
        }
    }

    /// Set active state
    pub fn with_active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }

    /// Set badge count
    pub fn with_badge(mut self, count: u32) -> Self {
        self.badge_count = Some(count);
        self
    }

    /// Set has new indicator
    pub fn with_new(mut self, has_new: bool) -> Self {
        self.has_new = has_new;
        self
    }

    /// Set accessibility hint
    pub fn with_accessibility_hint(mut self, hint: impl Into<String>) -> Self {
        self.accessibility_hint = Some(hint.into());
        self
    }

    /// Get the current icon based on active state
    pub fn current_icon(&self) -> &str {
        if self.is_active {
            &self.icon_active
        } else {
            &self.icon
        }
    }
}

/// Tab bar position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TabBarPosition {
    /// Bottom of screen (mobile default)
    #[default]
    Bottom,
    /// Left side (desktop sidebar)
    Left,
    /// Top of screen
    Top,
}

/// Complete tab bar component
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabBar {
    /// Tab items
    pub items: Vec<TabBarItem>,
    /// Position of the tab bar
    pub position: TabBarPosition,
    /// Whether to show labels
    pub show_labels: bool,
    /// Safe area bottom inset (for mobile)
    #[serde(default)]
    pub safe_area_bottom: f32,
    /// Whether the border should be hidden
    #[serde(default)]
    pub hide_border: bool,
    /// On tab press event handler
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_tab_press: Option<EventHandler>,
    /// On tab long press event handler (e.g., for account switcher)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_tab_long_press: Option<EventHandler>,
    /// Style properties
    #[serde(flatten)]
    pub style: StyleProps,
}

impl Default for TabBar {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            position: TabBarPosition::Bottom,
            show_labels: false,
            safe_area_bottom: 0.0,
            hide_border: false,
            on_tab_press: None,
            on_tab_long_press: None,
            style: StyleProps::default(),
        }
    }
}

impl TabBar {
    /// Create a new tab bar with default navigation tabs
    pub fn new() -> Self {
        Self {
            items: vec![
                TabBarItem::new("home", "home", "home-filled", "Home"),
                TabBarItem::new("search", "search", "search-filled", "Search"),
                TabBarItem::new("messages", "chat", "chat-filled", "Chat"),
                TabBarItem::new("notifications", "bell", "bell-filled", "Notifications"),
                TabBarItem::new("profile", "user", "user-filled", "Profile"),
            ],
            ..Default::default()
        }
    }

    /// Create a tab bar with custom items
    pub fn with_items(items: Vec<TabBarItem>) -> Self {
        Self {
            items,
            ..Default::default()
        }
    }

    /// Set the active tab by ID
    pub fn set_active(mut self, tab_id: &str) -> Self {
        for item in &mut self.items {
            item.is_active = item.id == tab_id;
        }
        self
    }

    /// Set position
    pub fn with_position(mut self, position: TabBarPosition) -> Self {
        self.position = position;
        self
    }

    /// Set whether to show labels
    pub fn with_labels(mut self, show: bool) -> Self {
        self.show_labels = show;
        self
    }

    /// Set safe area bottom inset
    pub fn with_safe_area(mut self, inset: f32) -> Self {
        self.safe_area_bottom = inset;
        self
    }

    /// Set border visibility
    pub fn with_border(mut self, show: bool) -> Self {
        self.hide_border = !show;
        self
    }

    /// Set on tab press handler
    pub fn on_press(mut self, handler: impl Into<EventHandler>) -> Self {
        self.on_tab_press = Some(handler.into());
        self
    }

    /// Set on tab long press handler
    pub fn on_long_press(mut self, handler: impl Into<EventHandler>) -> Self {
        self.on_tab_long_press = Some(handler.into());
        self
    }

    /// Update badge count for a specific tab
    pub fn set_badge(mut self, tab_id: &str, count: Option<u32>) -> Self {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == tab_id) {
            item.badge_count = count;
        }
        self
    }

    /// Set has new indicator for a specific tab
    pub fn set_has_new(mut self, tab_id: &str, has_new: bool) -> Self {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == tab_id) {
            item.has_new = has_new;
        }
        self
    }

    /// Get the currently active tab ID
    pub fn active_tab(&self) -> Option<&str> {
        self.items
            .iter()
            .find(|i| i.is_active)
            .map(|i| i.id.as_str())
    }

    /// Compute styles for the tab bar based on theme
    pub fn computed_styles(&self, theme: &Theme) -> TabBarStyles {
        let background = theme.colors.default.background.clone();
        let border_color = if self.hide_border {
            background.clone()
        } else {
            theme.palette.contrast.contrast_100.clone()
        };

        TabBarStyles {
            background,
            border_color,
            border_width: if self.hide_border { 0.0 } else { 1.0 },
            padding_bottom: self.safe_area_bottom.max(15.0).min(60.0),
            padding_left: 5.0,
            padding_right: 10.0,
            item_padding_top: 13.0,
            item_padding_bottom: 4.0,
        }
    }

    /// Compute styles for a tab item
    pub fn item_styles(&self, item: &TabBarItem, theme: &Theme) -> TabItemStyles {
        let icon_color = theme.colors.default.text.clone();
        let badge_background = theme.palette.primary.primary_500.clone();
        let badge_text = "#FFFFFF".to_string();

        TabItemStyles {
            icon_size: 28.0,
            icon_color,
            label_color: theme.colors.default.text.clone(),
            badge_background,
            badge_text,
            badge_size: if item.badge_count.is_some() { 18.0 } else { 8.0 },
            has_new_dot_size: 8.0,
        }
    }
}

/// Computed styles for tab bar
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabBarStyles {
    /// Background color
    pub background: Color,
    /// Border color
    pub border_color: Color,
    /// Border width (top border)
    pub border_width: f32,
    /// Bottom padding (for safe area)
    pub padding_bottom: f32,
    /// Left padding
    pub padding_left: f32,
    /// Right padding
    pub padding_right: f32,
    /// Item top padding
    pub item_padding_top: f32,
    /// Item bottom padding
    pub item_padding_bottom: f32,
}

/// Computed styles for a tab item
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabItemStyles {
    /// Icon size in pixels
    pub icon_size: f32,
    /// Icon color
    pub icon_color: Color,
    /// Label color
    pub label_color: Color,
    /// Badge background color
    pub badge_background: Color,
    /// Badge text color
    pub badge_text: Color,
    /// Badge size (for count badge)
    pub badge_size: f32,
    /// Has new dot size
    pub has_new_dot_size: f32,
}

// =============================================================================
// Avatar Component
// =============================================================================

/// Avatar shape
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AvatarShape {
    /// Circular avatar
    #[default]
    Circle,
    /// Rounded square (for labelers)
    Square,
}

/// Avatar size presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AvatarSize {
    /// Extra small (20px)
    Xs,
    /// Small (24px)
    Sm,
    /// Medium (32px) - default
    #[default]
    Md,
    /// Large (48px)
    Lg,
    /// Extra large (64px)
    Xl,
    /// Custom size
    Custom(u32),
}

impl AvatarSize {
    /// Get pixel size
    pub fn pixels(&self) -> u32 {
        match self {
            AvatarSize::Xs => 20,
            AvatarSize::Sm => 24,
            AvatarSize::Md => 32,
            AvatarSize::Lg => 48,
            AvatarSize::Xl => 64,
            AvatarSize::Custom(size) => *size,
        }
    }
}

/// User avatar component
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Avatar {
    /// Image URL (None for default avatar)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    /// Alternative text
    pub alt: String,
    /// Size preset
    pub size: AvatarSize,
    /// Shape
    pub shape: AvatarShape,
    /// Show live indicator ring
    #[serde(default)]
    pub live: bool,
    /// Hide live badge (show ring only)
    #[serde(default)]
    pub hide_live_badge: bool,
    /// Border when selected/active
    #[serde(default)]
    pub show_border: bool,
    /// Border color (uses theme text color if not set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_color: Option<Color>,
    /// Style properties
    #[serde(flatten)]
    pub style: StyleProps,
}

impl Default for Avatar {
    fn default() -> Self {
        Self {
            src: None,
            alt: "Avatar".to_string(),
            size: AvatarSize::Md,
            shape: AvatarShape::Circle,
            live: false,
            hide_live_badge: false,
            show_border: false,
            border_color: None,
            style: StyleProps::default(),
        }
    }
}

impl Avatar {
    /// Create a new avatar
    pub fn new(alt: impl Into<String>) -> Self {
        Self {
            alt: alt.into(),
            ..Default::default()
        }
    }

    /// Set image source
    pub fn with_src(mut self, src: impl Into<String>) -> Self {
        self.src = Some(src.into());
        self
    }

    /// Set size
    pub fn with_size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    /// Set shape
    pub fn with_shape(mut self, shape: AvatarShape) -> Self {
        self.shape = shape;
        self
    }

    /// Set live indicator
    pub fn with_live(mut self, live: bool) -> Self {
        self.live = live;
        self
    }

    /// Set border visibility
    pub fn with_border(mut self, show: bool) -> Self {
        self.show_border = show;
        self
    }

    /// Set border color
    pub fn with_border_color(mut self, color: impl Into<Color>) -> Self {
        self.border_color = Some(color.into());
        self
    }

    /// Compute styles based on theme
    pub fn computed_styles(&self, theme: &Theme) -> AvatarStyles {
        let size = self.size.pixels() as f32;
        let border_radius = match self.shape {
            AvatarShape::Circle => size / 2.0,
            AvatarShape::Square => 8.0,
        };
        let border_color = self
            .border_color
            .clone()
            .unwrap_or_else(|| theme.colors.default.text.clone());
        let border_width = if self.show_border && !self.live {
            1.0
        } else {
            0.0
        };

        AvatarStyles {
            size,
            border_radius,
            border_color,
            border_width,
            background: theme.palette.contrast.contrast_100.clone(),
        }
    }
}

/// Computed avatar styles
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AvatarStyles {
    /// Size in pixels
    pub size: f32,
    /// Border radius
    pub border_radius: f32,
    /// Border color
    pub border_color: Color,
    /// Border width
    pub border_width: f32,
    /// Background color (for loading/placeholder)
    pub background: Color,
}

// =============================================================================
// Badge Component
// =============================================================================

/// Badge variant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BadgeVariant {
    /// Primary colored badge
    #[default]
    Primary,
    /// Secondary colored badge
    Secondary,
    /// Success/positive badge
    Success,
    /// Warning badge
    Warning,
    /// Error/danger badge
    Error,
    /// Neutral badge
    Neutral,
}

/// Notification badge component
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Badge {
    /// Badge content (number or text)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Variant/color
    pub variant: BadgeVariant,
    /// Show as dot (no content)
    #[serde(default)]
    pub dot: bool,
    /// Maximum number to show (e.g., 99+)
    #[serde(default)]
    pub max: Option<u32>,
    /// Style properties
    #[serde(flatten)]
    pub style: StyleProps,
}

impl Default for Badge {
    fn default() -> Self {
        Self {
            content: None,
            variant: BadgeVariant::Primary,
            dot: false,
            max: Some(99),
            style: StyleProps::default(),
        }
    }
}

impl Badge {
    /// Create a badge with a count
    pub fn count(value: u32) -> Self {
        Self {
            content: Some(value.to_string()),
            ..Default::default()
        }
    }

    /// Create a dot badge
    pub fn dot() -> Self {
        Self {
            dot: true,
            ..Default::default()
        }
    }

    /// Set variant
    pub fn with_variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set max value
    pub fn with_max(mut self, max: u32) -> Self {
        self.max = Some(max);
        self
    }

    /// Get display content (respecting max)
    pub fn display_content(&self) -> Option<String> {
        if self.dot {
            return None;
        }
        match (&self.content, self.max) {
            (Some(content), Some(max)) => {
                if let Ok(num) = content.parse::<u32>() {
                    if num > max {
                        return Some(format!("{}+", max));
                    }
                }
                Some(content.clone())
            }
            (Some(content), None) => Some(content.clone()),
            (None, _) => None,
        }
    }

    /// Compute styles based on theme
    pub fn computed_styles(&self, theme: &Theme) -> BadgeStyles {
        let (background, text) = match self.variant {
            BadgeVariant::Primary => (
                theme.palette.primary.primary_500.clone(),
                "#FFFFFF".to_string(),
            ),
            BadgeVariant::Secondary => (
                theme.palette.contrast.contrast_200.clone(),
                theme.colors.default.text.clone(),
            ),
            BadgeVariant::Success => (
                theme.palette.positive.positive_500.clone(),
                "#FFFFFF".to_string(),
            ),
            BadgeVariant::Warning => ("#FFB703".to_string(), "#000000".to_string()),
            BadgeVariant::Error => (
                theme.palette.negative.negative_500.clone(),
                "#FFFFFF".to_string(),
            ),
            BadgeVariant::Neutral => (
                theme.palette.contrast.contrast_300.clone(),
                theme.colors.default.text.clone(),
            ),
        };

        let size = if self.dot { 8.0 } else { 18.0 };

        BadgeStyles {
            background,
            text,
            size,
            font_size: 12.0,
            border_radius: size / 2.0,
            padding_horizontal: if self.dot { 0.0 } else { 4.0 },
        }
    }
}

/// Computed badge styles
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BadgeStyles {
    /// Background color
    pub background: Color,
    /// Text color
    pub text: Color,
    /// Badge size (height)
    pub size: f32,
    /// Font size
    pub font_size: f32,
    /// Border radius
    pub border_radius: f32,
    /// Horizontal padding
    pub padding_horizontal: f32,
}

// =============================================================================
// Loading Components
// =============================================================================

/// Loading spinner size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LoaderSize {
    /// Small (16px)
    Sm,
    /// Medium (24px) - default
    #[default]
    Md,
    /// Large (32px)
    Lg,
    /// Extra large (48px)
    Xl,
}

impl LoaderSize {
    /// Get pixel size
    pub fn pixels(&self) -> f32 {
        match self {
            LoaderSize::Sm => 16.0,
            LoaderSize::Md => 24.0,
            LoaderSize::Lg => 32.0,
            LoaderSize::Xl => 48.0,
        }
    }
}

/// Loading spinner component
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Loader {
    /// Size of the loader
    pub size: LoaderSize,
    /// Color override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    /// Animation duration in milliseconds
    #[serde(default = "default_loader_duration")]
    pub duration_ms: u32,
    /// Style properties
    #[serde(flatten)]
    pub style: StyleProps,
}

fn default_loader_duration() -> u32 {
    500
}

impl Default for Loader {
    fn default() -> Self {
        Self {
            size: LoaderSize::Md,
            color: None,
            duration_ms: 500,
            style: StyleProps::default(),
        }
    }
}

impl Loader {
    /// Create a new loader
    pub fn new() -> Self {
        Self::default()
    }

    /// Set size
    pub fn with_size(mut self, size: LoaderSize) -> Self {
        self.size = size;
        self
    }

    /// Set color
    pub fn with_color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Compute styles based on theme
    pub fn computed_styles(&self, theme: &Theme) -> LoaderStyles {
        let color = self
            .color
            .clone()
            .unwrap_or_else(|| theme.palette.contrast.contrast_700.clone());
        let size = self.size.pixels();

        LoaderStyles {
            size,
            color,
            stroke_width: (size / 8.0).max(2.0),
        }
    }
}

/// Computed loader styles
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoaderStyles {
    /// Size in pixels
    pub size: f32,
    /// Stroke/fill color
    pub color: Color,
    /// Stroke width
    pub stroke_width: f32,
}

/// Skeleton placeholder shape
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SkeletonShape {
    /// Rectangle with rounded corners
    #[default]
    Rectangle,
    /// Circle
    Circle,
    /// Pill/capsule shape
    Pill,
}

/// Skeleton placeholder component
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Skeleton {
    /// Width (pixels or percentage)
    pub width: Dimension,
    /// Height in pixels
    pub height: f32,
    /// Shape
    pub shape: SkeletonShape,
    /// Border radius override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_radius: Option<f32>,
    /// Enable shimmer animation
    #[serde(default = "default_true")]
    pub animated: bool,
    /// Style properties
    #[serde(flatten)]
    pub style: StyleProps,
}

fn default_true() -> bool {
    true
}

impl Default for Skeleton {
    fn default() -> Self {
        Self {
            width: Dimension::percent(100.0),
            height: 16.0,
            shape: SkeletonShape::Rectangle,
            border_radius: None,
            animated: true,
            style: StyleProps::default(),
        }
    }
}

impl Skeleton {
    /// Create a skeleton with fixed dimensions
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width: Dimension::px(width),
            height,
            ..Default::default()
        }
    }

    /// Create a text line skeleton
    pub fn text_line(width_percent: f32, height: f32) -> Self {
        Self {
            width: Dimension::percent(width_percent),
            height,
            shape: SkeletonShape::Rectangle,
            border_radius: Some(4.0),
            ..Default::default()
        }
    }

    /// Create an avatar skeleton
    pub fn avatar(size: f32) -> Self {
        Self {
            width: Dimension::px(size),
            height: size,
            shape: SkeletonShape::Circle,
            ..Default::default()
        }
    }

    /// Create a thumbnail/image skeleton
    pub fn thumbnail(width: f32, height: f32) -> Self {
        Self {
            width: Dimension::px(width),
            height,
            shape: SkeletonShape::Rectangle,
            border_radius: Some(8.0),
            ..Default::default()
        }
    }

    /// Set shape
    pub fn with_shape(mut self, shape: SkeletonShape) -> Self {
        self.shape = shape;
        self
    }

    /// Set border radius
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.border_radius = Some(radius);
        self
    }

    /// Set animation
    pub fn with_animation(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }

    /// Compute styles based on theme
    pub fn computed_styles(&self, theme: &Theme) -> SkeletonStyles {
        let background = theme.palette.contrast.contrast_50.clone();
        let shimmer = theme.palette.contrast.contrast_100.clone();

        let border_radius = self.border_radius.unwrap_or_else(|| match self.shape {
            SkeletonShape::Rectangle => 6.0,
            SkeletonShape::Circle => self.height / 2.0,
            SkeletonShape::Pill => self.height / 2.0,
        });

        SkeletonStyles {
            background,
            shimmer,
            border_radius,
            animation_duration_ms: if self.animated { 1500 } else { 0 },
        }
    }
}

/// Computed skeleton styles
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkeletonStyles {
    /// Background color
    pub background: Color,
    /// Shimmer highlight color
    pub shimmer: Color,
    /// Border radius
    pub border_radius: f32,
    /// Animation duration (0 for no animation)
    pub animation_duration_ms: u32,
}

/// Post skeleton placeholder
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PostSkeleton {
    /// Show action buttons placeholders
    #[serde(default = "default_true")]
    pub show_actions: bool,
    /// Number of text lines
    #[serde(default = "default_text_lines")]
    pub text_lines: u8,
    /// Style properties
    #[serde(flatten)]
    pub style: StyleProps,
}

fn default_text_lines() -> u8 {
    3
}

impl PostSkeleton {
    /// Create a new post skeleton
    pub fn new() -> Self {
        Self::default()
    }

    /// Set number of text lines
    pub fn with_lines(mut self, lines: u8) -> Self {
        self.text_lines = lines;
        self
    }

    /// Set whether to show actions
    pub fn with_actions(mut self, show: bool) -> Self {
        self.show_actions = show;
        self
    }

    /// Get skeleton elements for this post
    pub fn elements(&self) -> Vec<SkeletonElement> {
        let mut elements = vec![
            // Avatar
            SkeletonElement {
                skeleton: Skeleton::avatar(42.0),
                x: 10.0,
                y: 14.0,
            },
            // Username
            SkeletonElement {
                skeleton: Skeleton::text_line(30.0, 6.0),
                x: 64.0,
                y: 14.0,
            },
        ];

        // Text lines
        let line_y_start = 30.0;
        for i in 0..self.text_lines {
            let width = if i == self.text_lines - 1 { 80.0 } else { 95.0 };
            elements.push(SkeletonElement {
                skeleton: Skeleton::text_line(width, 6.0),
                x: 64.0,
                y: line_y_start + (i as f32 * 14.0),
            });
        }

        elements
    }
}

/// Single skeleton element with position
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkeletonElement {
    /// The skeleton
    pub skeleton: Skeleton,
    /// X position
    pub x: f32,
    /// Y position
    pub y: f32,
}

/// Profile card skeleton
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileCardSkeleton {
    /// Show bio lines
    #[serde(default = "default_true")]
    pub show_bio: bool,
    /// Style properties
    #[serde(flatten)]
    pub style: StyleProps,
}

impl Default for ProfileCardSkeleton {
    fn default() -> Self {
        Self {
            show_bio: true,
            style: StyleProps::default(),
        }
    }
}

impl ProfileCardSkeleton {
    /// Create a new profile card skeleton
    pub fn new() -> Self {
        Self::default()
    }

    /// Get skeleton elements
    pub fn elements(&self) -> Vec<SkeletonElement> {
        let mut elements = vec![
            // Avatar
            SkeletonElement {
                skeleton: Skeleton::avatar(40.0),
                x: 10.0,
                y: 10.0,
            },
            // Display name
            SkeletonElement {
                skeleton: Skeleton::new(140.0, 8.0),
                x: 60.0,
                y: 10.0,
            },
            // Handle
            SkeletonElement {
                skeleton: Skeleton::new(120.0, 8.0),
                x: 60.0,
                y: 25.0,
            },
        ];

        if self.show_bio {
            elements.push(SkeletonElement {
                skeleton: Skeleton::new(220.0, 8.0),
                x: 60.0,
                y: 45.0,
            });
        }

        elements
    }
}

/// Notification skeleton
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NotificationSkeleton {
    /// Style properties
    #[serde(flatten)]
    pub style: StyleProps,
}

impl NotificationSkeleton {
    /// Create a new notification skeleton
    pub fn new() -> Self {
        Self::default()
    }

    /// Get skeleton elements
    pub fn elements(&self) -> Vec<SkeletonElement> {
        vec![
            // Icon placeholder
            SkeletonElement {
                skeleton: Skeleton::new(24.0, 24.0).with_shape(SkeletonShape::Circle),
                x: 26.0,
                y: 10.0,
            },
            // Avatar
            SkeletonElement {
                skeleton: Skeleton::avatar(35.0),
                x: 60.0,
                y: 10.0,
            },
            // Text line 1
            SkeletonElement {
                skeleton: Skeleton::text_line(90.0, 6.0),
                x: 60.0,
                y: 55.0,
            },
            // Text line 2
            SkeletonElement {
                skeleton: Skeleton::text_line(70.0, 6.0),
                x: 60.0,
                y: 68.0,
            },
        ]
    }
}

/// Chat list item skeleton
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ChatListSkeleton {
    /// Style properties
    #[serde(flatten)]
    pub style: StyleProps,
}

impl ChatListSkeleton {
    /// Create a new chat list skeleton
    pub fn new() -> Self {
        Self::default()
    }

    /// Get skeleton elements
    pub fn elements(&self) -> Vec<SkeletonElement> {
        vec![
            // Avatar
            SkeletonElement {
                skeleton: Skeleton::avatar(52.0),
                x: 16.0,
                y: 16.0,
            },
            // Name
            SkeletonElement {
                skeleton: Skeleton::new(140.0, 12.0),
                x: 80.0,
                y: 20.0,
            },
            // Last message line 1
            SkeletonElement {
                skeleton: Skeleton::new(120.0, 8.0),
                x: 80.0,
                y: 40.0,
            },
            // Last message line 2
            SkeletonElement {
                skeleton: Skeleton::new(100.0, 8.0),
                x: 80.0,
                y: 54.0,
            },
        ]
    }
}

/// Loading state wrapper for any content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoadingState<T> {
    /// The loading state
    pub state: LoadingStateType<T>,
}

/// Loading state types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum LoadingStateType<T> {
    /// Initial state, not yet loaded
    Idle,
    /// Currently loading
    Loading,
    /// Successfully loaded with data
    Success(T),
    /// Error state with message
    Error(String),
    /// Refreshing (has data but updating)
    Refreshing(T),
}

impl<T> Default for LoadingState<T> {
    fn default() -> Self {
        Self {
            state: LoadingStateType::Idle,
        }
    }
}

impl<T> LoadingState<T> {
    /// Create an idle state
    pub fn idle() -> Self {
        Self {
            state: LoadingStateType::Idle,
        }
    }

    /// Create a loading state
    pub fn loading() -> Self {
        Self {
            state: LoadingStateType::Loading,
        }
    }

    /// Create a success state
    pub fn success(data: T) -> Self {
        Self {
            state: LoadingStateType::Success(data),
        }
    }

    /// Create an error state
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            state: LoadingStateType::Error(message.into()),
        }
    }

    /// Create a refreshing state
    pub fn refreshing(data: T) -> Self {
        Self {
            state: LoadingStateType::Refreshing(data),
        }
    }

    /// Check if loading
    pub fn is_loading(&self) -> bool {
        matches!(self.state, LoadingStateType::Loading)
    }

    /// Check if has data (success or refreshing)
    pub fn has_data(&self) -> bool {
        matches!(
            self.state,
            LoadingStateType::Success(_) | LoadingStateType::Refreshing(_)
        )
    }

    /// Check if error
    pub fn is_error(&self) -> bool {
        matches!(self.state, LoadingStateType::Error(_))
    }

    /// Get data reference if available
    pub fn data(&self) -> Option<&T> {
        match &self.state {
            LoadingStateType::Success(data) | LoadingStateType::Refreshing(data) => Some(data),
            _ => None,
        }
    }

    /// Get error message if in error state
    pub fn error_message(&self) -> Option<&str> {
        match &self.state {
            LoadingStateType::Error(msg) => Some(msg),
            _ => None,
        }
    }
}

// =============================================================================
// Toast/Snackbar Components
// =============================================================================

/// Default toast duration in milliseconds
pub const DEFAULT_TOAST_DURATION: u32 = 3000;

/// Toast variant/type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ToastType {
    /// Default style
    #[default]
    Default,
    /// Success notification
    Success,
    /// Error notification
    Error,
    /// Warning notification
    Warning,
    /// Informational notification
    Info,
}

impl ToastType {
    /// Get icon name for this toast type
    pub fn icon(&self) -> &'static str {
        match self {
            ToastType::Default => "circle-check",
            ToastType::Success => "circle-check",
            ToastType::Error => "circle-info",
            ToastType::Warning => "warning",
            ToastType::Info => "circle-info",
        }
    }
}

/// Toast position on screen
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ToastPosition {
    /// Top of screen
    Top,
    /// Top left
    TopLeft,
    /// Top right
    TopRight,
    /// Bottom of screen (default)
    #[default]
    Bottom,
    /// Bottom left
    BottomLeft,
    /// Bottom right
    BottomRight,
}

/// Toast action button
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToastAction {
    /// Button label
    pub label: String,
    /// Action identifier
    pub action_id: String,
}

impl ToastAction {
    /// Create a new toast action
    pub fn new(label: impl Into<String>, action_id: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            action_id: action_id.into(),
        }
    }
}

/// Individual toast notification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Toast {
    /// Unique identifier
    pub id: String,
    /// Toast type/variant
    pub toast_type: ToastType,
    /// Message text
    pub message: String,
    /// Custom icon override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Duration in milliseconds (None for persistent)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u32>,
    /// Whether the toast can be dismissed by user
    #[serde(default = "default_true")]
    pub dismissible: bool,
    /// Optional action button
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<ToastAction>,
    /// Timestamp when created
    pub created_at: u64,
    /// On dismiss callback identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_dismiss: Option<EventHandler>,
    /// On press callback identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_press: Option<EventHandler>,
    /// On auto close callback identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_auto_close: Option<EventHandler>,
}

impl Toast {
    /// Create a new toast with default settings
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            toast_type: ToastType::Default,
            message: message.into(),
            icon: None,
            duration: Some(DEFAULT_TOAST_DURATION),
            dismissible: true,
            action: None,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            on_dismiss: None,
            on_press: None,
            on_auto_close: None,
        }
    }

    /// Create a success toast
    pub fn success(message: impl Into<String>) -> Self {
        Self::new(message).with_type(ToastType::Success)
    }

    /// Create an error toast
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(message).with_type(ToastType::Error)
    }

    /// Create a warning toast
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(message).with_type(ToastType::Warning)
    }

    /// Create an info toast
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message).with_type(ToastType::Info)
    }

    /// Set toast type
    pub fn with_type(mut self, toast_type: ToastType) -> Self {
        self.toast_type = toast_type;
        self
    }

    /// Set custom icon
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set duration (None for persistent)
    pub fn with_duration(mut self, duration: Option<u32>) -> Self {
        self.duration = duration;
        self
    }

    /// Make persistent (no auto-dismiss)
    pub fn persistent(mut self) -> Self {
        self.duration = None;
        self
    }

    /// Set dismissible
    pub fn with_dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = dismissible;
        self
    }

    /// Add action button
    pub fn with_action(mut self, label: impl Into<String>, action_id: impl Into<String>) -> Self {
        self.action = Some(ToastAction::new(label, action_id));
        self
    }

    /// Get the icon to display
    pub fn display_icon(&self) -> &str {
        self.icon.as_deref().unwrap_or_else(|| self.toast_type.icon())
    }

    /// Check if toast should auto-dismiss
    pub fn should_auto_dismiss(&self) -> bool {
        self.duration.is_some()
    }

    /// Compute styles based on theme
    pub fn computed_styles(&self, theme: &Theme) -> ToastStyles {
        let (background, border, icon_color, text_color) = match self.toast_type {
            ToastType::Default | ToastType::Warning | ToastType::Info => (
                theme.palette.contrast.contrast_25.clone(),
                theme.palette.contrast.contrast_100.clone(),
                theme.colors.default.text.clone(),
                theme.colors.default.text.clone(),
            ),
            ToastType::Success => (
                theme.palette.primary.primary_25.clone(),
                theme.palette.primary.primary_300.clone(),
                theme.palette.primary.primary_600.clone(),
                theme.palette.primary.primary_600.clone(),
            ),
            ToastType::Error => (
                theme.palette.negative.negative_25.clone(),
                theme.palette.negative.negative_200.clone(),
                theme.palette.negative.negative_700.clone(),
                theme.palette.negative.negative_700.clone(),
            ),
        };

        ToastStyles {
            background,
            border,
            icon_color,
            text_color,
            border_radius: 8.0,
            padding: 16.0,
        }
    }
}

/// Computed toast styles
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToastStyles {
    /// Background color
    pub background: Color,
    /// Border color
    pub border: Color,
    /// Icon color
    pub icon_color: Color,
    /// Text color
    pub text_color: Color,
    /// Border radius
    pub border_radius: f32,
    /// Padding
    pub padding: f32,
}

/// Toast queue/container for managing multiple toasts
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToastQueue {
    /// Active toasts
    pub toasts: Vec<Toast>,
    /// Position on screen
    pub position: ToastPosition,
    /// Maximum number of visible toasts
    pub max_visible: usize,
    /// Gap between toasts
    pub gap: f32,
    /// Safe area insets
    #[serde(default)]
    pub safe_area_top: f32,
    #[serde(default)]
    pub safe_area_bottom: f32,
}

impl Default for ToastQueue {
    fn default() -> Self {
        Self {
            toasts: Vec::new(),
            position: ToastPosition::Bottom,
            max_visible: 3,
            gap: 8.0,
            safe_area_top: 0.0,
            safe_area_bottom: 0.0,
        }
    }
}

impl ToastQueue {
    /// Create a new toast queue
    pub fn new() -> Self {
        Self::default()
    }

    /// Set position
    pub fn with_position(mut self, position: ToastPosition) -> Self {
        self.position = position;
        self
    }

    /// Set max visible toasts
    pub fn with_max_visible(mut self, max: usize) -> Self {
        self.max_visible = max;
        self
    }

    /// Set safe area insets
    pub fn with_safe_area(mut self, top: f32, bottom: f32) -> Self {
        self.safe_area_top = top;
        self.safe_area_bottom = bottom;
        self
    }

    /// Add a toast
    pub fn push(&mut self, toast: Toast) {
        self.toasts.push(toast);
    }

    /// Remove a toast by ID
    pub fn dismiss(&mut self, id: &str) -> Option<Toast> {
        if let Some(pos) = self.toasts.iter().position(|t| t.id == id) {
            Some(self.toasts.remove(pos))
        } else {
            None
        }
    }

    /// Clear all toasts
    pub fn clear(&mut self) {
        self.toasts.clear();
    }

    /// Get visible toasts (respecting max_visible)
    pub fn visible(&self) -> &[Toast] {
        let len = self.toasts.len();
        if len <= self.max_visible {
            &self.toasts
        } else {
            &self.toasts[len - self.max_visible..]
        }
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.toasts.is_empty()
    }

    /// Get number of toasts
    pub fn len(&self) -> usize {
        self.toasts.len()
    }

    /// Process auto-dismiss for expired toasts
    pub fn process_auto_dismiss(&mut self, current_time: u64) -> Vec<Toast> {
        let mut dismissed = Vec::new();
        self.toasts.retain(|toast| {
            if let Some(duration) = toast.duration {
                if current_time >= toast.created_at + duration as u64 {
                    dismissed.push(toast.clone());
                    return false;
                }
            }
            true
        });
        dismissed
    }
}

// =============================================================================
// Dialog/Modal Components
// =============================================================================

/// Dialog size presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DialogSize {
    /// Small dialog (300px max width)
    Small,
    /// Medium dialog (400px max width) - default
    #[default]
    Medium,
    /// Large dialog (600px max width)
    Large,
    /// Full width
    Full,
}

impl DialogSize {
    /// Get max width in pixels
    pub fn max_width(&self) -> Option<f32> {
        match self {
            DialogSize::Small => Some(300.0),
            DialogSize::Medium => Some(400.0),
            DialogSize::Large => Some(600.0),
            DialogSize::Full => None,
        }
    }
}

/// Dialog presentation style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DialogPresentation {
    /// Centered dialog overlay
    #[default]
    Modal,
    /// Bottom sheet (slides up from bottom)
    BottomSheet,
    /// Full screen
    FullScreen,
}

/// Dialog button action
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogButton {
    /// Button label
    pub label: String,
    /// Button action identifier
    pub action_id: String,
    /// Button variant
    pub variant: ButtonVariant,
    /// Button color
    pub color: ButtonColor,
    /// Is this the primary/confirm action
    #[serde(default)]
    pub is_primary: bool,
    /// Should close dialog when pressed
    #[serde(default = "default_true")]
    pub close_on_press: bool,
}

impl DialogButton {
    /// Create a primary confirm button
    pub fn confirm(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            action_id: "confirm".to_string(),
            variant: ButtonVariant::Solid,
            color: ButtonColor::Primary,
            is_primary: true,
            close_on_press: true,
        }
    }

    /// Create a destructive confirm button
    pub fn destructive(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            action_id: "confirm".to_string(),
            variant: ButtonVariant::Solid,
            color: ButtonColor::Negative,
            is_primary: true,
            close_on_press: true,
        }
    }

    /// Create a cancel button
    pub fn cancel(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            action_id: "cancel".to_string(),
            variant: ButtonVariant::Solid,
            color: ButtonColor::Secondary,
            is_primary: false,
            close_on_press: true,
        }
    }

    /// Create a custom button
    pub fn custom(
        label: impl Into<String>,
        action_id: impl Into<String>,
        color: ButtonColor,
    ) -> Self {
        Self {
            label: label.into(),
            action_id: action_id.into(),
            variant: ButtonVariant::Solid,
            color,
            is_primary: false,
            close_on_press: true,
        }
    }

    /// Set whether to close on press
    pub fn with_close(mut self, close: bool) -> Self {
        self.close_on_press = close;
        self
    }
}

/// Dialog/modal component
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dialog {
    /// Unique identifier
    pub id: String,
    /// Dialog title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Dialog description/content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Presentation style
    pub presentation: DialogPresentation,
    /// Size
    pub size: DialogSize,
    /// Whether dialog is open
    pub is_open: bool,
    /// Action buttons
    pub buttons: Vec<DialogButton>,
    /// Whether clicking backdrop dismisses
    #[serde(default = "default_true")]
    pub dismiss_on_backdrop: bool,
    /// Whether ESC key dismisses (web)
    #[serde(default = "default_true")]
    pub dismiss_on_escape: bool,
    /// Whether swipe down dismisses (bottom sheet)
    #[serde(default = "default_true")]
    pub dismiss_on_swipe: bool,
    /// Show close button in header
    #[serde(default)]
    pub show_close_button: bool,
    /// On close callback
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_close: Option<EventHandler>,
    /// Custom content (serialized component tree)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_content: Option<String>,
    /// Style properties
    #[serde(flatten)]
    pub style: StyleProps,
}

impl Default for Dialog {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: None,
            description: None,
            presentation: DialogPresentation::Modal,
            size: DialogSize::Medium,
            is_open: false,
            buttons: Vec::new(),
            dismiss_on_backdrop: true,
            dismiss_on_escape: true,
            dismiss_on_swipe: true,
            show_close_button: false,
            on_close: None,
            custom_content: None,
            style: StyleProps::default(),
        }
    }
}

impl Dialog {
    /// Create a new dialog
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an alert dialog (just OK button)
    pub fn alert(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            description: Some(message.into()),
            buttons: vec![DialogButton::confirm("OK")],
            ..Default::default()
        }
    }

    /// Create a confirm dialog (Confirm + Cancel)
    pub fn confirm(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            description: Some(message.into()),
            buttons: vec![
                DialogButton::confirm("Confirm"),
                DialogButton::cancel("Cancel"),
            ],
            ..Default::default()
        }
    }

    /// Create a destructive confirm dialog
    pub fn destructive(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            description: Some(message.into()),
            buttons: vec![
                DialogButton::destructive("Delete"),
                DialogButton::cancel("Cancel"),
            ],
            ..Default::default()
        }
    }

    /// Set title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set presentation style
    pub fn with_presentation(mut self, presentation: DialogPresentation) -> Self {
        self.presentation = presentation;
        self
    }

    /// Set size
    pub fn with_size(mut self, size: DialogSize) -> Self {
        self.size = size;
        self
    }

    /// Add a button
    pub fn with_button(mut self, button: DialogButton) -> Self {
        self.buttons.push(button);
        self
    }

    /// Set buttons
    pub fn with_buttons(mut self, buttons: Vec<DialogButton>) -> Self {
        self.buttons = buttons;
        self
    }

    /// Set dismiss on backdrop
    pub fn with_backdrop_dismiss(mut self, dismiss: bool) -> Self {
        self.dismiss_on_backdrop = dismiss;
        self
    }

    /// Show close button
    pub fn with_close_button(mut self, show: bool) -> Self {
        self.show_close_button = show;
        self
    }

    /// Open the dialog
    pub fn open(&mut self) {
        self.is_open = true;
    }

    /// Close the dialog
    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// Compute styles based on theme
    pub fn computed_styles(&self, theme: &Theme) -> DialogStyles {
        let max_width = self.size.max_width();
        let background = theme.colors.default.background.clone();
        let border_color = theme.palette.contrast.contrast_200.clone();
        let backdrop = "rgba(0, 0, 0, 0.5)".to_string();

        let border_radius = match self.presentation {
            DialogPresentation::Modal => 12.0,
            DialogPresentation::BottomSheet => 16.0,
            DialogPresentation::FullScreen => 0.0,
        };

        DialogStyles {
            max_width,
            background,
            border_color,
            backdrop,
            border_radius,
            padding: 16.0,
            shadow: crate::tokens::shadows::lg(),
        }
    }
}

/// Computed dialog styles
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogStyles {
    /// Max width (None for full width)
    pub max_width: Option<f32>,
    /// Background color
    pub background: Color,
    /// Border color
    pub border_color: Color,
    /// Backdrop color
    pub backdrop: Color,
    /// Border radius
    pub border_radius: f32,
    /// Content padding
    pub padding: f32,
    /// Box shadow
    pub shadow: crate::tokens::Shadow,
}

/// Dialog controller for managing multiple dialogs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DialogController {
    /// Stack of open dialogs
    pub dialogs: Vec<Dialog>,
}

impl DialogController {
    /// Create a new controller
    pub fn new() -> Self {
        Self::default()
    }

    /// Show a dialog
    pub fn show(&mut self, mut dialog: Dialog) {
        dialog.is_open = true;
        self.dialogs.push(dialog);
    }

    /// Close the top dialog
    pub fn close_top(&mut self) -> Option<Dialog> {
        self.dialogs.pop()
    }

    /// Close a specific dialog by ID
    pub fn close(&mut self, id: &str) -> Option<Dialog> {
        if let Some(pos) = self.dialogs.iter().position(|d| d.id == id) {
            Some(self.dialogs.remove(pos))
        } else {
            None
        }
    }

    /// Close all dialogs
    pub fn close_all(&mut self) {
        self.dialogs.clear();
    }

    /// Get the top dialog
    pub fn top(&self) -> Option<&Dialog> {
        self.dialogs.last()
    }

    /// Check if any dialog is open
    pub fn has_open(&self) -> bool {
        !self.dialogs.is_empty()
    }

    /// Get number of open dialogs
    pub fn count(&self) -> usize {
        self.dialogs.len()
    }
}

/// Confirmation prompt (simplified dialog for yes/no questions)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfirmPrompt {
    /// Unique identifier
    pub id: String,
    /// Title text
    pub title: String,
    /// Description text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Confirm button label
    pub confirm_label: String,
    /// Cancel button label
    pub cancel_label: String,
    /// Confirm button color
    pub confirm_color: ButtonColor,
    /// Whether to show cancel button
    #[serde(default = "default_true")]
    pub show_cancel: bool,
    /// On confirm callback
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_confirm: Option<EventHandler>,
    /// On cancel callback
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_cancel: Option<EventHandler>,
}

impl ConfirmPrompt {
    /// Create a new confirmation prompt
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.into(),
            description: None,
            confirm_label: "Confirm".to_string(),
            cancel_label: "Cancel".to_string(),
            confirm_color: ButtonColor::Primary,
            show_cancel: true,
            on_confirm: None,
            on_cancel: None,
        }
    }

    /// Create a destructive confirmation prompt
    pub fn destructive(title: impl Into<String>) -> Self {
        Self::new(title).with_confirm_color(ButtonColor::Negative)
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set confirm button label
    pub fn with_confirm_label(mut self, label: impl Into<String>) -> Self {
        self.confirm_label = label.into();
        self
    }

    /// Set cancel button label
    pub fn with_cancel_label(mut self, label: impl Into<String>) -> Self {
        self.cancel_label = label.into();
        self
    }

    /// Set confirm button color
    pub fn with_confirm_color(mut self, color: ButtonColor) -> Self {
        self.confirm_color = color;
        self
    }

    /// Hide cancel button
    pub fn without_cancel(mut self) -> Self {
        self.show_cancel = false;
        self
    }

    /// Convert to a Dialog
    pub fn to_dialog(&self) -> Dialog {
        let mut buttons = vec![DialogButton {
            label: self.confirm_label.clone(),
            action_id: "confirm".to_string(),
            variant: ButtonVariant::Solid,
            color: self.confirm_color,
            is_primary: true,
            close_on_press: true,
        }];

        if self.show_cancel {
            buttons.push(DialogButton::cancel(self.cancel_label.clone()));
        }

        Dialog {
            id: self.id.clone(),
            title: Some(self.title.clone()),
            description: self.description.clone(),
            buttons,
            ..Default::default()
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::light_theme;

    // ==========================================================================
    // Button Tests
    // ==========================================================================

    #[test]
    fn test_button_new() {
        let button = Button::new("Click me");
        assert_eq!(button.label, "Click me");
        assert_eq!(button.variant, ButtonVariant::Solid);
        assert_eq!(button.color, ButtonColor::Primary);
        assert!(!button.disabled);
    }

    #[test]
    fn test_button_builder() {
        let button = Button::new("Submit")
            .with_variant(ButtonVariant::Outline)
            .with_color(ButtonColor::Negative)
            .with_size(ButtonSize::Small)
            .disabled(true);

        assert_eq!(button.variant, ButtonVariant::Outline);
        assert_eq!(button.color, ButtonColor::Negative);
        assert_eq!(button.size, ButtonSize::Small);
        assert!(button.disabled);
    }

    #[test]
    fn test_button_computed_styles() {
        let button = Button::new("Test").with_color(ButtonColor::Primary);
        let theme = light_theme();
        let styles = button.computed_styles(&theme);

        // Primary solid button should have primary_500 background
        assert_eq!(styles.background, theme.palette.primary.primary_500);
        assert_eq!(styles.text_color, theme.palette.white);
    }

    #[test]
    fn test_button_disabled_styles() {
        let button = Button::new("Test")
            .with_color(ButtonColor::Primary)
            .disabled(true);
        let theme = light_theme();
        let styles = button.computed_styles(&theme);

        // Disabled should have lower opacity
        assert!(styles.opacity < 1.0);
    }

    #[test]
    fn test_button_serialization() {
        let button = Button::new("Click")
            .with_color(ButtonColor::Secondary)
            .on_press("handleClick");

        let json = serde_json::to_string(&button).unwrap();
        let deserialized: Button = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.label, "Click");
        assert_eq!(deserialized.color, ButtonColor::Secondary);
        assert_eq!(deserialized.on_press, Some("handleClick".to_string()));
    }

    // ==========================================================================
    // Text Tests
    // ==========================================================================

    #[test]
    fn test_text_new() {
        let text = Text::new("Hello world");
        assert_eq!(text.content, "Hello world");
        assert_eq!(text.variant, TypographyVariant::Md);
    }

    #[test]
    fn test_text_heading() {
        let h1 = Text::heading("Title", 1);
        assert_eq!(h1.role, TextRole::H1);

        let h2 = Text::heading("Subtitle", 2);
        assert_eq!(h2.role, TextRole::H2);
    }

    #[test]
    fn test_text_serialization() {
        let text = Text::new("Test")
            .with_variant(TypographyVariant::TitleLg)
            .with_align(TextAlign::Center);

        let json = serde_json::to_string(&text).unwrap();
        let deserialized: Text = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.content, "Test");
        assert_eq!(deserialized.align, TextAlign::Center);
    }

    // ==========================================================================
    // Container Tests
    // ==========================================================================

    #[test]
    fn test_container_new() {
        let container = Container::new();
        assert_eq!(container.direction, FlexDirection::Row);
    }

    #[test]
    fn test_container_row() {
        let row = Container::row()
            .with_gap(10.0)
            .with_justify(JustifyContent::SpaceBetween);

        assert_eq!(row.direction, FlexDirection::Row);
        assert_eq!(row.gap, Some(10.0));
        assert_eq!(row.justify, JustifyContent::SpaceBetween);
    }

    #[test]
    fn test_container_column() {
        let col = Container::column().with_align(Alignment::Center);

        assert_eq!(col.direction, FlexDirection::Column);
        assert_eq!(col.align, Alignment::Center);
    }

    // ==========================================================================
    // Input Tests
    // ==========================================================================

    #[test]
    fn test_input_new() {
        let input = Input::new();
        assert_eq!(input.input_type, InputType::Text);
        assert!(!input.disabled);
        assert!(!input.required);
    }

    #[test]
    fn test_input_password() {
        let input = Input::password();
        assert_eq!(input.input_type, InputType::Password);
    }

    #[test]
    fn test_input_email() {
        let input = Input::email();
        assert_eq!(input.input_type, InputType::Email);
        assert!(input.autocomplete.is_some());
    }

    #[test]
    fn test_input_with_validation() {
        let input = Input::new().with_error("Invalid email format");

        assert_eq!(input.validation, ValidationState::Invalid);
        assert!(input.error.is_some());
    }

    #[test]
    fn test_input_computed_height() {
        let small = Input::new();
        let large = Input {
            size: InputSize::Large,
            ..Input::new()
        };

        assert!(large.computed_height() > small.computed_height());
    }

    // ==========================================================================
    // Icon Tests
    // ==========================================================================

    #[test]
    fn test_icon_new() {
        let icon = Icon::new("heart");
        assert_eq!(icon.name, "heart");
        assert_eq!(icon.size, IconSize::Md);
    }

    #[test]
    fn test_icon_sizes() {
        assert!(IconSize::Xs.pixels() < IconSize::Sm.pixels());
        assert!(IconSize::Sm.pixels() < IconSize::Md.pixels());
        assert!(IconSize::Md.pixels() < IconSize::Lg.pixels());
        assert!(IconSize::Lg.pixels() < IconSize::Xl.pixels());
        assert!(IconSize::Xl.pixels() < IconSize::Xxl.pixels());
    }

    #[test]
    fn test_icon_serialization() {
        let icon = Icon::new("settings")
            .with_size(IconSize::Lg)
            .with_color("#FF0000");

        let json = serde_json::to_string(&icon).unwrap();
        let deserialized: Icon = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "settings");
        assert_eq!(deserialized.size, IconSize::Lg);
        assert_eq!(deserialized.color, Some("#FF0000".to_string()));
    }

    // ==========================================================================
    // Divider Tests
    // ==========================================================================

    #[test]
    fn test_divider_horizontal() {
        let divider = Divider::horizontal();
        assert_eq!(divider.orientation, DividerOrientation::Horizontal);
        assert_eq!(divider.thickness, 1.0);
    }

    #[test]
    fn test_divider_vertical() {
        let divider = Divider::vertical()
            .with_thickness(2.0)
            .with_color("#CCCCCC");

        assert_eq!(divider.orientation, DividerOrientation::Vertical);
        assert_eq!(divider.thickness, 2.0);
        assert_eq!(divider.color, Some("#CCCCCC".to_string()));
    }

    // ==========================================================================
    // Spacing Tests
    // ==========================================================================

    #[test]
    fn test_spacing_uniform() {
        let spacing = Spacing::uniform(16.0);
        match spacing {
            Spacing::Uniform(v) => assert_eq!(v, 16.0),
            _ => panic!("Expected uniform spacing"),
        }
    }

    #[test]
    fn test_spacing_symmetric() {
        let spacing = Spacing::symmetric(10.0, 20.0);
        match spacing {
            Spacing::Symmetric {
                vertical,
                horizontal,
            } => {
                assert_eq!(vertical, 10.0);
                assert_eq!(horizontal, 20.0);
            }
            _ => panic!("Expected symmetric spacing"),
        }
    }

    // ==========================================================================
    // Dimension Tests
    // ==========================================================================

    #[test]
    fn test_dimension_pixels() {
        let dim = Dimension::px(100.0);
        match dim {
            Dimension::Pixels(v) => assert_eq!(v, 100.0),
            _ => panic!("Expected pixels"),
        }
    }

    #[test]
    fn test_dimension_percent() {
        let dim = Dimension::percent(50.0);
        match dim {
            Dimension::Percent(s) => assert_eq!(s, "50%"),
            _ => panic!("Expected percent"),
        }
    }

    // ==========================================================================
    // Tab Bar Tests
    // ==========================================================================

    #[test]
    fn test_tab_bar_new() {
        let tab_bar = TabBar::new();
        assert_eq!(tab_bar.items.len(), 5);
        assert_eq!(tab_bar.items[0].id, "home");
        assert_eq!(tab_bar.items[1].id, "search");
        assert_eq!(tab_bar.items[2].id, "messages");
        assert_eq!(tab_bar.items[3].id, "notifications");
        assert_eq!(tab_bar.items[4].id, "profile");
    }

    #[test]
    fn test_tab_bar_set_active() {
        let tab_bar = TabBar::new().set_active("search");
        assert_eq!(tab_bar.active_tab(), Some("search"));
        assert!(!tab_bar.items[0].is_active);
        assert!(tab_bar.items[1].is_active);
    }

    #[test]
    fn test_tab_bar_badge() {
        let tab_bar = TabBar::new()
            .set_badge("notifications", Some(5))
            .set_has_new("messages", true);

        assert_eq!(tab_bar.items[3].badge_count, Some(5));
        assert!(tab_bar.items[2].has_new);
    }

    #[test]
    fn test_tab_bar_styles() {
        let theme = light_theme();
        let tab_bar = TabBar::new().with_safe_area(20.0);
        let styles = tab_bar.computed_styles(&theme);

        assert!(styles.padding_bottom >= 15.0);
        assert!(styles.border_width > 0.0);
    }

    #[test]
    fn test_tab_bar_item_icon() {
        let item = TabBarItem::new("home", "home", "home-filled", "Home");
        assert_eq!(item.current_icon(), "home");

        let active_item = item.with_active(true);
        assert_eq!(active_item.current_icon(), "home-filled");
    }

    #[test]
    fn test_tab_bar_serialization() {
        let tab_bar = TabBar::new().set_active("home");
        let json = serde_json::to_string(&tab_bar).unwrap();
        let deserialized: TabBar = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.items.len(), 5);
        assert_eq!(deserialized.active_tab(), Some("home"));
    }

    // ==========================================================================
    // Avatar Tests
    // ==========================================================================

    #[test]
    fn test_avatar_new() {
        let avatar = Avatar::new("User Avatar");
        assert_eq!(avatar.alt, "User Avatar");
        assert_eq!(avatar.size, AvatarSize::Md);
        assert_eq!(avatar.shape, AvatarShape::Circle);
    }

    #[test]
    fn test_avatar_sizes() {
        assert!(AvatarSize::Xs.pixels() < AvatarSize::Sm.pixels());
        assert!(AvatarSize::Sm.pixels() < AvatarSize::Md.pixels());
        assert!(AvatarSize::Md.pixels() < AvatarSize::Lg.pixels());
        assert!(AvatarSize::Lg.pixels() < AvatarSize::Xl.pixels());
    }

    #[test]
    fn test_avatar_styles() {
        let theme = light_theme();
        let avatar = Avatar::new("Test")
            .with_size(AvatarSize::Lg)
            .with_shape(AvatarShape::Circle);

        let styles = avatar.computed_styles(&theme);
        assert_eq!(styles.size, 48.0);
        assert_eq!(styles.border_radius, 24.0); // Half of 48
    }

    #[test]
    fn test_avatar_square_shape() {
        let theme = light_theme();
        let avatar = Avatar::new("Labeler")
            .with_shape(AvatarShape::Square);

        let styles = avatar.computed_styles(&theme);
        assert_eq!(styles.border_radius, 8.0);
    }

    // ==========================================================================
    // Badge Tests
    // ==========================================================================

    #[test]
    fn test_badge_count() {
        let badge = Badge::count(5);
        assert_eq!(badge.display_content(), Some("5".to_string()));
    }

    #[test]
    fn test_badge_max() {
        let badge = Badge::count(150).with_max(99);
        assert_eq!(badge.display_content(), Some("99+".to_string()));
    }

    #[test]
    fn test_badge_dot() {
        let badge = Badge::dot();
        assert!(badge.dot);
        assert_eq!(badge.display_content(), None);
    }

    #[test]
    fn test_badge_styles() {
        let theme = light_theme();
        let badge = Badge::count(10);
        let styles = badge.computed_styles(&theme);

        assert_eq!(styles.size, 18.0);
        assert_eq!(styles.font_size, 12.0);
    }

    #[test]
    fn test_badge_dot_styles() {
        let theme = light_theme();
        let badge = Badge::dot();
        let styles = badge.computed_styles(&theme);

        assert_eq!(styles.size, 8.0);
        assert_eq!(styles.padding_horizontal, 0.0);
    }

    // ==========================================================================
    // Loader Tests
    // ==========================================================================

    #[test]
    fn test_loader_new() {
        let loader = Loader::new();
        assert_eq!(loader.size, LoaderSize::Md);
        assert_eq!(loader.duration_ms, 500);
    }

    #[test]
    fn test_loader_sizes() {
        assert!(LoaderSize::Sm.pixels() < LoaderSize::Md.pixels());
        assert!(LoaderSize::Md.pixels() < LoaderSize::Lg.pixels());
        assert!(LoaderSize::Lg.pixels() < LoaderSize::Xl.pixels());
    }

    #[test]
    fn test_loader_styles() {
        let theme = light_theme();
        let loader = Loader::new().with_size(LoaderSize::Lg);
        let styles = loader.computed_styles(&theme);

        assert_eq!(styles.size, 32.0);
        assert!(styles.stroke_width >= 2.0);
    }

    // ==========================================================================
    // Skeleton Tests
    // ==========================================================================

    #[test]
    fn test_skeleton_new() {
        let skeleton = Skeleton::new(100.0, 16.0);
        assert_eq!(skeleton.height, 16.0);
        assert!(skeleton.animated);
    }

    #[test]
    fn test_skeleton_avatar() {
        let skeleton = Skeleton::avatar(48.0);
        assert_eq!(skeleton.shape, SkeletonShape::Circle);
        assert_eq!(skeleton.height, 48.0);
    }

    #[test]
    fn test_skeleton_text_line() {
        let skeleton = Skeleton::text_line(80.0, 6.0);
        assert_eq!(skeleton.shape, SkeletonShape::Rectangle);
        assert_eq!(skeleton.border_radius, Some(4.0));
    }

    #[test]
    fn test_skeleton_styles() {
        let theme = light_theme();
        let skeleton = Skeleton::avatar(40.0);
        let styles = skeleton.computed_styles(&theme);

        assert_eq!(styles.border_radius, 20.0); // Half of 40
        assert!(styles.animation_duration_ms > 0);
    }

    #[test]
    fn test_skeleton_no_animation() {
        let theme = light_theme();
        let skeleton = Skeleton::new(100.0, 16.0).with_animation(false);
        let styles = skeleton.computed_styles(&theme);

        assert_eq!(styles.animation_duration_ms, 0);
    }

    // ==========================================================================
    // Skeleton Layout Tests
    // ==========================================================================

    #[test]
    fn test_post_skeleton_elements() {
        let skeleton = PostSkeleton::new().with_lines(3);
        let elements = skeleton.elements();

        // Should have avatar + username + 3 text lines = 5 elements
        assert_eq!(elements.len(), 5);
    }

    #[test]
    fn test_profile_card_skeleton_elements() {
        let skeleton = ProfileCardSkeleton::new();
        let elements = skeleton.elements();

        // Should have avatar + display name + handle + bio = 4 elements
        assert_eq!(elements.len(), 4);
    }

    #[test]
    fn test_notification_skeleton_elements() {
        let skeleton = NotificationSkeleton::new();
        let elements = skeleton.elements();

        assert!(!elements.is_empty());
    }

    #[test]
    fn test_chat_list_skeleton_elements() {
        let skeleton = ChatListSkeleton::new();
        let elements = skeleton.elements();

        assert!(!elements.is_empty());
    }

    // ==========================================================================
    // Loading State Tests
    // ==========================================================================

    #[test]
    fn test_loading_state_idle() {
        let state: LoadingState<String> = LoadingState::idle();
        assert!(!state.is_loading());
        assert!(!state.has_data());
        assert!(!state.is_error());
    }

    #[test]
    fn test_loading_state_loading() {
        let state: LoadingState<String> = LoadingState::loading();
        assert!(state.is_loading());
        assert!(!state.has_data());
    }

    #[test]
    fn test_loading_state_success() {
        let state = LoadingState::success("data".to_string());
        assert!(!state.is_loading());
        assert!(state.has_data());
        assert_eq!(state.data(), Some(&"data".to_string()));
    }

    #[test]
    fn test_loading_state_error() {
        let state: LoadingState<String> = LoadingState::error("Failed to load");
        assert!(state.is_error());
        assert_eq!(state.error_message(), Some("Failed to load"));
    }

    #[test]
    fn test_loading_state_refreshing() {
        let state = LoadingState::refreshing("old data".to_string());
        assert!(state.has_data());
        assert_eq!(state.data(), Some(&"old data".to_string()));
    }

    // ==========================================================================
    // Toast Tests
    // ==========================================================================

    #[test]
    fn test_toast_new() {
        let toast = Toast::new("Hello world");
        assert_eq!(toast.message, "Hello world");
        assert_eq!(toast.toast_type, ToastType::Default);
        assert!(toast.dismissible);
        assert_eq!(toast.duration, Some(DEFAULT_TOAST_DURATION));
    }

    #[test]
    fn test_toast_variants() {
        let success = Toast::success("Success!");
        assert_eq!(success.toast_type, ToastType::Success);

        let error = Toast::error("Error!");
        assert_eq!(error.toast_type, ToastType::Error);

        let warning = Toast::warning("Warning!");
        assert_eq!(warning.toast_type, ToastType::Warning);

        let info = Toast::info("Info!");
        assert_eq!(info.toast_type, ToastType::Info);
    }

    #[test]
    fn test_toast_persistent() {
        let toast = Toast::new("Persistent").persistent();
        assert_eq!(toast.duration, None);
        assert!(!toast.should_auto_dismiss());
    }

    #[test]
    fn test_toast_action() {
        let toast = Toast::new("Action").with_action("Undo", "undo_action");
        assert!(toast.action.is_some());
        let action = toast.action.unwrap();
        assert_eq!(action.label, "Undo");
        assert_eq!(action.action_id, "undo_action");
    }

    #[test]
    fn test_toast_icon() {
        let toast = Toast::new("Test");
        assert_eq!(toast.display_icon(), "circle-check");

        let custom = Toast::new("Custom").with_icon("heart");
        assert_eq!(custom.display_icon(), "heart");
    }

    #[test]
    fn test_toast_styles() {
        let theme = light_theme();
        let toast = Toast::success("Test");
        let styles = toast.computed_styles(&theme);

        assert!(styles.border_radius > 0.0);
        assert!(styles.padding > 0.0);
    }

    #[test]
    fn test_toast_queue_new() {
        let queue = ToastQueue::new();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.max_visible, 3);
    }

    #[test]
    fn test_toast_queue_push_dismiss() {
        let mut queue = ToastQueue::new();
        let toast = Toast::new("Test");
        let id = toast.id.clone();

        queue.push(toast);
        assert_eq!(queue.len(), 1);
        assert!(!queue.is_empty());

        let dismissed = queue.dismiss(&id);
        assert!(dismissed.is_some());
        assert!(queue.is_empty());
    }

    #[test]
    fn test_toast_queue_visible() {
        let mut queue = ToastQueue::new().with_max_visible(2);

        queue.push(Toast::new("1"));
        queue.push(Toast::new("2"));
        queue.push(Toast::new("3"));

        assert_eq!(queue.len(), 3);
        assert_eq!(queue.visible().len(), 2);
    }

    #[test]
    fn test_toast_queue_clear() {
        let mut queue = ToastQueue::new();
        queue.push(Toast::new("1"));
        queue.push(Toast::new("2"));

        queue.clear();
        assert!(queue.is_empty());
    }

    #[test]
    fn test_toast_type_icons() {
        assert_eq!(ToastType::Default.icon(), "circle-check");
        assert_eq!(ToastType::Success.icon(), "circle-check");
        assert_eq!(ToastType::Error.icon(), "circle-info");
        assert_eq!(ToastType::Warning.icon(), "warning");
        assert_eq!(ToastType::Info.icon(), "circle-info");
    }

    // ==========================================================================
    // Dialog Tests
    // ==========================================================================

    #[test]
    fn test_dialog_new() {
        let dialog = Dialog::new();
        assert!(!dialog.is_open);
        assert!(dialog.dismiss_on_backdrop);
        assert_eq!(dialog.size, DialogSize::Medium);
    }

    #[test]
    fn test_dialog_alert() {
        let dialog = Dialog::alert("Title", "Message");
        assert_eq!(dialog.title, Some("Title".to_string()));
        assert_eq!(dialog.description, Some("Message".to_string()));
        assert_eq!(dialog.buttons.len(), 1);
        assert_eq!(dialog.buttons[0].label, "OK");
    }

    #[test]
    fn test_dialog_confirm() {
        let dialog = Dialog::confirm("Title", "Message");
        assert_eq!(dialog.buttons.len(), 2);
        assert!(dialog.buttons[0].is_primary);
    }

    #[test]
    fn test_dialog_destructive() {
        let dialog = Dialog::destructive("Delete?", "This cannot be undone.");
        assert_eq!(dialog.buttons.len(), 2);
        assert_eq!(dialog.buttons[0].color, ButtonColor::Negative);
    }

    #[test]
    fn test_dialog_open_close() {
        let mut dialog = Dialog::new();
        assert!(!dialog.is_open);

        dialog.open();
        assert!(dialog.is_open);

        dialog.close();
        assert!(!dialog.is_open);
    }

    #[test]
    fn test_dialog_size() {
        assert_eq!(DialogSize::Small.max_width(), Some(300.0));
        assert_eq!(DialogSize::Medium.max_width(), Some(400.0));
        assert_eq!(DialogSize::Large.max_width(), Some(600.0));
        assert_eq!(DialogSize::Full.max_width(), None);
    }

    #[test]
    fn test_dialog_styles() {
        let theme = light_theme();
        let dialog = Dialog::new();
        let styles = dialog.computed_styles(&theme);

        assert_eq!(styles.max_width, Some(400.0));
        assert!(styles.border_radius > 0.0);
        assert!(styles.padding > 0.0);
    }

    #[test]
    fn test_dialog_button_confirm() {
        let btn = DialogButton::confirm("OK");
        assert_eq!(btn.label, "OK");
        assert_eq!(btn.action_id, "confirm");
        assert!(btn.is_primary);
        assert!(btn.close_on_press);
    }

    #[test]
    fn test_dialog_button_cancel() {
        let btn = DialogButton::cancel("Cancel");
        assert_eq!(btn.color, ButtonColor::Secondary);
        assert!(!btn.is_primary);
    }

    #[test]
    fn test_dialog_controller() {
        let mut controller = DialogController::new();
        assert!(!controller.has_open());
        assert_eq!(controller.count(), 0);

        controller.show(Dialog::alert("Test", "Message"));
        assert!(controller.has_open());
        assert_eq!(controller.count(), 1);

        controller.close_top();
        assert!(!controller.has_open());
    }

    #[test]
    fn test_dialog_controller_close_by_id() {
        let mut controller = DialogController::new();
        let dialog = Dialog::alert("Test", "Message");
        let id = dialog.id.clone();

        controller.show(dialog);
        let closed = controller.close(&id);
        assert!(closed.is_some());
        assert!(!controller.has_open());
    }

    #[test]
    fn test_confirm_prompt() {
        let prompt = ConfirmPrompt::new("Delete item?")
            .with_description("This action cannot be undone.")
            .with_confirm_label("Delete")
            .with_confirm_color(ButtonColor::Negative);

        assert_eq!(prompt.title, "Delete item?");
        assert_eq!(prompt.confirm_label, "Delete");
        assert_eq!(prompt.confirm_color, ButtonColor::Negative);
    }

    #[test]
    fn test_confirm_prompt_to_dialog() {
        let prompt = ConfirmPrompt::new("Title")
            .with_description("Description");

        let dialog = prompt.to_dialog();
        assert_eq!(dialog.title, Some("Title".to_string()));
        assert_eq!(dialog.description, Some("Description".to_string()));
        assert_eq!(dialog.buttons.len(), 2); // confirm + cancel
    }

    #[test]
    fn test_confirm_prompt_without_cancel() {
        let prompt = ConfirmPrompt::new("Alert").without_cancel();
        let dialog = prompt.to_dialog();

        assert_eq!(dialog.buttons.len(), 1);
    }
}
