//! Theme system for terminal UI
//!
//! Provides color schemes and styling for different terminal environments
//! with automatic detection and accessibility support.

use ratatui::style::{Color, Style, Modifier};
use std::collections::HashMap;
use super::colors::{ColorPalette, ColorRole};
use super::styles::ComponentStyles;

/// Available theme variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeVariant {
    Dark,
    Light,
    HighContrast,
    Auto,
}

/// Complete theme definition
#[derive(Debug, Clone)]
pub struct Theme {
    pub variant: ThemeVariant,
    pub palette: ColorPalette,
    pub styles: ComponentStyles,
    // Backward compatibility
    pub colors: ThemeColors,
}

/// Color palette for the theme
#[derive(Debug, Clone)]
pub struct ThemeColors {
    // Base colors
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub background: Color,
    pub foreground: Color,
    pub border: Color,
    pub muted: Color,

    // Status colors
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub info: Color,

    // State colors
    pub pending: Color,
    pub processing: Color,
    pub completed: Color,
    pub failed: Color,
    pub skipped: Color,

    // UI element colors
    pub header_bg: Color,
    pub footer_bg: Color,
    pub panel_bg: Color,
    pub selected: Color,
    pub highlight: Color,
}

/// Style definitions for UI elements
#[derive(Debug, Clone)]
pub struct ThemeStyles {
    pub header: Style,
    pub footer: Style,
    pub panel_title: Style,
    pub panel_border: Style,
    pub text_normal: Style,
    pub text_muted: Style,
    pub text_bold: Style,
    pub text_emphasis: Style,
    pub progress_bar: Style,
    pub progress_bg: Style,
    pub selected: Style,
    pub error: Style,
    pub warning: Style,
    pub success: Style,
}

impl ThemeColors {
    /// Create ThemeColors from a ColorPalette for backward compatibility
    fn from_palette(palette: &ColorPalette) -> Self {
        Self {
            // Base colors
            primary: palette.get(ColorRole::Primary),
            secondary: palette.get(ColorRole::Secondary),
            accent: palette.get(ColorRole::Accent),
            background: palette.get(ColorRole::Background),
            foreground: palette.get(ColorRole::OnBackground),
            border: palette.get(ColorRole::BorderPrimary),
            muted: palette.get(ColorRole::TextMuted),

            // Status colors
            success: palette.get(ColorRole::Success),
            warning: palette.get(ColorRole::Warning),
            danger: palette.get(ColorRole::Error),
            info: palette.get(ColorRole::Info),

            // State colors
            pending: palette.get(ColorRole::Pending),
            processing: palette.get(ColorRole::Processing),
            completed: palette.get(ColorRole::Completed),
            failed: palette.get(ColorRole::Failed),
            skipped: palette.get(ColorRole::Skipped),

            // UI element colors
            header_bg: palette.get(ColorRole::Primary),
            footer_bg: palette.get(ColorRole::Surface),
            panel_bg: palette.get(ColorRole::Surface),
            selected: palette.get(ColorRole::Selected),
            highlight: palette.get(ColorRole::Focus),
        }
    }
}

impl Theme {
    /// Create dark theme (default)
    pub fn dark() -> Self {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette.clone());

        // Create backward-compatible colors
        let colors = ThemeColors::from_palette(&palette);

        Theme {
            variant: ThemeVariant::Dark,
            palette,
            styles,
            colors,
        }
    }

    /// Create light theme
    pub fn light() -> Self {
        let palette = ColorPalette::light();
        let styles = ComponentStyles::new(palette.clone());
        let colors = ThemeColors::from_palette(&palette);

        Theme {
            variant: ThemeVariant::Light,
            palette,
            styles,
            colors,
        }
    }

    /// Create high contrast theme for accessibility
    pub fn high_contrast() -> Self {
        let palette = ColorPalette::high_contrast();
        let styles = ComponentStyles::new(palette.clone());
        let colors = ThemeColors::from_palette(&palette);

        Theme {
            variant: ThemeVariant::HighContrast,
            palette,
            styles,
            colors,
        }
    }

    /// Auto-detect appropriate theme based on environment
    pub fn auto() -> Self {
        // Try to detect if terminal has light or dark background
        // This is a best-effort detection
        if let Ok(term) = std::env::var("TERM") {
            if term.contains("light") {
                return Self::light();
            }
        }

        // Check for accessibility preferences
        if let Ok(_) = std::env::var("ACCESSIBILITY_HIGH_CONTRAST") {
            return Self::high_contrast();
        }

        // Default to dark theme
        Self::dark()
    }
}

impl ThemeStyles {
    fn from_colors(colors: &ThemeColors) -> Self {
        ThemeStyles {
            header: Style::default()
                .bg(colors.header_bg)
                .fg(colors.foreground)
                .add_modifier(Modifier::BOLD),

            footer: Style::default()
                .bg(colors.footer_bg)
                .fg(colors.foreground),

            panel_title: Style::default()
                .fg(colors.primary)
                .add_modifier(Modifier::BOLD),

            panel_border: Style::default()
                .fg(colors.border),

            text_normal: Style::default()
                .fg(colors.foreground),

            text_muted: Style::default()
                .fg(colors.muted),

            text_bold: Style::default()
                .fg(colors.foreground)
                .add_modifier(Modifier::BOLD),

            text_emphasis: Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),

            progress_bar: Style::default()
                .bg(colors.primary)
                .fg(colors.background),

            progress_bg: Style::default()
                .bg(colors.muted),

            selected: Style::default()
                .bg(colors.selected)
                .fg(colors.background)
                .add_modifier(Modifier::BOLD),

            error: Style::default()
                .fg(colors.danger)
                .add_modifier(Modifier::BOLD),

            warning: Style::default()
                .fg(colors.warning)
                .add_modifier(Modifier::BOLD),

            success: Style::default()
                .fg(colors.success)
                .add_modifier(Modifier::BOLD),
        }
    }
}

/// Theme manager for handling theme selection and application
pub struct ThemeManager {
    current_theme: Theme,
    available_themes: HashMap<String, Theme>,
}

impl ThemeManager {
    pub fn new() -> Self {
        let mut available_themes = HashMap::new();
        available_themes.insert("dark".to_string(), Theme::dark());
        available_themes.insert("light".to_string(), Theme::light());
        available_themes.insert("high_contrast".to_string(), Theme::high_contrast());
        available_themes.insert("auto".to_string(), Theme::auto());

        Self {
            current_theme: Theme::auto(),
            available_themes,
        }
    }

    pub fn current_theme(&self) -> &Theme {
        &self.current_theme
    }

    pub fn set_theme(&mut self, name: &str) -> Result<(), String> {
        if let Some(theme) = self.available_themes.get(name) {
            self.current_theme = theme.clone();
            Ok(())
        } else {
            Err(format!("Theme '{}' not found", name))
        }
    }

    pub fn available_themes(&self) -> Vec<&str> {
        self.available_themes.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}