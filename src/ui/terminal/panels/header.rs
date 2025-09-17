//! Header panel with contextual information
//!
//! Displays application title, current mode, connection status,
//! and other contextual information at the top of the screen.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph, Widget as RatatuiWidget},
    text::{Line, Span},
    Frame,
};
use crossterm::event::KeyEvent;
use std::time::{SystemTime, UNIX_EPOCH};

use super::super::{
    styles::ComponentStyles,
    state::AppState,
};

/// Header panel configuration
#[derive(Debug, Clone)]
pub struct HeaderConfig {
    pub show_logo: bool,
    pub show_status: bool,
    pub show_stats: bool,
    pub show_time: bool,
    pub compact_mode: bool,
}

impl Default for HeaderConfig {
    fn default() -> Self {
        Self {
            show_logo: true,
            show_status: true,
            show_stats: true,
            show_time: true,
            compact_mode: false,
        }
    }
}

/// Header panel component
#[derive(Debug)]
pub struct HeaderPanel {
    styles: ComponentStyles,
    config: HeaderConfig,
    last_update: SystemTime,
}

impl HeaderPanel {
    /// Create a new header panel
    pub fn new(styles: ComponentStyles) -> Self {
        Self {
            styles,
            config: HeaderConfig::default(),
            last_update: SystemTime::now(),
        }
    }

    /// Configure the header panel
    pub fn with_config(mut self, config: HeaderConfig) -> Self {
        self.config = config;
        self
    }

    /// Enable or disable compact mode
    pub fn set_compact_mode(&mut self, compact: bool) {
        self.config.compact_mode = compact;
    }

    /// Update header with current state
    pub fn update(&mut self, _state: &AppState) {
        self.last_update = SystemTime::now();
    }

    /// Handle input events
    pub fn handle_input(&mut self, _event: &KeyEvent) -> bool {
        // Header panel doesn't handle input by default
        false
    }

    /// Render the header panel
    pub fn render(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        if self.config.compact_mode {
            self.render_compact(frame, area, state);
        } else {
            self.render_full(frame, area, state);
        }
    }

    /// Render full header with all information
    fn render_full(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        // Create border block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.styles.panel_border())
            .style(self.styles.header_background());

        let inner = block.inner(area);

        // Split header into sections
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(30), // Logo and title
                Constraint::Min(0),     // Status and stats (flexible)
                Constraint::Length(20), // Time and info
            ])
            .split(inner);

        // Render border
        RatatuiWidget::render(block, area, frame.buffer_mut());

        // Left section: Logo and title
        if self.config.show_logo {
            self.render_logo_section(frame, chunks[0], state);
        }

        // Middle section: Status and stats
        if self.config.show_status || self.config.show_stats {
            self.render_status_section(frame, chunks[1], state);
        }

        // Right section: Time and info
        if self.config.show_time {
            self.render_time_section(frame, chunks[2], state);
        }
    }

    /// Render compact header for small screens
    fn render_compact(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.styles.panel_border())
            .style(self.styles.header_background());

        let inner = block.inner(area);

        // Single line with essential info
        let status_text = self.get_status_summary(state);
        let paragraph = Paragraph::new(Line::from(vec![
            Span::styled("🎵 lrcget-cli", self.styles.header_title()),
            Span::raw(" • "),
            Span::styled(status_text, self.styles.header_subtitle()),
        ]))
        .alignment(Alignment::Left);

        RatatuiWidget::render(block, area, frame.buffer_mut());
        RatatuiWidget::render(paragraph, inner, frame.buffer_mut());
    }

    /// Render logo and title section
    fn render_logo_section(&self, frame: &mut Frame, area: Rect, _state: &AppState) {
        let content = vec![
            Line::from(vec![
                Span::styled("🎵", self.styles.text_emphasis()),
                Span::raw(" "),
                Span::styled("lrcget-cli", self.styles.header_title()),
            ]),
            Line::from(Span::styled("Lyrics Downloader", self.styles.header_subtitle())),
        ];

        let paragraph = Paragraph::new(content).alignment(Alignment::Left);
        RatatuiWidget::render(paragraph, area, frame.buffer_mut());
    }

    /// Render status and stats section
    fn render_status_section(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let mut lines = Vec::new();

        // Connection status
        if self.config.show_status {
            let status_line = self.create_status_line(state);
            lines.push(status_line);
        }

        // Statistics
        if self.config.show_stats && lines.len() < 2 {
            let stats_line = self.create_stats_line(state);
            lines.push(stats_line);
        }

        let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
        RatatuiWidget::render(paragraph, area, frame.buffer_mut());
    }

    /// Render time and info section
    fn render_time_section(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let current_time = self.format_current_time();
        let mode_info = self.get_mode_info(state);

        let content = vec![
            Line::from(Span::styled(current_time, self.styles.text_secondary())),
            Line::from(Span::styled(mode_info, self.styles.text_muted())),
        ];

        let paragraph = Paragraph::new(content).alignment(Alignment::Right);
        RatatuiWidget::render(paragraph, area, frame.buffer_mut());
    }

    /// Create status line with connection and operation info
    fn create_status_line(&self, state: &AppState) -> Line {
        let (status_icon, status_text, status_style) = match self.get_connection_status(state) {
            ConnectionStatus::Connected => ("🟢", "Connected", self.styles.text_success()),
            ConnectionStatus::Connecting => ("🟡", "Connecting", self.styles.text_warning()),
            ConnectionStatus::Disconnected => ("🔴", "Disconnected", self.styles.text_error()),
            ConnectionStatus::Offline => ("⚫", "Offline", self.styles.text_muted()),
        };

        let operation = self.get_current_operation(state);

        Line::from(vec![
            Span::styled(status_icon, status_style),
            Span::raw(" "),
            Span::styled(status_text, status_style),
            Span::raw(" • "),
            Span::styled(operation, self.styles.text_secondary()),
        ])
    }

    /// Create statistics line
    fn create_stats_line(&self, state: &AppState) -> Line {
        let stats = &state.stats;

        Line::from(vec![
            Span::styled("📊 ", self.styles.text_muted()),
            Span::styled(format!("✅ {}", stats.completed), self.styles.text_success()),
            Span::raw(" • "),
            Span::styled(format!("❌ {}", stats.failed), self.styles.text_error()),
            Span::raw(" • "),
            Span::styled(format!("⏭️ {}", stats.skipped), self.styles.text_warning()),
        ])
    }

    /// Get connection status based on app state
    fn get_connection_status(&self, state: &AppState) -> ConnectionStatus {
        // This is a simplified implementation
        // In a real app, you'd check actual network connectivity
        let total = state.stats.total_processed + state.stats.completed + state.stats.failed + state.stats.skipped;
        if total > 0 && state.stats.total_processed > 0 {
            ConnectionStatus::Connected
        } else if total > 0 {
            ConnectionStatus::Connecting
        } else {
            ConnectionStatus::Offline
        }
    }

    /// Get current operation description
    fn get_current_operation(&self, state: &AppState) -> String {
        let processing_count = state.queue.items.iter()
            .filter(|track| matches!(track.status, crate::ui::terminal::state::TrackStatus::Downloading | crate::ui::terminal::state::TrackStatus::Processing))
            .count();
        let pending_count = state.queue.items.iter()
            .filter(|track| track.status == crate::ui::terminal::state::TrackStatus::Pending)
            .count();

        if processing_count > 0 {
            format!("Processing {} tracks", processing_count)
        } else if pending_count > 0 {
            format!("{} tracks pending", pending_count)
        } else if state.stats.total_processed > 0 {
            "Ready".to_string()
        } else {
            "Idle".to_string()
        }
    }

    /// Get mode information
    fn get_mode_info(&self, _state: &AppState) -> String {
        // This could be extended to show current mode (scan, download, etc.)
        "Download Mode".to_string()
    }

    /// Format current time
    fn format_current_time(&self) -> String {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                let secs = duration.as_secs();
                let hours = (secs / 3600) % 24;
                let minutes = (secs / 60) % 60;
                format!("{:02}:{:02}", hours, minutes)
            }
            Err(_) => "--:--".to_string(),
        }
    }

    /// Get status summary for compact mode
    fn get_status_summary(&self, state: &AppState) -> String {
        let stats = &state.stats;
        let total = stats.total_processed + stats.completed + stats.failed + stats.skipped;
        if total > 0 {
            format!("{}/{} tracks", stats.total_processed, total)
        } else {
            "Ready".to_string()
        }
    }
}

/// Connection status enum
#[derive(Debug, Clone, Copy, PartialEq)]
enum ConnectionStatus {
    Connected,
    Connecting,
    Disconnected,
    Offline,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::terminal::{colors::ColorPalette, state::Statistics};

    fn create_test_state() -> AppState {
        AppState {
            stats: Statistics {
                total: 100,
                processed_tracks: 50,
                completed: 40,
                failed: 5,
                skipped: 5,
                pending: 50,
                processing: 0,
                session_start: SystemTime::now(),
            },
            logs: Vec::new(),
            track_queue: Vec::new(),
        }
    }

    #[test]
    fn test_header_panel_creation() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let header = HeaderPanel::new(styles);

        assert!(header.config.show_logo);
        assert!(header.config.show_status);
        assert!(!header.config.compact_mode);
    }

    #[test]
    fn test_header_configuration() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let config = HeaderConfig {
            show_logo: false,
            show_status: true,
            show_stats: false,
            show_time: false,
            compact_mode: true,
        };

        let header = HeaderPanel::new(styles).with_config(config);
        assert!(!header.config.show_logo);
        assert!(header.config.compact_mode);
    }

    #[test]
    fn test_connection_status() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let header = HeaderPanel::new(styles);

        let state = create_test_state();
        let status = header.get_connection_status(&state);
        assert_eq!(status, ConnectionStatus::Connected);
    }

    #[test]
    fn test_current_operation() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let header = HeaderPanel::new(styles);

        let mut state = create_test_state();
        // Note: processing is now calculated from queue, not stored in stats

        let operation = header.get_current_operation(&state);
        assert_eq!(operation, "Processing 5 tracks");
    }

    #[test]
    fn test_compact_mode() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let mut header = HeaderPanel::new(styles);

        header.set_compact_mode(true);
        assert!(header.config.compact_mode);

        header.set_compact_mode(false);
        assert!(!header.config.compact_mode);
    }

    #[test]
    fn test_status_summary() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let header = HeaderPanel::new(styles);

        let state = create_test_state();
        let summary = header.get_status_summary(&state);
        assert_eq!(summary, "50/100 tracks");
    }
}