//! Main content panel with scrollable content
//!
//! Displays the primary application content such as track lists,
//! search results, download progress, and other interactive content.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Widget as RatatuiWidget},
    text::{Line, Span},
    Frame,
};
use crossterm::event::{KeyEvent, KeyCode, KeyModifiers};

use super::super::{
    styles::ComponentStyles,
    state::{AppState, TrackQueueItem, TrackStatus},
    widgets::{Table, TableRow, Widget},
};

/// Content type for the main panel
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContentType {
    TrackList,
    SearchResults,
    DownloadProgress,
    Settings,
    Help,
    FileExplorer,
    Logs,
}

/// Main panel configuration
#[derive(Debug, Clone)]
pub struct MainPanelConfig {
    pub show_borders: bool,
    pub show_title: bool,
    pub scrollable: bool,
    pub selectable: bool,
}

impl Default for MainPanelConfig {
    fn default() -> Self {
        Self {
            show_borders: true,
            show_title: true,
            scrollable: true,
            selectable: true,
        }
    }
}

/// Main content panel component
#[derive(Debug)]
pub struct MainPanel {
    styles: ComponentStyles,
    config: MainPanelConfig,
    content_type: ContentType,
    selected_index: Option<usize>,
    scroll_offset: usize,
    list_state: ListState,
}

impl MainPanel {
    /// Create a new main panel
    pub fn new(styles: ComponentStyles) -> Self {
        Self {
            styles,
            config: MainPanelConfig::default(),
            content_type: ContentType::TrackList,
            selected_index: None,
            scroll_offset: 0,
            list_state: ListState::default(),
        }
    }

    /// Configure the main panel
    pub fn with_config(mut self, config: MainPanelConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the content type
    pub fn set_content_type(&mut self, content_type: ContentType) {
        self.content_type = content_type;
        self.selected_index = None;
        self.scroll_offset = 0;
    }

    /// Get current content type
    pub fn content_type(&self) -> ContentType {
        self.content_type
    }

    /// Select an item by index
    pub fn select(&mut self, index: Option<usize>) {
        self.selected_index = index;
        self.list_state.select(index);
    }

    /// Get currently selected index
    pub fn selected(&self) -> Option<usize> {
        self.selected_index
    }

    /// Move selection to next item
    pub fn select_next(&mut self, items_count: usize) -> bool {
        if items_count == 0 {
            return false;
        }

        match self.selected_index {
            Some(i) => {
                if i + 1 < items_count {
                    self.select(Some(i + 1));
                    true
                } else {
                    false
                }
            }
            None => {
                self.select(Some(0));
                true
            }
        }
    }

    /// Move selection to previous item
    pub fn select_previous(&mut self) -> bool {
        match self.selected_index {
            Some(i) => {
                if i > 0 {
                    self.select(Some(i - 1));
                    true
                } else {
                    false
                }
            }
            None => false,
        }
    }

    /// Scroll up by one page
    pub fn scroll_up(&mut self, page_size: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
    }

    /// Scroll down by one page
    pub fn scroll_down(&mut self, page_size: usize, total_items: usize) {
        let max_scroll = total_items.saturating_sub(page_size);
        self.scroll_offset = (self.scroll_offset + page_size).min(max_scroll);
    }

    /// Update panel with current state
    pub fn update(&mut self, state: &AppState) {
        // Auto-select content type based on state
        if !state.queue.items.is_empty() {
            self.content_type = ContentType::TrackList;
        } else if !state.logs.entries.is_empty() {
            self.content_type = ContentType::Logs;
        }
    }

    /// Handle input events
    pub fn handle_input(&mut self, event: &KeyEvent) -> bool {
        if !self.config.selectable && !self.config.scrollable {
            return false;
        }

        match (event.code, event.modifiers) {
            // Navigation
            (KeyCode::Up, KeyModifiers::NONE) => {
                self.select_previous()
            }
            (KeyCode::Down, KeyModifiers::NONE) => {
                // We need the items count - this is a simplified implementation
                self.select_next(100) // Placeholder
            }
            (KeyCode::PageUp, KeyModifiers::NONE) => {
                self.scroll_up(10);
                true
            }
            (KeyCode::PageDown, KeyModifiers::NONE) => {
                self.scroll_down(10, 100); // Placeholder
                true
            }
            (KeyCode::Home, KeyModifiers::NONE) => {
                self.select(Some(0));
                self.scroll_offset = 0;
                true
            }
            (KeyCode::End, KeyModifiers::NONE) => {
                // Would need actual items count
                self.select(Some(99)); // Placeholder
                true
            }
            // Content-specific shortcuts
            (KeyCode::Char('r'), KeyModifiers::NONE) => {
                // Refresh content
                true
            }
            (KeyCode::Char('f'), KeyModifiers::NONE) => {
                // Focus search/filter
                true
            }
            _ => false,
        }
    }

    /// Render the main panel
    pub fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState) {
        match self.content_type {
            ContentType::TrackList => self.render_track_list(frame, area, state),
            ContentType::SearchResults => self.render_search_results(frame, area, state),
            ContentType::DownloadProgress => self.render_download_progress(frame, area, state),
            ContentType::Settings => self.render_settings(frame, area, state),
            ContentType::Help => self.render_help(frame, area, state),
            ContentType::FileExplorer => self.render_file_explorer(frame, area, state),
            ContentType::Logs => self.render_logs(frame, area, state),
        }
    }

    /// Render track list content
    fn render_track_list(&mut self, frame: &mut Frame, area: Rect, state: &AppState) {
        let title = format!("Track Queue ({} items)", state.queue.items.len());
        let block = self.create_panel_block(&title);
        let inner = block.inner(area);

        RatatuiWidget::render(block, area, frame.buffer_mut());

        if state.queue.items.is_empty() {
            self.render_empty_message(frame, inner, "No tracks in queue. Add some tracks to get started.");
            return;
        }

        // Update list state first
        let mut list_state = self.list_state.clone();
        list_state.select(self.selected_index);

        // Create list items
        let items: Vec<ListItem> = state.queue.items
            .iter()
            .enumerate()
            .map(|(i, track)| self.create_track_list_item(i, track))
            .collect();

        let list = List::new(items)
            .highlight_style(self.styles.list_item_selected())
            .highlight_symbol("► ");

        ratatui::widgets::StatefulWidget::render(list, inner, frame.buffer_mut(), &mut list_state);
        self.list_state = list_state;
    }

    /// Render search results content
    fn render_search_results(&mut self, frame: &mut Frame, area: Rect, _state: &AppState) {
        let block = self.create_panel_block("Search Results");
        let inner = block.inner(area);

        RatatuiWidget::render(block, area, frame.buffer_mut());
        self.render_empty_message(frame, inner, "No search results. Use the search function to find tracks.");
    }

    /// Render download progress content
    fn render_download_progress(&mut self, frame: &mut Frame, area: Rect, state: &AppState) {
        let block = self.create_panel_block("Download Progress");
        let inner = block.inner(area);

        RatatuiWidget::render(block, area, frame.buffer_mut());

        let processing_count = state.queue.items.iter()
            .filter(|track| matches!(track.status, crate::ui::terminal::state::TrackStatus::Downloading | crate::ui::terminal::state::TrackStatus::Processing))
            .count();
        if processing_count == 0 {
            self.render_empty_message(frame, inner, "No active downloads.");
            return;
        }

        // Create a simple progress display
        let content = vec![
            Line::from(vec![
                Span::styled("Processing: ", self.styles.text_secondary()),
                Span::styled(processing_count.to_string(), self.styles.text_emphasis()),
                Span::raw(" tracks"),
            ]),
            Line::from(vec![
                Span::styled("Completed: ", self.styles.text_secondary()),
                Span::styled(state.stats.completed.to_string(), self.styles.text_success()),
            ]),
            Line::from(vec![
                Span::styled("Failed: ", self.styles.text_secondary()),
                Span::styled(state.stats.failed.to_string(), self.styles.text_error()),
            ]),
        ];

        let paragraph = Paragraph::new(content);
        RatatuiWidget::render(paragraph, inner, frame.buffer_mut());
    }

    /// Render settings content
    fn render_settings(&mut self, frame: &mut Frame, area: Rect, _state: &AppState) {
        let block = self.create_panel_block("Settings");
        let inner = block.inner(area);

        RatatuiWidget::render(block, area, frame.buffer_mut());
        self.render_empty_message(frame, inner, "Settings panel - implementation pending");
    }

    /// Render help content
    fn render_help(&mut self, frame: &mut Frame, area: Rect, _state: &AppState) {
        let block = self.create_panel_block("Help");
        let inner = block.inner(area);

        RatatuiWidget::render(block, area, frame.buffer_mut());

        let help_content = vec![
            Line::from(Span::styled("lrcget-cli - Lyrics Downloader", self.styles.text_bold())),
            Line::from(""),
            Line::from(Span::styled("Navigation:", self.styles.text_emphasis())),
            Line::from("  ↑/↓ - Navigate items"),
            Line::from("  Tab - Switch panels"),
            Line::from("  Enter - Select/Activate"),
            Line::from("  Esc - Cancel/Back"),
            Line::from(""),
            Line::from(Span::styled("Actions:", self.styles.text_emphasis())),
            Line::from("  s - Start download"),
            Line::from("  r - Retry failed"),
            Line::from("  c - Clear queue"),
            Line::from("  q - Quit application"),
        ];

        let paragraph = Paragraph::new(help_content);
        RatatuiWidget::render(paragraph, inner, frame.buffer_mut());
    }

    /// Render file explorer content
    fn render_file_explorer(&mut self, frame: &mut Frame, area: Rect, _state: &AppState) {
        let block = self.create_panel_block("File Explorer");
        let inner = block.inner(area);

        RatatuiWidget::render(block, area, frame.buffer_mut());
        self.render_empty_message(frame, inner, "File explorer - implementation pending");
    }

    /// Render logs content
    fn render_logs(&mut self, frame: &mut Frame, area: Rect, state: &AppState) {
        let title = format!("Logs ({} entries)", state.logs.entries.len());
        let block = self.create_panel_block(&title);
        let inner = block.inner(area);

        RatatuiWidget::render(block, area, frame.buffer_mut());

        if state.logs.entries.is_empty() {
            self.render_empty_message(frame, inner, "No log entries yet.");
            return;
        }

        // Create list items for logs
        let log_items: Vec<ListItem> = state.logs.entries
            .iter()
            .map(|log_entry| {
                let level_style = match log_entry.level {
                    crate::ui::terminal::state::LogLevel::Error => self.styles.text_error(),
                    crate::ui::terminal::state::LogLevel::Warning => self.styles.text_warning(),
                    crate::ui::terminal::state::LogLevel::Info => self.styles.text_info(),
                    crate::ui::terminal::state::LogLevel::Debug => self.styles.text_muted(),
                };

                let content = Line::from(vec![
                    Span::styled(format!("[{:?}]", log_entry.level), level_style),
                    Span::raw(" "),
                    Span::styled(&log_entry.message, self.styles.text_primary()),
                ]);

                ListItem::new(content)
            })
            .collect();

        let list = List::new(log_items)
            .highlight_style(self.styles.list_item_selected())
            .highlight_symbol("► ");

        self.list_state.select(self.selected_index);
        RatatuiWidget::render(list, inner, frame.buffer_mut());
    }

    /// Create a panel block with optional title
    fn create_panel_block<'a>(&self, title: &'a str) -> Block<'a> {
        let mut block = Block::default();

        if self.config.show_borders {
            block = block.borders(Borders::ALL)
                .border_style(self.styles.panel_border());
        }

        if self.config.show_title {
            block = block.title(title)
                .title_style(self.styles.panel_title());
        }

        block.style(self.styles.panel_background())
    }

    /// Create a list item for a track
    fn create_track_list_item<'a>(&self, _index: usize, track: &'a TrackQueueItem) -> ListItem<'a> {
        let status_icon = match track.status {
            TrackStatus::Pending => "⏳",
            TrackStatus::Downloading => "⬇️",
            TrackStatus::Processing => "⚙️",
            TrackStatus::Completed => "✅",
            TrackStatus::Failed => "❌",
            TrackStatus::Skipped => "⏭️",
        };

        let status_style = match track.status {
            TrackStatus::Pending => self.styles.state_pending(),
            TrackStatus::Downloading => self.styles.state_processing(),
            TrackStatus::Processing => self.styles.state_processing(),
            TrackStatus::Completed => self.styles.state_completed(),
            TrackStatus::Failed => self.styles.state_failed(),
            TrackStatus::Skipped => self.styles.state_skipped(),
        };

        let content = Line::from(vec![
            Span::styled(status_icon, status_style),
            Span::raw(" "),
            Span::styled(&track.title, self.styles.text_primary()),
            Span::raw(" - "),
            Span::styled(&track.artist, self.styles.text_secondary()),
        ]);

        ListItem::new(content)
    }

    /// Render an empty message
    fn render_empty_message(&self, frame: &mut Frame, area: Rect, message: &str) {
        let paragraph = Paragraph::new(Line::from(Span::styled(
            message,
            self.styles.text_muted(),
        )))
        .alignment(ratatui::layout::Alignment::Center);

        // Center the message vertically
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Ratio(1, 3),
                Constraint::Length(1),
                Constraint::Ratio(2, 3),
            ])
            .split(area);

        RatatuiWidget::render(paragraph, vertical_chunks[1], frame.buffer_mut());
    }

    /// Get title for current content type
    pub fn get_title(&self) -> &'static str {
        match self.content_type {
            ContentType::TrackList => "Track List",
            ContentType::SearchResults => "Search Results",
            ContentType::DownloadProgress => "Download Progress",
            ContentType::Settings => "Settings",
            ContentType::Help => "Help",
            ContentType::FileExplorer => "File Explorer",
            ContentType::Logs => "Logs",
        }
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
                total: 5,
                processed_tracks: 2,
                completed: 1,
                failed: 1,
                skipped: 0,
                pending: 3,
                processing: 1,
                session_start: SystemTime::now(),
            },
            logs: vec![
                LogEntry {
                    timestamp: SystemTime::now(),
                    level: LogLevel::Info,
                    message: "Test log message".to_string(),
                },
            ],
            track_queue: vec![
                TrackQueueItem {
                    title: "Test Track".to_string(),
                    artist: "Test Artist".to_string(),
                    album: Some("Test Album".to_string()),
                    status: TrackStatus::Pending,
                    file_path: None,
                },
            ],
        }
    }

    #[test]
    fn test_main_panel_creation() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let panel = MainPanel::new(styles);

        assert_eq!(panel.content_type, ContentType::TrackList);
        assert!(panel.config.show_borders);
        assert!(panel.config.selectable);
    }

    #[test]
    fn test_content_type_switching() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let mut panel = MainPanel::new(styles);

        panel.set_content_type(ContentType::Help);
        assert_eq!(panel.content_type(), ContentType::Help);

        panel.set_content_type(ContentType::Settings);
        assert_eq!(panel.content_type(), ContentType::Settings);
    }

    #[test]
    fn test_selection() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let mut panel = MainPanel::new(styles);

        assert_eq!(panel.selected(), None);

        panel.select(Some(5));
        assert_eq!(panel.selected(), Some(5));

        panel.select(None);
        assert_eq!(panel.selected(), None);
    }

    #[test]
    fn test_navigation() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let mut panel = MainPanel::new(styles);

        // Test next selection
        assert!(panel.select_next(10));
        assert_eq!(panel.selected(), Some(0));

        assert!(panel.select_next(10));
        assert_eq!(panel.selected(), Some(1));

        // Test previous selection
        assert!(panel.select_previous());
        assert_eq!(panel.selected(), Some(0));

        assert!(!panel.select_previous()); // Can't go before 0
        assert_eq!(panel.selected(), Some(0));
    }

    #[test]
    fn test_scrolling() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let mut panel = MainPanel::new(styles);

        assert_eq!(panel.scroll_offset, 0);

        panel.scroll_down(5, 20);
        assert_eq!(panel.scroll_offset, 5);

        panel.scroll_up(3);
        assert_eq!(panel.scroll_offset, 2);

        panel.scroll_up(10); // Should not go below 0
        assert_eq!(panel.scroll_offset, 0);
    }

    #[test]
    fn test_update_content_type() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let mut panel = MainPanel::new(styles);

        let state = create_test_state();
        panel.update(&state);

        // Should switch to track list when tracks are available
        assert_eq!(panel.content_type, ContentType::TrackList);
    }

    #[test]
    fn test_titles() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let mut panel = MainPanel::new(styles);

        panel.set_content_type(ContentType::Help);
        assert_eq!(panel.get_title(), "Help");

        panel.set_content_type(ContentType::Settings);
        assert_eq!(panel.get_title(), "Settings");

        panel.set_content_type(ContentType::TrackList);
        assert_eq!(panel.get_title(), "Track List");
    }
}