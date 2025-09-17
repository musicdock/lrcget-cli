//! Component style definitions
//!
//! This module provides predefined styles for common UI components,
//! making it easy to maintain consistent styling across the application.

use ratatui::style::{Style, Modifier};
use super::colors::{ColorPalette, ColorRole};

/// Style definitions for specific UI components
#[derive(Debug, Clone)]
pub struct ComponentStyles {
    palette: ColorPalette,
}

impl ComponentStyles {
    /// Create component styles from a color palette
    pub fn new(palette: ColorPalette) -> Self {
        Self { palette }
    }

    /// Get the underlying color palette
    pub fn palette(&self) -> &ColorPalette {
        &self.palette
    }

    // Header styles
    pub fn header_background(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Primary))
            .fg(self.palette.get(ColorRole::TextInverse))
            .add_modifier(Modifier::BOLD)
    }

    pub fn header_title(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::TextInverse))
            .add_modifier(Modifier::BOLD)
    }

    pub fn header_subtitle(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::TextInverse))
    }

    // Footer styles
    pub fn footer_background(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Surface))
            .fg(self.palette.get(ColorRole::TextPrimary))
    }

    pub fn footer_hotkey(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Accent))
            .add_modifier(Modifier::BOLD)
    }

    pub fn footer_description(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::TextSecondary))
    }

    // Panel styles
    pub fn panel_background(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Surface))
            .fg(self.palette.get(ColorRole::OnSurface))
    }

    pub fn panel_title(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Primary))
            .add_modifier(Modifier::BOLD)
    }

    pub fn panel_border(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::BorderPrimary))
    }

    pub fn panel_border_focused(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Focus))
    }

    // Text styles
    pub fn text_primary(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::TextPrimary))
    }

    pub fn text_secondary(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::TextSecondary))
    }

    pub fn text_muted(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::TextMuted))
    }

    pub fn text_bold(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::TextPrimary))
            .add_modifier(Modifier::BOLD)
    }

    pub fn text_emphasis(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Accent))
            .add_modifier(Modifier::BOLD)
    }

    pub fn text_link(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Primary))
            .add_modifier(Modifier::UNDERLINED)
    }

    // Status text styles
    pub fn text_success(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Success))
            .add_modifier(Modifier::BOLD)
    }

    pub fn text_warning(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Warning))
            .add_modifier(Modifier::BOLD)
    }

    pub fn text_error(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Error))
            .add_modifier(Modifier::BOLD)
    }

    pub fn text_info(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Info))
            .add_modifier(Modifier::BOLD)
    }

    // Input styles
    pub fn input_normal(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Surface))
            .fg(self.palette.get(ColorRole::TextPrimary))
    }

    pub fn input_focused(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Surface))
            .fg(self.palette.get(ColorRole::TextPrimary))
    }

    pub fn input_border(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::BorderPrimary))
    }

    pub fn input_border_focused(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Focus))
    }

    pub fn input_placeholder(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::TextMuted))
            .add_modifier(Modifier::ITALIC)
    }

    pub fn input_cursor(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Focus))
            .fg(self.palette.get(ColorRole::TextInverse))
    }

    // List/Table styles
    pub fn list_item(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::TextPrimary))
    }

    pub fn list_item_selected(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Selected))
            .fg(self.palette.get(ColorRole::TextInverse))
            .add_modifier(Modifier::BOLD)
    }

    pub fn list_item_hover(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Hover))
            .fg(self.palette.get(ColorRole::TextPrimary))
    }

    pub fn table_header(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::TextPrimary))
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    }

    pub fn table_row(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::TextPrimary))
    }

    pub fn table_row_alt(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Surface))
            .fg(self.palette.get(ColorRole::TextPrimary))
    }

    pub fn table_row_selected(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Selected))
            .fg(self.palette.get(ColorRole::TextInverse))
    }

    // Progress styles
    pub fn progress_bar(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Primary))
            .fg(self.palette.get(ColorRole::TextInverse))
    }

    pub fn progress_background(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Surface))
    }

    pub fn progress_text(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::TextPrimary))
            .add_modifier(Modifier::BOLD)
    }

    // State styles for track status
    pub fn state_pending(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Pending))
    }

    pub fn state_processing(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Processing))
            .add_modifier(Modifier::BOLD)
    }

    pub fn state_completed(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Completed))
            .add_modifier(Modifier::BOLD)
    }

    pub fn state_failed(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Failed))
            .add_modifier(Modifier::BOLD)
    }

    pub fn state_skipped(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Skipped))
    }

    // Button styles
    pub fn button_normal(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Primary))
            .fg(self.palette.get(ColorRole::TextInverse))
            .add_modifier(Modifier::BOLD)
    }

    pub fn button_focused(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Focus))
            .fg(self.palette.get(ColorRole::TextInverse))
            .add_modifier(Modifier::BOLD)
    }

    pub fn button_pressed(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Pressed))
            .fg(self.palette.get(ColorRole::TextInverse))
            .add_modifier(Modifier::BOLD)
    }

    pub fn button_disabled(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Disabled))
            .fg(self.palette.get(ColorRole::TextMuted))
    }

    // Chart styles
    pub fn chart_axis(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::BorderPrimary))
    }

    pub fn chart_data_primary(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Primary))
    }

    pub fn chart_data_secondary(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Secondary))
    }

    pub fn chart_data_accent(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Accent))
    }

    // Scrollbar styles
    pub fn scrollbar_track(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::BorderSecondary))
    }

    pub fn scrollbar_thumb(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::BorderPrimary))
    }

    // Badge/Tag styles
    pub fn badge_normal(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Surface))
            .fg(self.palette.get(ColorRole::TextSecondary))
    }

    pub fn badge_success(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Success))
            .fg(self.palette.get(ColorRole::TextInverse))
    }

    pub fn badge_warning(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Warning))
            .fg(self.palette.get(ColorRole::TextInverse))
    }

    pub fn badge_error(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Error))
            .fg(self.palette.get(ColorRole::TextInverse))
    }

    pub fn badge_info(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Info))
            .fg(self.palette.get(ColorRole::TextInverse))
    }

    // Divider/Separator styles
    pub fn divider_horizontal(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Divider))
    }

    pub fn divider_vertical(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Divider))
    }

    // Modal/Dialog styles
    pub fn modal_background(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Surface))
            .fg(self.palette.get(ColorRole::OnSurface))
    }

    pub fn modal_border(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::BorderPrimary))
    }

    pub fn modal_title(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Primary))
            .add_modifier(Modifier::BOLD)
    }

    // Notification styles
    pub fn notification_success(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Success))
            .fg(self.palette.get(ColorRole::TextInverse))
    }

    pub fn notification_warning(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Warning))
            .fg(self.palette.get(ColorRole::TextInverse))
    }

    pub fn notification_error(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Error))
            .fg(self.palette.get(ColorRole::TextInverse))
    }

    pub fn notification_info(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Info))
            .fg(self.palette.get(ColorRole::TextInverse))
    }
}

/// Helper functions for common style combinations
impl ComponentStyles {
    /// Create a style for syntax highlighting (if needed for code display)
    pub fn syntax_keyword(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Primary))
            .add_modifier(Modifier::BOLD)
    }

    pub fn syntax_string(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Success))
    }

    pub fn syntax_number(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Accent))
    }

    pub fn syntax_comment(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::TextMuted))
            .add_modifier(Modifier::ITALIC)
    }

    /// Get style for track duration display
    pub fn track_duration(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::TextSecondary))
    }

    /// Get style for file size display
    pub fn file_size(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::TextMuted))
    }

    /// Get style for timestamp display
    pub fn timestamp(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::TextMuted))
    }

    /// Get style for percentage display
    pub fn percentage(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Accent))
            .add_modifier(Modifier::BOLD)
    }

    /// Get style for speed/rate display (downloads per second, etc.)
    pub fn rate(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Info))
    }

    // Sidebar specific styles
    pub fn sidebar_background(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Surface))
            .fg(self.palette.get(ColorRole::OnSurface))
    }

    pub fn sidebar_title(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Primary))
            .add_modifier(Modifier::BOLD)
    }

    pub fn sidebar_section_title(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Accent))
            .add_modifier(Modifier::BOLD)
    }

    pub fn sidebar_section_border(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::BorderSecondary))
    }

    pub fn sidebar_label(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::TextSecondary))
    }

    pub fn sidebar_icon(&self) -> Style {
        Style::default()
            .fg(self.palette.get(ColorRole::Accent))
    }

    pub fn progress_bar_success(&self) -> Style {
        Style::default()
            .bg(self.palette.get(ColorRole::Success))
            .fg(self.palette.get(ColorRole::TextInverse))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::terminal::colors::ColorPalette;

    #[test]
    fn test_component_styles_creation() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);

        // Test that styles are created without panics
        let _ = styles.header_background();
        let _ = styles.text_primary();
        let _ = styles.button_normal();
    }

    #[test]
    fn test_all_style_methods() {
        let palette = ColorPalette::light();
        let styles = ComponentStyles::new(palette);

        // Test that all style methods work
        let _ = styles.header_background();
        let _ = styles.footer_background();
        let _ = styles.panel_background();
        let _ = styles.input_normal();
        let _ = styles.list_item();
        let _ = styles.progress_bar();
        let _ = styles.state_pending();
        let _ = styles.button_normal();
        let _ = styles.chart_axis();
        let _ = styles.scrollbar_track();
        let _ = styles.badge_normal();
        let _ = styles.divider_horizontal();
        let _ = styles.modal_background();
        let _ = styles.notification_success();
    }

    #[test]
    fn test_helper_styles() {
        let palette = ColorPalette::high_contrast();
        let styles = ComponentStyles::new(palette);

        // Test helper style methods
        let _ = styles.syntax_keyword();
        let _ = styles.track_duration();
        let _ = styles.file_size();
        let _ = styles.timestamp();
        let _ = styles.percentage();
        let _ = styles.rate();
    }
}