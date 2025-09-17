//! Queue panel implementation for displaying track download queue
//!
//! This module provides the visual representation of the download queue with
//! interactive navigation, filtering, and real-time status updates.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState,
    },
};
use std::collections::VecDeque;

use crate::ui::terminal::{
    colors::{ColorPalette, ColorRole},
    input::ScrollDirection,
    state::{TrackQueueItem, TrackStatus, UiState},
    widgets::{Widget, WidgetState},
};


/// Queue panel widget for displaying and managing track queue
#[derive(Debug)]
pub struct QueuePanel {
    /// Widget state for focus and styling
    pub state: WidgetState,
    /// Color palette for styling
    pub colors: ColorPalette,
    /// List state for navigation
    pub list_state: ListState,
    /// Scrollbar state
    pub scrollbar_state: ScrollbarState,
    /// Current filter text
    pub filter: String,
    /// Whether filter input is active
    pub filter_active: bool,
    /// Visible area for calculations
    pub visible_area: Rect,
}

impl QueuePanel {
    /// Create a new queue panel
    pub fn new(colors: ColorPalette) -> Self {
        Self {
            state: WidgetState::Normal,
            colors,
            list_state: ListState::default(),
            scrollbar_state: ScrollbarState::default(),
            filter: String::new(),
            filter_active: false,
            visible_area: Rect::default(),
        }
    }

    /// Set focus state
    pub fn set_focus(&mut self, focused: bool) {
        self.state = if focused {
            WidgetState::Focused
        } else {
            WidgetState::Normal
        };
    }

    /// Handle scroll navigation
    pub fn handle_scroll(&mut self, direction: ScrollDirection, queue_size: usize) {
        if queue_size == 0 {
            return;
        }

        let current = self.list_state.selected().unwrap_or(0);
        let page_size = (self.visible_area.height.saturating_sub(2)) as usize;

        let new_index = match direction {
            ScrollDirection::Up => current.saturating_sub(1),
            ScrollDirection::Down => (current + 1).min(queue_size.saturating_sub(1)),
            ScrollDirection::PageUp => current.saturating_sub(page_size),
            ScrollDirection::PageDown => (current + page_size).min(queue_size.saturating_sub(1)),
            ScrollDirection::Home => 0,
            ScrollDirection::End => queue_size.saturating_sub(1),
        };

        self.list_state.select(Some(new_index));
        self.scrollbar_state = ScrollbarState::new(queue_size).position(new_index);
    }

    /// Filter tracks based on current filter
    pub fn filter_tracks<'a>(&self, tracks: &'a VecDeque<TrackQueueItem>) -> Vec<&'a TrackQueueItem> {
        if self.filter.is_empty() {
            tracks.iter().collect()
        } else {
            let filter_lower = self.filter.to_lowercase();
            tracks
                .iter()
                .filter(|track| {
                    track.title.to_lowercase().contains(&filter_lower)
                        || track.artist.to_lowercase().contains(&filter_lower)
                        || track.album.to_lowercase().contains(&filter_lower)
                })
                .collect()
        }
    }

    /// Set filter text
    pub fn set_filter(&mut self, filter: String) {
        self.filter = filter;
        // Reset selection when filter changes
        self.list_state.select(Some(0));
    }

    /// Toggle filter input mode
    pub fn toggle_filter(&mut self) {
        self.filter_active = !self.filter_active;
    }

    /// Get currently selected track ID
    pub fn selected_track_id(&self, filtered_tracks: &[&TrackQueueItem]) -> Option<u64> {
        self.list_state
            .selected()
            .and_then(|index| filtered_tracks.get(index))
            .map(|track| track.id)
    }

    /// Render the queue panel
    pub fn render_queue(&self, area: Rect, buf: &mut Buffer, tracks: &VecDeque<TrackQueueItem>) {
        // Note: visible_area would need to be updated elsewhere for navigation

        // Create layout for filter and list
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(if self.filter_active { 3 } else { 0 }),
                Constraint::Min(0),
            ])
            .split(area);

        // Render filter if active
        if self.filter_active {
            self.render_filter(chunks[0], buf);
        }

        let list_area = if self.filter_active { chunks[1] } else { area };

        // Filter tracks
        let filtered_tracks = self.filter_tracks(tracks);

        // Clone the list state to avoid borrowing issues
        let mut list_state = self.list_state.clone();

        // Create list items
        let items: Vec<ListItem> = filtered_tracks
            .iter()
            .map(|track| self.create_track_item(track))
            .collect();

        // Create the list widget
        let block_style = match self.state {
            WidgetState::Focused => Style::default()
                .fg(self.colors.get(ColorRole::Accent))
                .add_modifier(Modifier::BOLD),
            _ => Style::default().fg(self.colors.get(ColorRole::BorderPrimary)),
        };

        let title = if self.filter.is_empty() {
            format!(" Queue ({}) ", filtered_tracks.len())
        } else {
            format!(" Queue ({}/{}) [Filter: {}] ",
                filtered_tracks.len(),
                tracks.len(),
                self.filter
            )
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(block_style),
            )
            .highlight_style(
                Style::default()
                    .bg(self.colors.get(ColorRole::Accent))
                    .fg(self.colors.get(ColorRole::Background))
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        // Render list
        ratatui::widgets::StatefulWidget::render(list, list_area, buf, &mut list_state);

        // Render scrollbar if needed
        if filtered_tracks.len() > list_area.height.saturating_sub(2) as usize {
            let scrollbar_area = Rect {
                x: list_area.right().saturating_sub(1),
                y: list_area.y + 1,
                width: 1,
                height: list_area.height.saturating_sub(2),
            };

            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            let mut scrollbar_state = self.scrollbar_state.clone();
            ratatui::widgets::StatefulWidget::render(
                scrollbar,
                scrollbar_area,
                buf,
                &mut scrollbar_state,
            );
        }
    }

    /// Render filter input
    fn render_filter(&self, area: Rect, buf: &mut Buffer) {
        let filter_style = if self.filter_active {
            Style::default()
                .fg(self.colors.get(ColorRole::Accent))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.colors.get(ColorRole::BorderPrimary))
        };

        let filter_widget = Paragraph::new(format!("/{}", self.filter))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Filter ")
                    .border_style(filter_style),
            )
            .style(Style::default().fg(self.colors.get(ColorRole::TextPrimary)));

        ratatui::widgets::Widget::render(filter_widget, area, buf);
    }

    /// Create a list item for a track
    fn create_track_item(&self, track: &TrackQueueItem) -> ListItem {
        let status_symbol = self.get_status_symbol(track.status);
        let status_color = self.get_status_color(track.status);

        // Format progress
        let progress_text = if track.status == TrackStatus::Downloading {
            format!(" {:3.0}%", track.progress * 100.0)
        } else {
            String::from("     ")
        };

        // Format speed
        let speed_text = if let Some(speed) = track.download_speed {
            format!(" {:4.1}/min", speed)
        } else {
            String::from("         ")
        };

        // Create the main line with status, title, and artist
        let main_line = Line::from(vec![
            Span::styled(
                format!("{} ", status_symbol),
                Style::default().fg(status_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<30}", truncate_string(&track.title, 30)),
                Style::default().fg(self.colors.get(ColorRole::TextPrimary)),
            ),
            Span::styled(
                format!(" - {:<20}", truncate_string(&track.artist, 20)),
                Style::default().fg(self.colors.get(ColorRole::TextSecondary)),
            ),
            Span::styled(
                progress_text,
                Style::default().fg(self.colors.get(ColorRole::Accent)),
            ),
            Span::styled(
                speed_text,
                Style::default().fg(self.colors.get(ColorRole::TextSecondary)),
            ),
        ]);

        // Add error message if present
        let mut lines = vec![main_line];
        if let Some(error) = &track.error_message {
            lines.push(Line::from(vec![
                Span::styled("   ", Style::default()),
                Span::styled(
                    format!("Error: {}", truncate_string(error, 60)),
                    Style::default()
                        .fg(self.colors.get(ColorRole::Error))
                        .add_modifier(Modifier::ITALIC),
                ),
            ]));
        }

        ListItem::new(lines)
    }

    /// Get symbol for track status
    fn get_status_symbol(&self, status: TrackStatus) -> &'static str {
        match status {
            TrackStatus::Pending => "⏳",
            TrackStatus::Downloading => "⬇",
            TrackStatus::Processing => "⚙",
            TrackStatus::Completed => "✓",
            TrackStatus::Failed => "✗",
            TrackStatus::Skipped => "⊘",
        }
    }

    /// Get color for track status
    fn get_status_color(&self, status: TrackStatus) -> Color {
        match status {
            TrackStatus::Pending => self.colors.get(ColorRole::Pending),
            TrackStatus::Downloading => self.colors.get(ColorRole::Accent),
            TrackStatus::Processing => self.colors.get(ColorRole::Processing),
            TrackStatus::Completed => self.colors.get(ColorRole::Completed),
            TrackStatus::Failed => self.colors.get(ColorRole::Failed),
            TrackStatus::Skipped => self.colors.get(ColorRole::Skipped),
        }
    }
}

impl Widget for QueuePanel {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        // This is a placeholder - actual rendering should be done via render_queue
        let placeholder = Paragraph::new("Queue Panel")
            .block(Block::default().borders(Borders::ALL).title(" Queue "));
        ratatui::widgets::Widget::render(placeholder, area, buf);
    }

    fn can_focus(&self) -> bool {
        true
    }

    fn widget_id(&self) -> &str {
        "queue_panel"
    }
}

/// Utility function to truncate strings with ellipsis
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::terminal::colors::ColorPalette;
    use std::time::SystemTime;

    #[test]
    fn test_queue_panel_creation() {
        let colors = ColorPalette::default();
        let panel = QueuePanel::new(colors);
        assert_eq!(panel.state, WidgetState::Normal);
        assert!(panel.filter.is_empty());
        assert!(!panel.filter_active);
    }

    #[test]
    fn test_scroll_navigation() {
        let colors = ColorPalette::default();
        let mut panel = QueuePanel::new(colors);
        panel.visible_area = Rect::new(0, 0, 80, 20);

        // Test basic navigation
        panel.handle_scroll(ScrollDirection::Down, 10);
        assert_eq!(panel.list_state.selected(), Some(1));

        panel.handle_scroll(ScrollDirection::Up, 10);
        assert_eq!(panel.list_state.selected(), Some(0));

        // Test bounds
        panel.handle_scroll(ScrollDirection::End, 10);
        assert_eq!(panel.list_state.selected(), Some(9));
    }

    #[test]
    fn test_track_filtering() {
        let colors = ColorPalette::default();
        let mut panel = QueuePanel::new(colors);

        let mut tracks = VecDeque::new();
        tracks.push_back(TrackQueueItem {
            id: 1,
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            album: "Test Album".to_string(),
            status: TrackStatus::Pending,
            progress: 0.0,
            error_message: None,
            download_speed: None,
            timestamp: SystemTime::now(),
            started_at: None,
            completed_at: None,
        });

        // Test no filter
        let filtered = panel.filter_tracks(&tracks);
        assert_eq!(filtered.len(), 1);

        // Test filter match
        panel.set_filter("Test".to_string());
        let filtered = panel.filter_tracks(&tracks);
        assert_eq!(filtered.len(), 1);

        // Test filter no match
        panel.set_filter("NoMatch".to_string());
        let filtered = panel.filter_tracks(&tracks);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("this is a long string", 10), "this is...");
        assert_eq!(truncate_string("exact", 5), "exact");
    }
}