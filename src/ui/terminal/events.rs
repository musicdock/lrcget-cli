//! Event handling system for terminal UI
//!
//! Manages keyboard input, mouse events, terminal resize, and application events
//! with efficient processing and proper event routing to UI components.

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Application events that can trigger UI updates
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Keyboard input event
    KeyPress(KeyEvent),
    /// Mouse input event
    MouseEvent(MouseEvent),
    /// Terminal was resized
    Resize(u16, u16),
    /// Application should quit
    Quit,
    /// Pause/resume operation
    TogglePause,
    /// Refresh display
    Refresh,
    /// Panel focus change
    FocusPanel(PanelId),
    /// Filter/search activated
    Search(String),
    /// Show help
    ShowHelp,
    /// Show configuration
    ShowConfig,
    /// Application update from background task
    AppUpdate(UpdateType),
}

/// Types of application updates from background tasks
#[derive(Debug, Clone)]
pub enum UpdateType {
    /// Progress update for current operation
    Progress { current: usize, total: usize },
    /// New song started processing
    SongStarted { song: String, artist: String },
    /// Song completed processing
    SongCompleted { song: String, success: bool, message: Option<String> },
    /// Statistics update
    StatsUpdate { songs_per_min: f64, success_rate: f64 },
    /// Error occurred
    Error { message: String },
}

/// Panel identifiers for focus management
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelId {
    Queue,
    Performance,
    Statistics,
    Logs,
    Header,
    Footer,
}

/// Event handler for terminal UI
pub struct EventHandler {
    /// Channel for receiving events from background tasks
    event_rx: mpsc::UnboundedReceiver<AppEvent>,
    /// Channel for sending events to background tasks
    event_tx: mpsc::UnboundedSender<AppEvent>,
    /// Last input time for timeout handling
    last_input: Instant,
    /// Current panel focus
    current_focus: Option<PanelId>,
    /// Whether help is currently shown
    help_visible: bool,
}

impl EventHandler {
    pub fn new() -> (Self, mpsc::UnboundedSender<AppEvent>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let handler = Self {
            event_rx,
            event_tx: event_tx.clone(),
            last_input: Instant::now(),
            current_focus: Some(PanelId::Queue),
            help_visible: false,
        };

        (handler, event_tx)
    }

    /// Poll for events with timeout
    pub async fn next_event(&mut self, timeout: Duration) -> Option<AppEvent> {
        // Check for background events first
        if let Ok(event) = self.event_rx.try_recv() {
            return Some(event);
        }

        // Poll for terminal events
        if event::poll(timeout).unwrap_or(false) {
            match event::read() {
                Ok(Event::Key(key)) => {
                    self.last_input = Instant::now();
                    Some(self.handle_key_event(key))
                }
                Ok(Event::Mouse(mouse)) => {
                    self.last_input = Instant::now();
                    Some(AppEvent::MouseEvent(mouse))
                }
                Ok(Event::Resize(width, height)) => {
                    Some(AppEvent::Resize(width, height))
                }
                _ => None,
            }
        } else {
            None
        }
    }

    /// Handle keyboard events and convert to application events
    fn handle_key_event(&mut self, key: KeyEvent) -> AppEvent {
        match (key.code, key.modifiers) {
            // Global shortcuts
            (KeyCode::Char('q'), KeyModifiers::NONE) => AppEvent::Quit,
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => AppEvent::Quit,
            (KeyCode::Esc, KeyModifiers::NONE) => {
                if self.help_visible {
                    self.help_visible = false;
                    AppEvent::Refresh
                } else {
                    AppEvent::Quit
                }
            }

            // Control shortcuts
            (KeyCode::Char(' '), KeyModifiers::NONE) => AppEvent::TogglePause,
            (KeyCode::Char('r'), KeyModifiers::NONE) => AppEvent::Refresh,
            (KeyCode::Char('h'), KeyModifiers::NONE) |
            (KeyCode::Char('?'), KeyModifiers::NONE) => {
                self.help_visible = !self.help_visible;
                AppEvent::ShowHelp
            }
            (KeyCode::Char('c'), KeyModifiers::NONE) => AppEvent::ShowConfig,

            // Panel navigation
            (KeyCode::Tab, KeyModifiers::NONE) => {
                self.current_focus = Some(self.next_panel());
                AppEvent::FocusPanel(self.current_focus.unwrap())
            }
            (KeyCode::BackTab, KeyModifiers::SHIFT) => {
                self.current_focus = Some(self.previous_panel());
                AppEvent::FocusPanel(self.current_focus.unwrap())
            }

            // Direct panel access
            (KeyCode::Char('1'), KeyModifiers::NONE) => {
                self.current_focus = Some(PanelId::Queue);
                AppEvent::FocusPanel(PanelId::Queue)
            }
            (KeyCode::Char('2'), KeyModifiers::NONE) => {
                self.current_focus = Some(PanelId::Performance);
                AppEvent::FocusPanel(PanelId::Performance)
            }
            (KeyCode::Char('3'), KeyModifiers::NONE) => {
                self.current_focus = Some(PanelId::Statistics);
                AppEvent::FocusPanel(PanelId::Statistics)
            }
            (KeyCode::Char('4'), KeyModifiers::NONE) => {
                self.current_focus = Some(PanelId::Logs);
                AppEvent::FocusPanel(PanelId::Logs)
            }

            // Search/filter
            (KeyCode::Char('/'), KeyModifiers::NONE) => {
                AppEvent::Search(String::new())
            }

            // Pass through other key events
            _ => AppEvent::KeyPress(key),
        }
    }

    /// Get next panel in focus order
    fn next_panel(&self) -> PanelId {
        match self.current_focus {
            Some(PanelId::Queue) => PanelId::Performance,
            Some(PanelId::Performance) => PanelId::Statistics,
            Some(PanelId::Statistics) => PanelId::Logs,
            Some(PanelId::Logs) => PanelId::Queue,
            _ => PanelId::Queue,
        }
    }

    /// Get previous panel in focus order
    fn previous_panel(&self) -> PanelId {
        match self.current_focus {
            Some(PanelId::Queue) => PanelId::Logs,
            Some(PanelId::Performance) => PanelId::Queue,
            Some(PanelId::Statistics) => PanelId::Performance,
            Some(PanelId::Logs) => PanelId::Statistics,
            _ => PanelId::Queue,
        }
    }

    /// Get current focused panel
    pub fn current_focus(&self) -> Option<PanelId> {
        self.current_focus
    }

    /// Check if help is currently visible
    pub fn is_help_visible(&self) -> bool {
        self.help_visible
    }

    /// Send event to the application
    pub fn send_event(&self, event: AppEvent) -> Result<(), mpsc::error::SendError<AppEvent>> {
        self.event_tx.send(event)
    }

    /// Get time since last user input
    pub fn time_since_last_input(&self) -> Duration {
        self.last_input.elapsed()
    }
}

/// Helper for creating update events from background tasks
pub struct UpdateSender {
    sender: mpsc::UnboundedSender<AppEvent>,
}

impl UpdateSender {
    pub fn new(sender: mpsc::UnboundedSender<AppEvent>) -> Self {
        Self { sender }
    }

    /// Send progress update
    pub fn send_progress(&self, current: usize, total: usize) -> Result<(), mpsc::error::SendError<AppEvent>> {
        self.sender.send(AppEvent::AppUpdate(UpdateType::Progress { current, total }))
    }

    /// Send song started notification
    pub fn send_song_started(&self, song: String, artist: String) -> Result<(), mpsc::error::SendError<AppEvent>> {
        self.sender.send(AppEvent::AppUpdate(UpdateType::SongStarted { song, artist }))
    }

    /// Send song completed notification
    pub fn send_song_completed(&self, song: String, success: bool, message: Option<String>) -> Result<(), mpsc::error::SendError<AppEvent>> {
        self.sender.send(AppEvent::AppUpdate(UpdateType::SongCompleted { song, success, message }))
    }

    /// Send statistics update
    pub fn send_stats_update(&self, songs_per_min: f64, success_rate: f64) -> Result<(), mpsc::error::SendError<AppEvent>> {
        self.sender.send(AppEvent::AppUpdate(UpdateType::StatsUpdate { songs_per_min, success_rate }))
    }

    /// Send error notification
    pub fn send_error(&self, message: String) -> Result<(), mpsc::error::SendError<AppEvent>> {
        self.sender.send(AppEvent::AppUpdate(UpdateType::Error { message }))
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new().0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_navigation() {
        let (mut handler, _) = EventHandler::new();

        // Test forward navigation
        handler.current_focus = Some(PanelId::Queue);
        assert_eq!(handler.next_panel(), PanelId::Performance);

        handler.current_focus = Some(PanelId::Logs);
        assert_eq!(handler.next_panel(), PanelId::Queue);

        // Test backward navigation
        handler.current_focus = Some(PanelId::Performance);
        assert_eq!(handler.previous_panel(), PanelId::Queue);

        handler.current_focus = Some(PanelId::Queue);
        assert_eq!(handler.previous_panel(), PanelId::Logs);
    }

    #[test]
    fn test_key_event_handling() {
        let (mut handler, _) = EventHandler::new();

        // Test quit key
        let quit_event = handler.handle_key_event(KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });

        assert!(matches!(quit_event, AppEvent::Quit));
    }

    #[test]
    fn test_update_sender() {
        let (_, sender) = EventHandler::new();
        let update_sender = UpdateSender::new(sender);

        // Test that sending doesn't panic
        let _ = update_sender.send_progress(10, 100);
        let _ = update_sender.send_song_started("Test Song".to_string(), "Test Artist".to_string());
    }
}