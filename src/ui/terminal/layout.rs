//! Responsive layout system for terminal UI
//!
//! Automatically adapts the interface based on terminal size and provides
//! different layout modes for optimal user experience across various screen sizes.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Different layout modes based on terminal size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// Full layout: >120 cols - 3 panels + header + footer + logs
    Full,
    /// Compact layout: 80-120 cols - 2 panels + logs below
    Compact,
    /// Minimal layout: 40-80 cols - 1 panel with tabs
    Minimal,
    /// Text mode: <40 cols - simple text output
    Text,
}

/// Manages layout adaptation and panel arrangement
pub struct LayoutManager {
    mode: LayoutMode,
    terminal_size: (u16, u16),
    panel_ratios: PanelRatios,
}

/// Panel size ratios for different layout modes
#[derive(Debug, Clone)]
pub struct PanelRatios {
    pub header_height: u16,
    pub footer_height: u16,
    pub logs_height: u16,
    pub left_panel_width: u16,  // Percentage of available width
    pub right_panel_width: u16, // Percentage of available width
}

/// Complete layout structure with all panel areas
#[derive(Debug, Clone)]
pub struct AppLayout {
    pub header: Rect,
    pub main_content: Rect,
    pub logs: Rect,
    pub footer: Rect,

    // Main content sub-areas (depending on layout mode)
    pub left_panel: Option<Rect>,    // Queue panel
    pub center_panel: Option<Rect>,  // Performance panel
    pub right_panel: Option<Rect>,   // Statistics panel
    pub single_panel: Option<Rect>,  // For minimal mode
}

impl LayoutManager {
    pub fn new() -> Self {
        let (width, height) = crossterm::terminal::size().unwrap_or((80, 24));
        let mode = Self::determine_layout_mode(width, height);
        let panel_ratios = PanelRatios::for_mode(mode);

        Self {
            mode,
            terminal_size: (width, height),
            panel_ratios,
        }
    }

    /// Update layout based on new terminal size
    pub fn update_size(&mut self, width: u16, height: u16) {
        self.terminal_size = (width, height);
        let new_mode = Self::determine_layout_mode(width, height);

        if new_mode != self.mode {
            self.mode = new_mode;
            self.panel_ratios = PanelRatios::for_mode(new_mode);
        }
    }

    /// Get current layout mode
    pub fn mode(&self) -> LayoutMode {
        self.mode
    }

    /// Calculate complete layout for current terminal size
    pub fn calculate_layout(&self, area: Rect) -> AppLayout {
        match self.mode {
            LayoutMode::Full => self.calculate_full_layout(area),
            LayoutMode::Compact => self.calculate_compact_layout(area),
            LayoutMode::Minimal => self.calculate_minimal_layout(area),
            LayoutMode::Text => self.calculate_text_layout(area),
        }
    }

    /// Determine layout mode based on terminal dimensions
    fn determine_layout_mode(width: u16, height: u16) -> LayoutMode {
        match (width, height) {
            (w, h) if w >= 120 && h >= 30 => LayoutMode::Full,
            (w, h) if w >= 80 && h >= 24 => LayoutMode::Compact,
            (w, h) if w >= 40 && h >= 16 => LayoutMode::Minimal,
            _ => LayoutMode::Text,
        }
    }

    /// Calculate layout for full mode (3 panels)
    fn calculate_full_layout(&self, area: Rect) -> AppLayout {
        // Main vertical split: header / content / logs / footer
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(self.panel_ratios.header_height),     // Header
                Constraint::Min(10),                                     // Main content
                Constraint::Length(self.panel_ratios.logs_height),       // Logs
                Constraint::Length(self.panel_ratios.footer_height),     // Footer
            ])
            .split(area);

        let header = main_chunks[0];
        let main_content = main_chunks[1];
        let logs = main_chunks[2];
        let footer = main_chunks[3];

        // Split main content into 3 panels
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(self.panel_ratios.left_panel_width),   // Queue
                Constraint::Percentage(33),                                   // Performance
                Constraint::Percentage(self.panel_ratios.right_panel_width),  // Statistics
            ])
            .split(main_content);

        AppLayout {
            header,
            main_content,
            logs,
            footer,
            left_panel: Some(content_chunks[0]),
            center_panel: Some(content_chunks[1]),
            right_panel: Some(content_chunks[2]),
            single_panel: None,
        }
    }

    /// Calculate layout for compact mode (2 panels)
    fn calculate_compact_layout(&self, area: Rect) -> AppLayout {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(self.panel_ratios.header_height),
                Constraint::Min(10),
                Constraint::Length(self.panel_ratios.logs_height),
                Constraint::Length(self.panel_ratios.footer_height),
            ])
            .split(area);

        let header = main_chunks[0];
        let main_content = main_chunks[1];
        let logs = main_chunks[2];
        let footer = main_chunks[3];

        // Split main content into 2 panels
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(60),  // Queue + Performance combined
                Constraint::Percentage(40),  // Statistics
            ])
            .split(main_content);

        AppLayout {
            header,
            main_content,
            logs,
            footer,
            left_panel: Some(content_chunks[0]),
            center_panel: None,
            right_panel: Some(content_chunks[1]),
            single_panel: None,
        }
    }

    /// Calculate layout for minimal mode (1 panel with tabs)
    fn calculate_minimal_layout(&self, area: Rect) -> AppLayout {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(self.panel_ratios.header_height),
                Constraint::Min(10),
                Constraint::Length(self.panel_ratios.logs_height),
                Constraint::Length(self.panel_ratios.footer_height),
            ])
            .split(area);

        let header = main_chunks[0];
        let main_content = main_chunks[1];
        let logs = main_chunks[2];
        let footer = main_chunks[3];

        AppLayout {
            header,
            main_content,
            logs,
            footer,
            left_panel: None,
            center_panel: None,
            right_panel: None,
            single_panel: Some(main_content),
        }
    }

    /// Calculate layout for text mode (minimal UI)
    fn calculate_text_layout(&self, area: Rect) -> AppLayout {
        // Very simple layout for tiny terminals
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),  // Minimal header
                Constraint::Min(1),     // All content
                Constraint::Length(1),  // Minimal footer
            ])
            .split(area);

        AppLayout {
            header: main_chunks[0],
            main_content: main_chunks[1],
            logs: main_chunks[1], // Share space with main content
            footer: main_chunks[2],
            left_panel: None,
            center_panel: None,
            right_panel: None,
            single_panel: Some(main_chunks[1]),
        }
    }

    /// Get recommended minimum terminal size for optimal experience
    pub fn recommended_size() -> (u16, u16) {
        (120, 30)
    }

    /// Check if current terminal size is adequate
    pub fn is_size_adequate(&self) -> bool {
        let (width, height) = self.terminal_size;
        width >= 80 && height >= 24
    }
}

impl PanelRatios {
    fn for_mode(mode: LayoutMode) -> Self {
        match mode {
            LayoutMode::Full => Self {
                header_height: 3,
                footer_height: 2,
                logs_height: 8,
                left_panel_width: 34,
                right_panel_width: 33,
            },
            LayoutMode::Compact => Self {
                header_height: 3,
                footer_height: 2,
                logs_height: 6,
                left_panel_width: 60,
                right_panel_width: 40,
            },
            LayoutMode::Minimal => Self {
                header_height: 2,
                footer_height: 2,
                logs_height: 4,
                left_panel_width: 100,
                right_panel_width: 0,
            },
            LayoutMode::Text => Self {
                header_height: 2,
                footer_height: 1,
                logs_height: 0,
                left_panel_width: 100,
                right_panel_width: 0,
            },
        }
    }
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_mode_determination() {
        assert_eq!(LayoutManager::determine_layout_mode(150, 40), LayoutMode::Full);
        assert_eq!(LayoutManager::determine_layout_mode(100, 30), LayoutMode::Compact);
        assert_eq!(LayoutManager::determine_layout_mode(60, 20), LayoutMode::Minimal);
        assert_eq!(LayoutManager::determine_layout_mode(30, 10), LayoutMode::Text);
    }

    #[test]
    fn test_layout_calculation() {
        let manager = LayoutManager::new();
        let area = Rect::new(0, 0, 120, 30);
        let layout = manager.calculate_layout(area);

        // Basic sanity checks
        assert!(layout.header.height > 0);
        assert!(layout.footer.height > 0);
        assert!(layout.main_content.height > 0);
    }

    #[test]
    fn test_size_adequacy() {
        let mut manager = LayoutManager::new();

        manager.update_size(100, 30);
        assert!(manager.is_size_adequate());

        manager.update_size(60, 15);
        assert!(!manager.is_size_adequate());
    }
}