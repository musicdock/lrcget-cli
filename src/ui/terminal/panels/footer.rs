//! Footer panel with dynamic hotkeys and help information
//!
//! Displays context-sensitive hotkeys, help text, and status information
//! at the bottom of the screen.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph, Widget as RatatuiWidget},
    text::{Line, Span},
    Frame,
};
use crossterm::event::{KeyEvent, KeyCode, KeyModifiers};
use std::collections::HashMap;

use super::super::{
    styles::ComponentStyles,
    state::AppState,
};

/// Hotkey definition
#[derive(Debug, Clone)]
pub struct Hotkey {
    pub key: String,
    pub description: String,
    pub enabled: bool,
}

impl Hotkey {
    /// Create a new hotkey
    pub fn new<K: Into<String>, D: Into<String>>(key: K, description: D) -> Self {
        Self {
            key: key.into(),
            description: description.into(),
            enabled: true,
        }
    }

    /// Create a disabled hotkey
    pub fn disabled<K: Into<String>, D: Into<String>>(key: K, description: D) -> Self {
        Self {
            key: key.into(),
            description: description.into(),
            enabled: false,
        }
    }

    /// Enable or disable this hotkey
    pub fn set_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Context for dynamic hotkey display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HotkeyContext {
    Global,
    Download,
    Search,
    Settings,
    Help,
    FileExplorer,
    TrackList,
}

/// Footer panel configuration
#[derive(Debug, Clone)]
pub struct FooterConfig {
    pub show_hotkeys: bool,
    pub show_status: bool,
    pub max_hotkeys: usize,
    pub compact_mode: bool,
}

impl Default for FooterConfig {
    fn default() -> Self {
        Self {
            show_hotkeys: true,
            show_status: true,
            max_hotkeys: 8,
            compact_mode: false,
        }
    }
}

/// Footer panel component
#[derive(Debug)]
pub struct FooterPanel {
    styles: ComponentStyles,
    config: FooterConfig,
    current_context: HotkeyContext,
    hotkey_registry: HashMap<HotkeyContext, Vec<Hotkey>>,
    custom_message: Option<String>,
}

impl FooterPanel {
    /// Create a new footer panel
    pub fn new(styles: ComponentStyles) -> Self {
        let mut footer = Self {
            styles,
            config: FooterConfig::default(),
            current_context: HotkeyContext::Global,
            hotkey_registry: HashMap::new(),
            custom_message: None,
        };

        footer.initialize_hotkeys();
        footer
    }

    /// Configure the footer panel
    pub fn with_config(mut self, config: FooterConfig) -> Self {
        self.config = config;
        self
    }

    /// Set current context for dynamic hotkeys
    pub fn set_context(&mut self, context: HotkeyContext) {
        self.current_context = context;
    }

    /// Display a temporary custom message
    pub fn show_message<S: Into<String>>(&mut self, message: S) {
        self.custom_message = Some(message.into());
    }

    /// Clear custom message
    pub fn clear_message(&mut self) {
        self.custom_message = None;
    }

    /// Add or update hotkeys for a specific context
    pub fn set_hotkeys(&mut self, context: HotkeyContext, hotkeys: Vec<Hotkey>) {
        self.hotkey_registry.insert(context, hotkeys);
    }

    /// Add a single hotkey to a context
    pub fn add_hotkey(&mut self, context: HotkeyContext, hotkey: Hotkey) {
        self.hotkey_registry.entry(context).or_insert_with(Vec::new).push(hotkey);
    }

    /// Update footer with current state
    pub fn update(&mut self, state: &AppState) {
        // Update context based on app state
        self.update_context_from_state(state);

        // Clear temporary messages after some time
        // In a real implementation, you might want to track message time
    }

    /// Handle input events
    pub fn handle_input(&mut self, event: &KeyEvent) -> bool {
        // Footer can handle help key globally
        match (event.code, event.modifiers) {
            (KeyCode::F(1), KeyModifiers::NONE) | (KeyCode::Char('?'), KeyModifiers::NONE) => {
                self.set_context(HotkeyContext::Help);
                true
            }
            (KeyCode::Esc, KeyModifiers::NONE) => {
                self.clear_message();
                self.set_context(HotkeyContext::Global);
                true
            }
            _ => false,
        }
    }

    /// Render the footer panel
    pub fn render(&self, frame: &mut Frame, area: Rect, _state: &AppState) {
        if self.config.compact_mode {
            self.render_compact(frame, area);
        } else {
            self.render_full(frame, area);
        }
    }

    /// Render full footer with hotkeys and status
    fn render_full(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.styles.panel_border())
            .style(self.styles.footer_background());

        let inner = block.inner(area);

        // If there's a custom message, show it prominently
        if let Some(ref message) = self.custom_message {
            let paragraph = Paragraph::new(Line::from(Span::styled(
                message,
                self.styles.text_emphasis(),
            )))
            .alignment(Alignment::Center);

            RatatuiWidget::render(block, area, frame.buffer_mut());
            RatatuiWidget::render(paragraph, inner, frame.buffer_mut());
            return;
        }

        // Split footer into sections
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(0),     // Hotkeys (flexible)
                Constraint::Length(20), // Status info
            ])
            .split(inner);

        RatatuiWidget::render(block, area, frame.buffer_mut());

        // Render hotkeys
        if self.config.show_hotkeys {
            self.render_hotkeys(frame, chunks[0]);
        }

        // Render status
        if self.config.show_status {
            self.render_status(frame, chunks[1]);
        }
    }

    /// Render compact footer for small screens
    fn render_compact(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.styles.panel_border())
            .style(self.styles.footer_background());

        let inner = block.inner(area);

        // Show only the most important hotkeys
        let essential_hotkeys = self.get_essential_hotkeys();
        let hotkey_text = self.format_hotkeys_compact(&essential_hotkeys);

        let paragraph = Paragraph::new(Line::from(hotkey_text))
            .alignment(Alignment::Center);

        RatatuiWidget::render(block, area, frame.buffer_mut());
        RatatuiWidget::render(paragraph, inner, frame.buffer_mut());
    }

    /// Render hotkeys section
    fn render_hotkeys(&self, frame: &mut Frame, area: Rect) {
        let hotkeys = self.get_current_hotkeys();
        let lines = self.format_hotkeys_multiline(&hotkeys, area.width as usize);

        let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
        RatatuiWidget::render(paragraph, area, frame.buffer_mut());
    }

    /// Render status section
    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let context_name = self.get_context_name();
        let content = vec![
            Line::from(Span::styled("Context:", self.styles.text_muted())),
            Line::from(Span::styled(context_name, self.styles.text_secondary())),
        ];

        let paragraph = Paragraph::new(content).alignment(Alignment::Right);
        RatatuiWidget::render(paragraph, area, frame.buffer_mut());
    }

    /// Initialize default hotkeys for all contexts
    fn initialize_hotkeys(&mut self) {
        // Global hotkeys
        self.set_hotkeys(HotkeyContext::Global, vec![
            Hotkey::new("q", "Quit"),
            Hotkey::new("?", "Help"),
            Hotkey::new("Tab", "Next Panel"),
            Hotkey::new("Enter", "Select"),
            Hotkey::new("Esc", "Cancel"),
        ]);

        // Download context hotkeys
        self.set_hotkeys(HotkeyContext::Download, vec![
            Hotkey::new("Space", "Pause/Resume"),
            Hotkey::new("s", "Start Download"),
            Hotkey::new("r", "Retry Failed"),
            Hotkey::new("c", "Clear Queue"),
            Hotkey::new("d", "Download Selected"),
        ]);

        // Search context hotkeys
        self.set_hotkeys(HotkeyContext::Search, vec![
            Hotkey::new("/", "Search"),
            Hotkey::new("n", "Next Result"),
            Hotkey::new("p", "Previous Result"),
            Hotkey::new("f", "Filter"),
            Hotkey::new("Enter", "Download"),
        ]);

        // Settings context hotkeys
        self.set_hotkeys(HotkeyContext::Settings, vec![
            Hotkey::new("s", "Save Settings"),
            Hotkey::new("r", "Reset to Default"),
            Hotkey::new("t", "Toggle Theme"),
            Hotkey::new("Enter", "Edit Value"),
        ]);

        // Help context hotkeys
        self.set_hotkeys(HotkeyContext::Help, vec![
            Hotkey::new("↑↓", "Navigate"),
            Hotkey::new("PgUp/PgDn", "Page Up/Down"),
            Hotkey::new("Home/End", "First/Last"),
            Hotkey::new("Esc", "Close Help"),
        ]);

        // File explorer hotkeys
        self.set_hotkeys(HotkeyContext::FileExplorer, vec![
            Hotkey::new("Enter", "Open Directory"),
            Hotkey::new("Space", "Select File"),
            Hotkey::new("a", "Select All"),
            Hotkey::new("r", "Refresh"),
            Hotkey::new("h", "Show Hidden"),
        ]);

        // Track list hotkeys
        self.set_hotkeys(HotkeyContext::TrackList, vec![
            Hotkey::new("↑↓", "Navigate"),
            Hotkey::new("Space", "Toggle Select"),
            Hotkey::new("a", "Select All"),
            Hotkey::new("Del", "Remove"),
            Hotkey::new("Enter", "Download"),
        ]);
    }

    /// Get hotkeys for current context
    fn get_current_hotkeys(&self) -> Vec<Hotkey> {
        let mut hotkeys = Vec::new();

        // Always include global hotkeys
        if let Some(global) = self.hotkey_registry.get(&HotkeyContext::Global) {
            hotkeys.extend(global.clone());
        }

        // Add context-specific hotkeys
        if self.current_context != HotkeyContext::Global {
            if let Some(context_hotkeys) = self.hotkey_registry.get(&self.current_context) {
                hotkeys.extend(context_hotkeys.clone());
            }
        }

        // Limit number of hotkeys displayed
        hotkeys.truncate(self.config.max_hotkeys);
        hotkeys
    }

    /// Get essential hotkeys for compact mode
    fn get_essential_hotkeys(&self) -> Vec<Hotkey> {
        vec![
            Hotkey::new("q", "Quit"),
            Hotkey::new("?", "Help"),
            Hotkey::new("Tab", "Navigate"),
        ]
    }

    /// Format hotkeys for compact display
    fn format_hotkeys_compact<'a>(&self, hotkeys: &'a [Hotkey]) -> Vec<Span<'a>> {
        let mut spans = Vec::new();

        for (i, hotkey) in hotkeys.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(" • "));
            }

            let style = if hotkey.enabled {
                self.styles.footer_hotkey()
            } else {
                self.styles.text_muted()
            };

            spans.push(Span::styled(&hotkey.key, style));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(&hotkey.description, self.styles.footer_description()));
        }

        spans
    }

    /// Format hotkeys for multiline display
    fn format_hotkeys_multiline<'a>(&self, hotkeys: &'a [Hotkey], width: usize) -> Vec<Line<'a>> {
        let mut lines = Vec::new();
        let mut current_line = Vec::new();
        let mut current_width = 0;

        for hotkey in hotkeys {
            let hotkey_text = format!("{} {}", hotkey.key, hotkey.description);
            let hotkey_len = hotkey_text.len() + 3; // Add spacing

            // Check if we need a new line
            if current_width + hotkey_len > width && !current_line.is_empty() {
                lines.push(Line::from(current_line));
                current_line = Vec::new();
                current_width = 0;
            }

            // Add separator if not first item on line
            if !current_line.is_empty() {
                current_line.push(Span::raw(" • "));
                current_width += 3;
            }

            // Add hotkey
            let style = if hotkey.enabled {
                self.styles.footer_hotkey()
            } else {
                self.styles.text_muted()
            };

            current_line.push(Span::styled(&hotkey.key, style));
            current_line.push(Span::raw(" "));
            current_line.push(Span::styled(&hotkey.description, self.styles.footer_description()));

            current_width += hotkey_len;
        }

        // Add remaining line
        if !current_line.is_empty() {
            lines.push(Line::from(current_line));
        }

        // If no lines, add empty line
        if lines.is_empty() {
            lines.push(Line::from(""));
        }

        lines
    }

    /// Get display name for current context
    fn get_context_name(&self) -> &str {
        match self.current_context {
            HotkeyContext::Global => "Global",
            HotkeyContext::Download => "Download",
            HotkeyContext::Search => "Search",
            HotkeyContext::Settings => "Settings",
            HotkeyContext::Help => "Help",
            HotkeyContext::FileExplorer => "Explorer",
            HotkeyContext::TrackList => "Track List",
        }
    }

    /// Update context based on application state
    fn update_context_from_state(&mut self, state: &AppState) {
        // This is a simplified implementation
        // In a real app, you'd determine context from app state
        let processing_count = state.queue.items.iter()
            .filter(|track| matches!(track.status, crate::ui::terminal::state::TrackStatus::Downloading | crate::ui::terminal::state::TrackStatus::Processing))
            .count();
        if processing_count > 0 {
            self.current_context = HotkeyContext::Download;
        } else if state.stats.total_processed > 0 {
            self.current_context = HotkeyContext::TrackList;
        }
        // Keep current context otherwise
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::terminal::{colors::ColorPalette, state::Statistics};
    use std::time::SystemTime;

    fn create_test_state() -> AppState {
        AppState {
            stats: Statistics {
                total: 10,
                processed_tracks: 5,
                completed: 4,
                failed: 1,
                skipped: 0,
                pending: 5,
                processing: 2,
                session_start: SystemTime::now(),
            },
            logs: Vec::new(),
            track_queue: Vec::new(),
        }
    }

    #[test]
    fn test_footer_panel_creation() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let footer = FooterPanel::new(styles);

        assert_eq!(footer.current_context, HotkeyContext::Global);
        assert!(footer.config.show_hotkeys);
        assert!(footer.config.show_status);
    }

    #[test]
    fn test_hotkey_creation() {
        let hotkey = Hotkey::new("q", "Quit");
        assert_eq!(hotkey.key, "q");
        assert_eq!(hotkey.description, "Quit");
        assert!(hotkey.enabled);

        let disabled_hotkey = Hotkey::disabled("x", "Disabled");
        assert!(!disabled_hotkey.enabled);
    }

    #[test]
    fn test_context_switching() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let mut footer = FooterPanel::new(styles);

        footer.set_context(HotkeyContext::Download);
        assert_eq!(footer.current_context, HotkeyContext::Download);

        footer.set_context(HotkeyContext::Search);
        assert_eq!(footer.current_context, HotkeyContext::Search);
    }

    #[test]
    fn test_custom_messages() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let mut footer = FooterPanel::new(styles);

        footer.show_message("Test message");
        assert!(footer.custom_message.is_some());
        assert_eq!(footer.custom_message.as_ref().unwrap(), "Test message");

        footer.clear_message();
        assert!(footer.custom_message.is_none());
    }

    #[test]
    fn test_hotkey_registry() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let mut footer = FooterPanel::new(styles);

        let custom_hotkeys = vec![
            Hotkey::new("x", "Custom Action"),
            Hotkey::new("y", "Another Action"),
        ];

        footer.set_hotkeys(HotkeyContext::Settings, custom_hotkeys);

        footer.set_context(HotkeyContext::Settings);
        let current_hotkeys = footer.get_current_hotkeys();

        // Should include both global and settings hotkeys
        assert!(current_hotkeys.len() > 2);
        assert!(current_hotkeys.iter().any(|h| h.key == "x"));
    }

    #[test]
    fn test_essential_hotkeys() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let footer = FooterPanel::new(styles);

        let essential = footer.get_essential_hotkeys();
        assert_eq!(essential.len(), 3);
        assert!(essential.iter().any(|h| h.key == "q"));
        assert!(essential.iter().any(|h| h.key == "?"));
    }

    #[test]
    fn test_context_names() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let mut footer = FooterPanel::new(styles);

        footer.set_context(HotkeyContext::Download);
        assert_eq!(footer.get_context_name(), "Download");

        footer.set_context(HotkeyContext::Search);
        assert_eq!(footer.get_context_name(), "Search");

        footer.set_context(HotkeyContext::Global);
        assert_eq!(footer.get_context_name(), "Global");
    }

    #[test]
    fn test_update_context_from_state() {
        let palette = ColorPalette::dark();
        let styles = ComponentStyles::new(palette);
        let mut footer = FooterPanel::new(styles);

        let state = create_test_state();
        footer.update_context_from_state(&state);

        // Should switch to download context when processing tracks
        assert_eq!(footer.current_context, HotkeyContext::Download);
    }
}