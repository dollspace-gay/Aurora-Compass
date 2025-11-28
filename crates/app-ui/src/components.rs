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
}
