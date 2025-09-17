//! Color palette definitions and utilities
//!
//! This module provides structured color palettes for different themes
//! with accessibility considerations and semantic color names.

use ratatui::style::Color;

/// Semantic color roles for consistent theming
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorRole {
    // Base roles
    Primary,
    Secondary,
    Accent,
    Background,
    Surface,
    OnBackground,
    OnSurface,

    // Status roles
    Success,
    Warning,
    Error,
    Info,

    // State roles
    Pending,
    Processing,
    Completed,
    Failed,
    Skipped,

    // Interactive roles
    Focus,
    Selected,
    Hover,
    Pressed,
    Disabled,

    // Text roles
    TextPrimary,
    TextSecondary,
    TextMuted,
    TextInverse,

    // Border and outline roles
    BorderPrimary,
    BorderSecondary,
    Outline,
    Divider,
}

/// Color palette with semantic naming
#[derive(Debug, Clone)]
pub struct ColorPalette {
    colors: std::collections::HashMap<ColorRole, Color>,
}

impl ColorPalette {
    /// Create a new empty palette
    pub fn new() -> Self {
        Self {
            colors: std::collections::HashMap::new(),
        }
    }

    /// Set a color for a specific role
    pub fn set(&mut self, role: ColorRole, color: Color) {
        self.colors.insert(role, color);
    }

    /// Get a color for a specific role
    pub fn get(&self, role: ColorRole) -> Color {
        self.colors.get(&role).copied().unwrap_or(Color::Reset)
    }

    /// Dark theme palette optimized for readability
    pub fn dark() -> Self {
        let mut palette = Self::new();

        // Base colors - dark background with light text
        palette.set(ColorRole::Background, Color::Rgb(26, 27, 38));      // Very dark blue-gray
        palette.set(ColorRole::Surface, Color::Rgb(40, 42, 54));         // Dark blue-gray
        palette.set(ColorRole::OnBackground, Color::Rgb(248, 248, 242)); // Off-white
        palette.set(ColorRole::OnSurface, Color::Rgb(248, 248, 242));    // Off-white

        // Primary brand colors
        palette.set(ColorRole::Primary, Color::Rgb(139, 233, 253));      // Cyan
        palette.set(ColorRole::Secondary, Color::Rgb(80, 250, 123));     // Green
        palette.set(ColorRole::Accent, Color::Rgb(255, 184, 108));       // Orange

        // Status colors with good contrast
        palette.set(ColorRole::Success, Color::Rgb(80, 250, 123));       // Green
        palette.set(ColorRole::Warning, Color::Rgb(255, 184, 108));      // Orange
        palette.set(ColorRole::Error, Color::Rgb(255, 85, 85));          // Red
        palette.set(ColorRole::Info, Color::Rgb(139, 233, 253));         // Cyan

        // State colors for track status
        palette.set(ColorRole::Pending, Color::Rgb(98, 114, 164));       // Muted blue
        palette.set(ColorRole::Processing, Color::Rgb(139, 233, 253));   // Cyan
        palette.set(ColorRole::Completed, Color::Rgb(80, 250, 123));     // Green
        palette.set(ColorRole::Failed, Color::Rgb(255, 85, 85));         // Red
        palette.set(ColorRole::Skipped, Color::Rgb(241, 250, 140));      // Yellow

        // Interactive states
        palette.set(ColorRole::Focus, Color::Rgb(255, 184, 108));        // Orange
        palette.set(ColorRole::Selected, Color::Rgb(68, 71, 90));        // Dark blue-gray
        palette.set(ColorRole::Hover, Color::Rgb(54, 58, 79));           // Slightly lighter surface
        palette.set(ColorRole::Pressed, Color::Rgb(33, 34, 44));         // Darker than surface
        palette.set(ColorRole::Disabled, Color::Rgb(98, 114, 164));      // Muted blue

        // Text variations
        palette.set(ColorRole::TextPrimary, Color::Rgb(248, 248, 242));  // Primary text
        palette.set(ColorRole::TextSecondary, Color::Rgb(191, 191, 191)); // Secondary text
        palette.set(ColorRole::TextMuted, Color::Rgb(98, 114, 164));     // Muted text
        palette.set(ColorRole::TextInverse, Color::Rgb(26, 27, 38));     // Text on light backgrounds

        // Borders and dividers
        palette.set(ColorRole::BorderPrimary, Color::Rgb(68, 71, 90));   // Primary borders
        palette.set(ColorRole::BorderSecondary, Color::Rgb(54, 58, 79)); // Secondary borders
        palette.set(ColorRole::Outline, Color::Rgb(139, 233, 253));     // Focus outlines
        palette.set(ColorRole::Divider, Color::Rgb(54, 58, 79));        // Divider lines

        palette
    }

    /// Light theme palette optimized for readability
    pub fn light() -> Self {
        let mut palette = Self::new();

        // Base colors - light background with dark text
        palette.set(ColorRole::Background, Color::Rgb(255, 255, 255));   // White
        palette.set(ColorRole::Surface, Color::Rgb(248, 248, 248));      // Light gray
        palette.set(ColorRole::OnBackground, Color::Rgb(33, 37, 43));    // Very dark gray
        palette.set(ColorRole::OnSurface, Color::Rgb(33, 37, 43));       // Very dark gray

        // Primary brand colors - darker for contrast
        palette.set(ColorRole::Primary, Color::Rgb(0, 122, 204));        // Blue
        palette.set(ColorRole::Secondary, Color::Rgb(40, 167, 69));      // Green
        palette.set(ColorRole::Accent, Color::Rgb(255, 133, 27));        // Orange

        // Status colors with good contrast on light background
        palette.set(ColorRole::Success, Color::Rgb(40, 167, 69));        // Green
        palette.set(ColorRole::Warning, Color::Rgb(255, 133, 27));       // Orange
        palette.set(ColorRole::Error, Color::Rgb(220, 53, 69));          // Red
        palette.set(ColorRole::Info, Color::Rgb(23, 162, 184));          // Teal

        // State colors for track status
        palette.set(ColorRole::Pending, Color::Rgb(108, 117, 125));      // Gray
        palette.set(ColorRole::Processing, Color::Rgb(23, 162, 184));    // Teal
        palette.set(ColorRole::Completed, Color::Rgb(40, 167, 69));      // Green
        palette.set(ColorRole::Failed, Color::Rgb(220, 53, 69));         // Red
        palette.set(ColorRole::Skipped, Color::Rgb(255, 193, 7));        // Yellow

        // Interactive states
        palette.set(ColorRole::Focus, Color::Rgb(0, 123, 255));          // Blue
        palette.set(ColorRole::Selected, Color::Rgb(0, 123, 255));       // Blue
        palette.set(ColorRole::Hover, Color::Rgb(233, 236, 239));        // Light gray
        palette.set(ColorRole::Pressed, Color::Rgb(222, 226, 230));      // Darker gray
        palette.set(ColorRole::Disabled, Color::Rgb(173, 181, 189));     // Muted gray

        // Text variations
        palette.set(ColorRole::TextPrimary, Color::Rgb(33, 37, 43));     // Primary text
        palette.set(ColorRole::TextSecondary, Color::Rgb(108, 117, 125)); // Secondary text
        palette.set(ColorRole::TextMuted, Color::Rgb(173, 181, 189));    // Muted text
        palette.set(ColorRole::TextInverse, Color::Rgb(255, 255, 255));  // Text on dark backgrounds

        // Borders and dividers
        palette.set(ColorRole::BorderPrimary, Color::Rgb(222, 226, 230)); // Primary borders
        palette.set(ColorRole::BorderSecondary, Color::Rgb(233, 236, 239)); // Secondary borders
        palette.set(ColorRole::Outline, Color::Rgb(0, 123, 255));        // Focus outlines
        palette.set(ColorRole::Divider, Color::Rgb(233, 236, 239));      // Divider lines

        palette
    }

    /// High contrast palette for accessibility
    pub fn high_contrast() -> Self {
        let mut palette = Self::new();

        // Maximum contrast base colors
        palette.set(ColorRole::Background, Color::Rgb(0, 0, 0));         // Black
        palette.set(ColorRole::Surface, Color::Rgb(0, 0, 0));            // Black
        palette.set(ColorRole::OnBackground, Color::Rgb(255, 255, 255)); // White
        palette.set(ColorRole::OnSurface, Color::Rgb(255, 255, 255));    // White

        // High contrast brand colors
        palette.set(ColorRole::Primary, Color::Rgb(255, 255, 255));      // White
        palette.set(ColorRole::Secondary, Color::Rgb(255, 255, 0));      // Yellow
        palette.set(ColorRole::Accent, Color::Rgb(0, 255, 255));         // Cyan

        // Maximum contrast status colors
        palette.set(ColorRole::Success, Color::Rgb(0, 255, 0));          // Lime
        palette.set(ColorRole::Warning, Color::Rgb(255, 255, 0));        // Yellow
        palette.set(ColorRole::Error, Color::Rgb(255, 0, 0));            // Red
        palette.set(ColorRole::Info, Color::Rgb(0, 255, 255));           // Cyan

        // High contrast state colors
        palette.set(ColorRole::Pending, Color::Rgb(192, 192, 192));      // Light gray
        palette.set(ColorRole::Processing, Color::Rgb(0, 255, 255));     // Cyan
        palette.set(ColorRole::Completed, Color::Rgb(0, 255, 0));        // Lime
        palette.set(ColorRole::Failed, Color::Rgb(255, 0, 0));           // Red
        palette.set(ColorRole::Skipped, Color::Rgb(255, 255, 0));        // Yellow

        // High contrast interactive states
        palette.set(ColorRole::Focus, Color::Rgb(255, 255, 0));          // Yellow
        palette.set(ColorRole::Selected, Color::Rgb(255, 255, 255));     // White
        palette.set(ColorRole::Hover, Color::Rgb(64, 64, 64));           // Dark gray
        palette.set(ColorRole::Pressed, Color::Rgb(32, 32, 32));         // Darker gray
        palette.set(ColorRole::Disabled, Color::Rgb(128, 128, 128));     // Gray

        // High contrast text
        palette.set(ColorRole::TextPrimary, Color::Rgb(255, 255, 255));  // White
        palette.set(ColorRole::TextSecondary, Color::Rgb(255, 255, 255)); // White
        palette.set(ColorRole::TextMuted, Color::Rgb(192, 192, 192));    // Light gray
        palette.set(ColorRole::TextInverse, Color::Rgb(0, 0, 0));        // Black

        // High contrast borders
        palette.set(ColorRole::BorderPrimary, Color::Rgb(255, 255, 255)); // White
        palette.set(ColorRole::BorderSecondary, Color::Rgb(255, 255, 255)); // White
        palette.set(ColorRole::Outline, Color::Rgb(255, 255, 0));        // Yellow
        palette.set(ColorRole::Divider, Color::Rgb(255, 255, 255));      // White

        palette
    }

    /// Create a custom palette from base colors
    pub fn custom(
        background: Color,
        surface: Color,
        primary: Color,
        secondary: Color,
        accent: Color,
    ) -> Self {
        let mut palette = Self::new();

        // Set base colors
        palette.set(ColorRole::Background, background);
        palette.set(ColorRole::Surface, surface);
        palette.set(ColorRole::Primary, primary);
        palette.set(ColorRole::Secondary, secondary);
        palette.set(ColorRole::Accent, accent);

        // Generate derived colors based on luminance
        let is_dark = Self::is_dark_color(background);
        let text_color = if is_dark {
            Color::Rgb(248, 248, 242) // Light text for dark background
        } else {
            Color::Rgb(33, 37, 43) // Dark text for light background
        };

        palette.set(ColorRole::OnBackground, text_color);
        palette.set(ColorRole::OnSurface, text_color);
        palette.set(ColorRole::TextPrimary, text_color);

        // Set default status colors
        palette.set(ColorRole::Success, Color::Rgb(if is_dark { 80 } else { 40 }, if is_dark { 250 } else { 167 }, if is_dark { 123 } else { 69 }));
        palette.set(ColorRole::Warning, Color::Rgb(255, if is_dark { 184 } else { 133 }, if is_dark { 108 } else { 27 }));
        palette.set(ColorRole::Error, Color::Rgb(255, if is_dark { 85 } else { 53 }, if is_dark { 85 } else { 69 }));
        palette.set(ColorRole::Info, primary);

        palette
    }

    /// Check if a color is considered dark (simple luminance approximation)
    fn is_dark_color(color: Color) -> bool {
        match color {
            Color::Rgb(r, g, b) => {
                // Simple luminance calculation (ITU-R BT.709)
                let luminance = 0.2126 * (r as f32) + 0.7152 * (g as f32) + 0.0722 * (b as f32);
                luminance < 128.0
            }
            Color::Black | Color::DarkGray => true,
            Color::White | Color::Gray => false,
            _ => true, // Default to dark for other colors
        }
    }

    /// Validate that the palette has sufficient contrast for accessibility
    pub fn validate_contrast(&self) -> Vec<String> {
        let mut issues = Vec::new();

        // Check primary text contrast
        let bg = self.get(ColorRole::Background);
        let text = self.get(ColorRole::TextPrimary);
        if !Self::has_sufficient_contrast(bg, text) {
            issues.push("Insufficient contrast between background and primary text".to_string());
        }

        // Check interactive element contrast
        let focus = self.get(ColorRole::Focus);
        if !Self::has_sufficient_contrast(bg, focus) {
            issues.push("Insufficient contrast for focus indicators".to_string());
        }

        issues
    }

    /// Check if two colors have sufficient contrast (simplified WCAG check)
    fn has_sufficient_contrast(color1: Color, color2: Color) -> bool {
        let (r1, g1, b1) = Self::color_to_rgb(color1);
        let (r2, g2, b2) = Self::color_to_rgb(color2);

        let l1 = Self::relative_luminance(r1, g1, b1);
        let l2 = Self::relative_luminance(r2, g2, b2);

        let contrast_ratio = if l1 > l2 {
            (l1 + 0.05) / (l2 + 0.05)
        } else {
            (l2 + 0.05) / (l1 + 0.05)
        };

        contrast_ratio >= 4.5 // WCAG AA standard for normal text
    }

    fn color_to_rgb(color: Color) -> (u8, u8, u8) {
        match color {
            Color::Rgb(r, g, b) => (r, g, b),
            Color::Black => (0, 0, 0),
            Color::White => (255, 255, 255),
            Color::Gray => (128, 128, 128),
            Color::DarkGray => (64, 64, 64),
            Color::Red => (255, 0, 0),
            Color::Green => (0, 255, 0),
            Color::Blue => (0, 0, 255),
            Color::Yellow => (255, 255, 0),
            Color::Cyan => (0, 255, 255),
            Color::Magenta => (255, 0, 255),
            _ => (128, 128, 128), // Default to gray
        }
    }

    fn relative_luminance(r: u8, g: u8, b: u8) -> f32 {
        let r = Self::linearize_rgb_component(r as f32 / 255.0);
        let g = Self::linearize_rgb_component(g as f32 / 255.0);
        let b = Self::linearize_rgb_component(b as f32 / 255.0);

        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    fn linearize_rgb_component(component: f32) -> f32 {
        if component <= 0.03928 {
            component / 12.92
        } else {
            ((component + 0.055) / 1.055).powf(2.4)
        }
    }
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_palette_creation() {
        let palette = ColorPalette::dark();
        assert_ne!(palette.get(ColorRole::Background), Color::Reset);
        assert_ne!(palette.get(ColorRole::Primary), Color::Reset);
    }

    #[test]
    fn test_light_palette_creation() {
        let palette = ColorPalette::light();
        assert_ne!(palette.get(ColorRole::Background), Color::Reset);
        assert_ne!(palette.get(ColorRole::Primary), Color::Reset);
    }

    #[test]
    fn test_high_contrast_palette() {
        let palette = ColorPalette::high_contrast();
        assert_eq!(palette.get(ColorRole::Background), Color::Rgb(0, 0, 0));
        assert_eq!(palette.get(ColorRole::OnBackground), Color::Rgb(255, 255, 255));
    }

    #[test]
    fn test_color_role_coverage() {
        let palette = ColorPalette::dark();

        // Test that all essential roles have colors
        assert_ne!(palette.get(ColorRole::Primary), Color::Reset);
        assert_ne!(palette.get(ColorRole::Success), Color::Reset);
        assert_ne!(palette.get(ColorRole::Error), Color::Reset);
        assert_ne!(palette.get(ColorRole::Focus), Color::Reset);
    }

    #[test]
    fn test_custom_palette() {
        let palette = ColorPalette::custom(
            Color::Rgb(20, 20, 30),    // Dark background
            Color::Rgb(30, 30, 40),    // Surface
            Color::Rgb(100, 150, 200), // Primary
            Color::Rgb(80, 200, 120),  // Secondary
            Color::Rgb(255, 180, 100), // Accent
        );

        assert_eq!(palette.get(ColorRole::Background), Color::Rgb(20, 20, 30));
        assert_eq!(palette.get(ColorRole::Primary), Color::Rgb(100, 150, 200));
    }

    #[test]
    fn test_contrast_validation() {
        let palette = ColorPalette::high_contrast();
        let issues = palette.validate_contrast();

        // High contrast theme should have no contrast issues
        assert!(issues.is_empty(), "High contrast theme should pass validation");
    }

    #[test]
    fn test_is_dark_color() {
        assert!(ColorPalette::is_dark_color(Color::Rgb(0, 0, 0)));
        assert!(ColorPalette::is_dark_color(Color::Rgb(50, 50, 50)));
        assert!(!ColorPalette::is_dark_color(Color::Rgb(200, 200, 200)));
        assert!(!ColorPalette::is_dark_color(Color::Rgb(255, 255, 255)));
    }
}