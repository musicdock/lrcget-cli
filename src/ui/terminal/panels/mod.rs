//! Panel components for terminal UI layout
//!
//! This module provides the main structural components of the TUI:
//! header, footer, main content area, and sidebar panels.

pub mod header;
pub mod footer;
pub mod main;
pub mod sidebar;
pub mod queue;

// Re-export main panel types
pub use header::HeaderPanel;
pub use footer::FooterPanel;
pub use main::MainPanel;
pub use sidebar::SidebarPanel;
pub use queue::QueuePanel;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};
use super::{
    styles::ComponentStyles,
    state::AppState,
};

/// Main layout manager for organizing panels
#[derive(Debug)]
pub struct PanelLayout {
    show_sidebar: bool,
    header_height: u16,
    footer_height: u16,
    sidebar_width: u16,
}

impl PanelLayout {
    /// Create a new panel layout with default dimensions
    pub fn new() -> Self {
        Self {
            show_sidebar: true,
            header_height: 3,
            footer_height: 3,
            sidebar_width: 30,
        }
    }

    /// Configure whether to show the sidebar
    pub fn with_sidebar(mut self, show: bool) -> Self {
        self.show_sidebar = show;
        self
    }

    /// Set header height
    pub fn with_header_height(mut self, height: u16) -> Self {
        self.header_height = height;
        self
    }

    /// Set footer height
    pub fn with_footer_height(mut self, height: u16) -> Self {
        self.footer_height = height;
        self
    }

    /// Set sidebar width
    pub fn with_sidebar_width(mut self, width: u16) -> Self {
        self.sidebar_width = width;
        self
    }

    /// Calculate layout areas for the given terminal size
    pub fn calculate_areas(&self, area: Rect) -> LayoutAreas {
        // First, split vertically: header | main | footer
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(self.header_height),
                Constraint::Min(0), // Main area takes remaining space
                Constraint::Length(self.footer_height),
            ])
            .split(area);

        let header = vertical_chunks[0];
        let main_area = vertical_chunks[1];
        let footer = vertical_chunks[2];

        // Then split the main area horizontally if sidebar is enabled
        let (main, sidebar) = if self.show_sidebar {
            let horizontal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(0), // Main content takes remaining space
                    Constraint::Length(self.sidebar_width),
                ])
                .split(main_area);

            (horizontal_chunks[0], Some(horizontal_chunks[1]))
        } else {
            (main_area, None)
        };

        LayoutAreas {
            header,
            main,
            sidebar,
            footer,
        }
    }

    /// Auto-adjust layout based on terminal size
    pub fn auto_adjust(&mut self, area: Rect) {
        // Hide sidebar on very small screens
        if area.width < 80 {
            self.show_sidebar = false;
        }

        // Reduce header/footer height on very small screens
        if area.height < 20 {
            self.header_height = 2;
            self.footer_height = 2;
        }

        // Adjust sidebar width based on available space
        if area.width < 120 {
            self.sidebar_width = 25;
        } else if area.width > 150 {
            self.sidebar_width = 35;
        }
    }
}

impl Default for PanelLayout {
    fn default() -> Self {
        Self::new()
    }
}

/// Areas calculated by the panel layout
#[derive(Debug, Clone, Copy)]
pub struct LayoutAreas {
    pub header: Rect,
    pub main: Rect,
    pub sidebar: Option<Rect>,
    pub footer: Rect,
}

impl LayoutAreas {
    /// Get the main content area (excludes header and footer)
    pub fn content_area(&self) -> Rect {
        self.main
    }

    /// Get the sidebar area if available
    pub fn sidebar_area(&self) -> Option<Rect> {
        self.sidebar
    }

    /// Check if sidebar is visible
    pub fn has_sidebar(&self) -> bool {
        self.sidebar.is_some()
    }
}

/// Complete panel system for rendering all UI components
#[derive(Debug)]
pub struct PanelSystem {
    layout: PanelLayout,
    header: HeaderPanel,
    footer: FooterPanel,
    main: MainPanel,
    sidebar: SidebarPanel,
}

impl PanelSystem {
    /// Create a new panel system with the given styles
    pub fn new(styles: ComponentStyles) -> Self {
        Self {
            layout: PanelLayout::new(),
            header: HeaderPanel::new(styles.clone()),
            footer: FooterPanel::new(styles.clone()),
            main: MainPanel::new(styles.clone()),
            sidebar: SidebarPanel::new(styles),
        }
    }

    /// Configure the layout
    pub fn with_layout(mut self, layout: PanelLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Get a mutable reference to the header panel
    pub fn header_mut(&mut self) -> &mut HeaderPanel {
        &mut self.header
    }

    /// Get a mutable reference to the footer panel
    pub fn footer_mut(&mut self) -> &mut FooterPanel {
        &mut self.footer
    }

    /// Get a mutable reference to the main panel
    pub fn main_mut(&mut self) -> &mut MainPanel {
        &mut self.main
    }

    /// Get a mutable reference to the sidebar panel
    pub fn sidebar_mut(&mut self) -> &mut SidebarPanel {
        &mut self.sidebar
    }

    /// Render all panels to the frame
    pub fn render(&mut self, frame: &mut Frame, state: &AppState) {
        // Auto-adjust layout for current terminal size
        self.layout.auto_adjust(frame.size());

        // Calculate layout areas
        let areas = self.layout.calculate_areas(frame.size());

        // Render each panel
        self.header.render(frame, areas.header, state);
        self.main.render(frame, areas.main, state);
        self.footer.render(frame, areas.footer, state);

        if let Some(sidebar_area) = areas.sidebar {
            self.sidebar.render(frame, sidebar_area, state);
        }
    }

    /// Handle input events for panels
    pub fn handle_input(&mut self, event: &crossterm::event::KeyEvent) -> bool {
        // Try each panel in order of priority
        self.main.handle_input(event) ||
        self.footer.handle_input(event) ||
        self.sidebar.handle_input(event) ||
        self.header.handle_input(event)
    }

    /// Update panel states
    pub fn update(&mut self, state: &AppState) {
        self.header.update(state);
        self.footer.update(state);
        self.main.update(state);
        self.sidebar.update(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_layout_creation() {
        let layout = PanelLayout::new();
        assert_eq!(layout.header_height, 3);
        assert_eq!(layout.footer_height, 3);
        assert_eq!(layout.sidebar_width, 30);
        assert!(layout.show_sidebar);
    }

    #[test]
    fn test_panel_layout_configuration() {
        let layout = PanelLayout::new()
            .with_sidebar(false)
            .with_header_height(4)
            .with_footer_height(2)
            .with_sidebar_width(25);

        assert!(!layout.show_sidebar);
        assert_eq!(layout.header_height, 4);
        assert_eq!(layout.footer_height, 2);
        assert_eq!(layout.sidebar_width, 25);
    }

    #[test]
    fn test_layout_areas_calculation() {
        let layout = PanelLayout::new();
        let area = Rect::new(0, 0, 100, 30);
        let areas = layout.calculate_areas(area);

        assert_eq!(areas.header.height, 3);
        assert_eq!(areas.footer.height, 3);
        assert_eq!(areas.main.height, 24); // 30 - 3 - 3
        assert!(areas.sidebar.is_some());
        assert_eq!(areas.sidebar.unwrap().width, 30);
    }

    #[test]
    fn test_layout_without_sidebar() {
        let layout = PanelLayout::new().with_sidebar(false);
        let area = Rect::new(0, 0, 100, 30);
        let areas = layout.calculate_areas(area);

        assert!(areas.sidebar.is_none());
        assert_eq!(areas.main.width, 100); // Full width
    }

    #[test]
    fn test_auto_adjust_small_screen() {
        let mut layout = PanelLayout::new();
        let small_area = Rect::new(0, 0, 70, 15);

        layout.auto_adjust(small_area);

        assert!(!layout.show_sidebar);
        assert_eq!(layout.header_height, 2);
        assert_eq!(layout.footer_height, 2);
    }

    #[test]
    fn test_layout_areas_methods() {
        let layout = PanelLayout::new();
        let area = Rect::new(0, 0, 100, 30);
        let areas = layout.calculate_areas(area);

        assert_eq!(areas.content_area(), areas.main);
        assert!(areas.has_sidebar());
        assert!(areas.sidebar_area().is_some());
    }
}