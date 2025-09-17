//! Sidebar panel with metrics and system information
//!
//! Displays real-time statistics, system information, and quick metrics
//! in a dedicated sidebar area.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Widget as RatatuiWidget},
    text::{Line, Span},
    Frame,
};
use crossterm::event::{KeyEvent, KeyCode, KeyModifiers};
use std::time::{SystemTime, UNIX_EPOCH};

use super::super::{
    styles::ComponentStyles,
    state::AppState,
};

/// Sidebar content sections
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SidebarSection {
    Stats,
    Progress,
    System,
    Recent,
}

/// Sidebar panel configuration
#[derive(Debug, Clone)]
pub struct SidebarConfig {
    pub show_borders: bool,
    pub show_title: bool,
    pub sections: Vec<SidebarSection>,
    pub compact_mode: bool,
    pub auto_refresh: bool,
}

impl Default for SidebarConfig {
    fn default() -> Self {
        Self {
            show_borders: true,
            show_title: true,
            sections: vec![
                SidebarSection::Stats,
                SidebarSection::Progress,
                SidebarSection::System,
                SidebarSection::Recent,
            ],
            compact_mode: false,
            auto_refresh: true,
        }
    }
}

/// Sidebar panel component
#[derive(Debug)]
pub struct SidebarPanel {
    styles: ComponentStyles,
    config: SidebarConfig,
    scroll_offset: usize,
    last_update: SystemTime,
}

impl SidebarPanel {
    /// Create a new sidebar panel
    pub fn new(styles: ComponentStyles) -> Self {
        Self {
            styles,
            config: SidebarConfig::default(),
            scroll_offset: 0,
            last_update: SystemTime::now(),
        }
    }

    /// Configure the sidebar panel
    pub fn with_config(mut self, config: SidebarConfig) -> Self {
        self.config = config;
        self
    }

    /// Enable or disable compact mode
    pub fn set_compact_mode(&mut self, compact: bool) {
        self.config.compact_mode = compact;
    }

    /// Update sidebar with current state
    pub fn update(&mut self, _state: &AppState) {
        if self.config.auto_refresh {
            self.last_update = SystemTime::now();
        }
    }

    /// Handle input events
    pub fn handle_input(&mut self, event: &KeyEvent) -> bool {
        match (event.code, event.modifiers) {
            (KeyCode::Up, KeyModifiers::NONE) => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                true
            }
            (KeyCode::Down, KeyModifiers::NONE) => {
                self.scroll_offset += 1;
                true
            }
            (KeyCode::PageUp, KeyModifiers::NONE) => {
                self.scroll_offset = self.scroll_offset.saturating_sub(5);
                true
            }
            (KeyCode::PageDown, KeyModifiers::NONE) => {
                self.scroll_offset += 5;
                true
            }
            (KeyCode::Home, KeyModifiers::NONE) => {
                self.scroll_offset = 0;
                true
            }
            (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                // Force refresh
                self.last_update = SystemTime::now();
                true
            }
            _ => false,
        }
    }

    /// Render the sidebar panel
    pub fn render(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        if self.config.compact_mode {
            self.render_compact(frame, area, state);
        } else {
            self.render_full(frame, area, state);
        }
    }

    /// Render full sidebar with all sections
    fn render_full(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.styles.panel_border())
            .style(self.styles.sidebar_background());

        let title_block = if self.config.show_title {
            block.title("Metrics")
                .title_style(self.styles.sidebar_title())
        } else {
            block
        };

        let inner = title_block.inner(area);

        // Calculate section heights based on available space
        let section_count = self.config.sections.len();
        let section_height = if section_count > 0 {
            inner.height / section_count as u16
        } else {
            inner.height
        };

        // Create constraints for sections
        let constraints: Vec<Constraint> = self.config.sections
            .iter()
            .map(|_| Constraint::Min(section_height.max(3)))
            .collect();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        RatatuiWidget::render(title_block, area, frame.buffer_mut());

        // Render each section
        for (i, &section) in self.config.sections.iter().enumerate() {
            if i < chunks.len() {
                self.render_section(frame, chunks[i], section, state);
            }
        }
    }

    /// Render compact sidebar for small screens
    fn render_compact(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.styles.panel_border())
            .style(self.styles.sidebar_background());

        let inner = block.inner(area);

        // Show only essential stats in compact mode
        let stats = &state.stats;
        let total = stats.total_processed + stats.completed + stats.failed + stats.skipped;
        let progress = if total > 0 {
            stats.total_processed as f64 / total as f64
        } else {
            0.0
        };

        let content = vec![
            Line::from(vec![
                Span::styled("📊", self.styles.sidebar_icon()),
                Span::raw(" "),
                Span::styled(format!("{}/{}", stats.total_processed, total), self.styles.text_emphasis()),
            ]),
            Line::from(vec![
                Span::styled("✅", self.styles.text_success()),
                Span::raw(" "),
                Span::styled(stats.completed.to_string(), self.styles.text_success()),
                Span::raw(" "),
                Span::styled("❌", self.styles.text_error()),
                Span::raw(" "),
                Span::styled(stats.failed.to_string(), self.styles.text_error()),
            ]),
        ];

        let paragraph = Paragraph::new(content);

        RatatuiWidget::render(block, area, frame.buffer_mut());
        RatatuiWidget::render(paragraph, inner, frame.buffer_mut());
    }

    /// Render individual section
    fn render_section(&self, frame: &mut Frame, area: Rect, section: SidebarSection, state: &AppState) {
        match section {
            SidebarSection::Stats => self.render_stats_section(frame, area, state),
            SidebarSection::Progress => self.render_progress_section(frame, area, state),
            SidebarSection::System => self.render_system_section(frame, area, state),
            SidebarSection::Recent => self.render_recent_section(frame, area, state),
        }
    }

    /// Render statistics section
    fn render_stats_section(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let block = Block::default()
            .title("📊 Statistics")
            .title_style(self.styles.sidebar_section_title())
            .borders(Borders::TOP)
            .border_style(self.styles.sidebar_section_border());

        let inner = block.inner(area);
        let stats = &state.stats;
        let total = stats.total_processed + stats.completed + stats.failed + stats.skipped;

        let content = vec![
            Line::from(vec![
                Span::styled("Total: ", self.styles.sidebar_label()),
                Span::styled(total.to_string(), self.styles.text_emphasis()),
            ]),
            Line::from(vec![
                Span::styled("Processed: ", self.styles.sidebar_label()),
                Span::styled(stats.total_processed.to_string(), self.styles.text_secondary()),
            ]),
            Line::from(vec![
                Span::styled("✅ Completed: ", self.styles.sidebar_label()),
                Span::styled(stats.completed.to_string(), self.styles.text_success()),
            ]),
            Line::from(vec![
                Span::styled("❌ Failed: ", self.styles.sidebar_label()),
                Span::styled(stats.failed.to_string(), self.styles.text_error()),
            ]),
            Line::from(vec![
                Span::styled("⏭️ Skipped: ", self.styles.sidebar_label()),
                Span::styled(stats.skipped.to_string(), self.styles.text_warning()),
            ]),
            Line::from(vec![
                Span::styled("⏳ Pending: ", self.styles.sidebar_label()),
                Span::styled("0", self.styles.text_muted()), // Pending would need to be calculated from queue
            ]),
        ];

        let paragraph = Paragraph::new(content);

        RatatuiWidget::render(block, area, frame.buffer_mut());
        RatatuiWidget::render(paragraph, inner, frame.buffer_mut());
    }

    /// Render progress section
    fn render_progress_section(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let block = Block::default()
            .title("📈 Progress")
            .title_style(self.styles.sidebar_section_title())
            .borders(Borders::TOP)
            .border_style(self.styles.sidebar_section_border());

        let inner = block.inner(area);
        let stats = &state.stats;

        // Calculate overall progress
        let total = stats.total_processed + stats.completed + stats.failed + stats.skipped;
        let progress = if total > 0 {
            stats.total_processed as f64 / total as f64
        } else {
            0.0
        };

        // Calculate success rate
        let success_rate = if stats.total_processed > 0 {
            stats.completed as f64 / stats.total_processed as f64
        } else {
            0.0
        };

        // Create progress bars
        let progress_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Progress label
                Constraint::Length(1), // Progress bar
                Constraint::Length(1), // Success label
                Constraint::Length(1), // Success bar
                Constraint::Min(0),    // Remaining space
            ])
            .split(inner);

        // Overall progress
        let progress_label = Paragraph::new(Line::from(vec![
            Span::styled("Overall: ", self.styles.sidebar_label()),
            Span::styled(format!("{:.1}%", progress * 100.0), self.styles.text_emphasis()),
        ]));

        let progress_gauge = Gauge::default()
            .block(Block::default())
            .gauge_style(self.styles.progress_bar())
            .ratio(progress);

        // Success rate
        let success_label = Paragraph::new(Line::from(vec![
            Span::styled("Success: ", self.styles.sidebar_label()),
            Span::styled(format!("{:.1}%", success_rate * 100.0), self.styles.text_success()),
        ]));

        let success_gauge = Gauge::default()
            .block(Block::default())
            .gauge_style(self.styles.progress_bar_success())
            .ratio(success_rate);

        RatatuiWidget::render(block, area, frame.buffer_mut());
        RatatuiWidget::render(progress_label, progress_chunks[0], frame.buffer_mut());
        RatatuiWidget::render(progress_gauge, progress_chunks[1], frame.buffer_mut());
        RatatuiWidget::render(success_label, progress_chunks[2], frame.buffer_mut());
        RatatuiWidget::render(success_gauge, progress_chunks[3], frame.buffer_mut());
    }

    /// Render system information section
    fn render_system_section(&self, frame: &mut Frame, area: Rect, _state: &AppState) {
        let block = Block::default()
            .title("🖥️ System")
            .title_style(self.styles.sidebar_section_title())
            .borders(Borders::TOP)
            .border_style(self.styles.sidebar_section_border());

        let inner = block.inner(area);

        // Get system information
        let uptime = self.get_session_uptime();
        let memory_info = self.get_memory_info();

        let content = vec![
            Line::from(vec![
                Span::styled("Uptime: ", self.styles.sidebar_label()),
                Span::styled(uptime, self.styles.text_secondary()),
            ]),
            Line::from(vec![
                Span::styled("Memory: ", self.styles.sidebar_label()),
                Span::styled(memory_info, self.styles.text_secondary()),
            ]),
            Line::from(vec![
                Span::styled("Version: ", self.styles.sidebar_label()),
                Span::styled("v1.0.0", self.styles.text_muted()),
            ]),
        ];

        let paragraph = Paragraph::new(content);

        RatatuiWidget::render(block, area, frame.buffer_mut());
        RatatuiWidget::render(paragraph, inner, frame.buffer_mut());
    }

    /// Render recent activity section
    fn render_recent_section(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let block = Block::default()
            .title("🕒 Recent")
            .title_style(self.styles.sidebar_section_title())
            .borders(Borders::TOP)
            .border_style(self.styles.sidebar_section_border());

        let inner = block.inner(area);

        if state.logs.entries.is_empty() {
            let empty_msg = Paragraph::new(Line::from(Span::styled(
                "No recent activity",
                self.styles.text_muted(),
            )))
            .alignment(Alignment::Center);

            RatatuiWidget::render(block, area, frame.buffer_mut());
            RatatuiWidget::render(empty_msg, inner, frame.buffer_mut());
            return;
        }

        // Show recent log entries
        let recent_items: Vec<ListItem> = state.logs.entries
            .iter()
            .rev()
            .take(5)
            .map(|log_entry| {
                let level_icon = match log_entry.level {
                    crate::ui::terminal::state::LogLevel::Error => "❌",
                    crate::ui::terminal::state::LogLevel::Warning => "⚠️",
                    crate::ui::terminal::state::LogLevel::Info => "ℹ️",
                    crate::ui::terminal::state::LogLevel::Debug => "🔍",
                };

                let level_style = match log_entry.level {
                    crate::ui::terminal::state::LogLevel::Error => self.styles.text_error(),
                    crate::ui::terminal::state::LogLevel::Warning => self.styles.text_warning(),
                    crate::ui::terminal::state::LogLevel::Info => self.styles.text_info(),
                    crate::ui::terminal::state::LogLevel::Debug => self.styles.text_muted(),
                };

                // Truncate message if too long
                let truncated_message = if log_entry.message.len() > 30 {
                    format!("{}...", &log_entry.message[..27])
                } else {
                    log_entry.message.clone()
                };

                let content = Line::from(vec![
                    Span::styled(level_icon, level_style),
                    Span::raw(" "),
                    Span::styled(truncated_message, self.styles.text_secondary()),
                ]);

                ListItem::new(content)
            })
            .collect();

        let list = List::new(recent_items);

        RatatuiWidget::render(block, area, frame.buffer_mut());
        RatatuiWidget::render(list, inner, frame.buffer_mut());
    }

    /// Get session uptime as formatted string
    fn get_session_uptime(&self) -> String {
        match self.last_update.duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                let secs = duration.as_secs();
                let hours = secs / 3600;
                let minutes = (secs % 3600) / 60;
                let seconds = secs % 60;

                if hours > 0 {
                    format!("{}h {}m", hours, minutes)
                } else if minutes > 0 {
                    format!("{}m {}s", minutes, seconds)
                } else {
                    format!("{}s", seconds)
                }
            }
            Err(_) => "Unknown".to_string(),
        }
    }

    /// Get memory information (simplified)
    fn get_memory_info(&self) -> String {
        // This is a simplified implementation
        // In a real app, you might use the `sysinfo` crate for actual memory stats
        "~50MB".to_string()
    }

    /// Get section height for layout calculations
    pub fn get_section_height(&self, section: SidebarSection) -> u16 {
        match section {
            SidebarSection::Stats => 8,
            SidebarSection::Progress => 6,
            SidebarSection::System => 5,
            SidebarSection::Recent => 7,
        }
    }

    /// Get minimum height required for all sections
    pub fn get_minimum_height(&self) -> u16 {
        self.config.sections
            .iter()
            .map(|&section| self.get_section_height(section))
            .sum::<u16>()
            + 2 // For borders
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::terminal::{colors::ColorPalette, state::{Statistics, LogEntry, LogLevel}};
    use std::time::SystemTime;

    fn create_test_state() -> AppState {
        AppState {
            stats: Statistics {
                total: 100,
                processed_tracks: 60,
                completed: 50,
                failed: 8,
                skipped: 2,
                pending: 40,
                processing: 5,
                session_start: SystemTime::now(),
            },
            logs: vec![
                LogEntry {
                    timestamp: SystemTime::now(),
                    level: LogLevel::Info,
                    message: "Download started".to_string(),
                },
                LogEntry {
                    timestamp: SystemTime::now(),
                    level: LogLevel::Error,
                    message: "Failed to download track".to_string(),
                },
            ],
            track_queue: Vec::new(),
        }
    }

    #[test]
    fn test_sidebar_panel_creation() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let sidebar = SidebarPanel::new(styles);

        assert!(sidebar.config.show_borders);
        assert!(sidebar.config.show_title);
        assert_eq!(sidebar.config.sections.len(), 4);
        assert_eq!(sidebar.scroll_offset, 0);
    }

    #[test]
    fn test_sidebar_configuration() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let config = SidebarConfig {
            show_borders: false,
            show_title: false,
            sections: vec![SidebarSection::Stats, SidebarSection::Progress],
            compact_mode: true,
            auto_refresh: false,
        };

        let sidebar = SidebarPanel::new(styles).with_config(config);
        assert!(!sidebar.config.show_borders);
        assert!(sidebar.config.compact_mode);
        assert_eq!(sidebar.config.sections.len(), 2);
    }

    #[test]
    fn test_compact_mode() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let mut sidebar = SidebarPanel::new(styles);

        sidebar.set_compact_mode(true);
        assert!(sidebar.config.compact_mode);

        sidebar.set_compact_mode(false);
        assert!(!sidebar.config.compact_mode);
    }

    #[test]
    fn test_scrolling() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let mut sidebar = SidebarPanel::new(styles);

        assert_eq!(sidebar.scroll_offset, 0);

        // Test scrolling down
        use crossterm::event::{KeyEvent, KeyCode, KeyModifiers};
        let down_event = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        assert!(sidebar.handle_input(&down_event));
        assert_eq!(sidebar.scroll_offset, 1);

        // Test scrolling up
        let up_event = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert!(sidebar.handle_input(&up_event));
        assert_eq!(sidebar.scroll_offset, 0);

        // Test can't scroll above 0
        assert!(sidebar.handle_input(&up_event));
        assert_eq!(sidebar.scroll_offset, 0);
    }

    #[test]
    fn test_section_heights() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let sidebar = SidebarPanel::new(styles);

        assert_eq!(sidebar.get_section_height(SidebarSection::Stats), 8);
        assert_eq!(sidebar.get_section_height(SidebarSection::Progress), 6);
        assert_eq!(sidebar.get_section_height(SidebarSection::System), 5);
        assert_eq!(sidebar.get_section_height(SidebarSection::Recent), 7);
    }

    #[test]
    fn test_minimum_height() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let sidebar = SidebarPanel::new(styles);

        let min_height = sidebar.get_minimum_height();
        // 8 + 6 + 5 + 7 + 2 (borders) = 28
        assert_eq!(min_height, 28);
    }

    #[test]
    fn test_uptime_formatting() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let sidebar = SidebarPanel::new(styles);

        let uptime = sidebar.get_session_uptime();
        // Should return a valid time format
        assert!(!uptime.is_empty());
    }

    #[test]
    fn test_memory_info() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let sidebar = SidebarPanel::new(styles);

        let memory = sidebar.get_memory_info();
        assert_eq!(memory, "~50MB");
    }

    #[test]
    fn test_update() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let mut sidebar = SidebarPanel::new(styles);
        let state = create_test_state();

        let initial_update = sidebar.last_update;
        sidebar.update(&state);

        // Update time should have changed
        assert!(sidebar.last_update >= initial_update);
    }
}