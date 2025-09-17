//! Input handling for terminal UI
//!
//! This module provides centralized input event handling for the terminal UI,
//! including queue navigation, filtering, and contextual actions.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::collections::VecDeque;

use crate::ui::terminal::{
    state::{TrackQueueItem, TrackStatus},
};

/// Scroll direction for queue navigation
#[derive(Debug, Clone, Copy)]
pub enum ScrollDirection {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
}

/// Input context for determining which UI component should handle input
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputContext {
    /// Main queue navigation
    Queue,
    /// Filter input mode
    Filter,
    /// Help screen
    Help,
    /// Configuration mode
    Config,
}

/// Actions that can be triggered by user input
#[derive(Debug, Clone)]
pub enum InputAction {
    /// Navigate in the queue
    Navigate(ScrollDirection),
    /// Toggle filter input mode
    ToggleFilter,
    /// Update filter text
    UpdateFilter(String),
    /// Clear filter
    ClearFilter,
    /// Retry selected track download
    RetryTrack(u64),
    /// Skip selected track
    SkipTrack(u64),
    /// Show track details
    ShowDetails(u64),
    /// Remove track from queue
    RemoveTrack(u64),
    /// Pause/resume downloads
    TogglePause,
    /// Show help
    ShowHelp,
    /// Exit application
    Exit,
    /// No action
    None,
}

/// Queue-specific input handler
#[derive(Debug)]
pub struct QueueInputHandler {
    /// Current input context
    pub context: InputContext,
    /// Current filter text when in filter mode
    pub filter_buffer: String,
    /// Whether the queue has focus
    pub queue_focused: bool,
}

impl QueueInputHandler {
    /// Create a new queue input handler
    pub fn new() -> Self {
        Self {
            context: InputContext::Queue,
            filter_buffer: String::new(),
            queue_focused: true,
        }
    }

    /// Handle input event and return the appropriate action
    pub fn handle_input(&mut self, event: &Event) -> InputAction {
        match event {
            Event::Key(key_event) => self.handle_key_event(key_event),
            _ => InputAction::None,
        }
    }

    /// Handle keyboard input based on current context
    fn handle_key_event(&mut self, key: &KeyEvent) -> InputAction {
        match self.context {
            InputContext::Queue => self.handle_queue_navigation(key),
            InputContext::Filter => self.handle_filter_input(key),
            InputContext::Help => self.handle_help_input(key),
            InputContext::Config => self.handle_config_input(key),
        }
    }

    /// Handle input in queue navigation context
    fn handle_queue_navigation(&mut self, key: &KeyEvent) -> InputAction {
        match key.code {
            // Navigation
            KeyCode::Up | KeyCode::Char('k') => InputAction::Navigate(ScrollDirection::Up),
            KeyCode::Down | KeyCode::Char('j') => InputAction::Navigate(ScrollDirection::Down),
            KeyCode::PageUp => InputAction::Navigate(ScrollDirection::PageUp),
            KeyCode::PageDown => InputAction::Navigate(ScrollDirection::PageDown),
            KeyCode::Home | KeyCode::Char('g') => InputAction::Navigate(ScrollDirection::Home),
            KeyCode::End | KeyCode::Char('G') => InputAction::Navigate(ScrollDirection::End),

            // Search/Filter
            KeyCode::Char('/') => {
                self.context = InputContext::Filter;
                self.filter_buffer.clear();
                InputAction::ToggleFilter
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                InputAction::ClearFilter
            }

            // Track actions
            KeyCode::Char('r') => {
                // Retry selected track - we need the track ID
                InputAction::None // Will be handled by the panel with actual track ID
            }
            KeyCode::Char('s') => {
                // Skip selected track
                InputAction::None // Will be handled by the panel with actual track ID
            }
            KeyCode::Char('d') => {
                // Show details for selected track
                InputAction::None // Will be handled by the panel with actual track ID
            }
            KeyCode::Delete | KeyCode::Char('x') => {
                // Remove track from queue
                InputAction::None // Will be handled by the panel with actual track ID
            }

            // Global actions
            KeyCode::Char('p') | KeyCode::Char(' ') => InputAction::TogglePause,
            KeyCode::Char('h') | KeyCode::F(1) => {
                self.context = InputContext::Help;
                InputAction::ShowHelp
            }
            KeyCode::Char('q') | KeyCode::Esc => InputAction::Exit,

            _ => InputAction::None,
        }
    }

    /// Handle input in filter context
    fn handle_filter_input(&mut self, key: &KeyEvent) -> InputAction {
        match key.code {
            // Exit filter mode
            KeyCode::Esc | KeyCode::Enter => {
                self.context = InputContext::Queue;
                let filter = if self.filter_buffer.is_empty() {
                    InputAction::ClearFilter
                } else {
                    InputAction::UpdateFilter(self.filter_buffer.clone())
                };
                self.filter_buffer.clear();
                filter
            }

            // Clear filter
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.filter_buffer.clear();
                InputAction::ClearFilter
            }

            // Edit filter text
            KeyCode::Char(c) => {
                self.filter_buffer.push(c);
                InputAction::UpdateFilter(self.filter_buffer.clone())
            }
            KeyCode::Backspace => {
                self.filter_buffer.pop();
                InputAction::UpdateFilter(self.filter_buffer.clone())
            }

            _ => InputAction::None,
        }
    }

    /// Handle input in help context
    fn handle_help_input(&mut self, key: &KeyEvent) -> InputAction {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('h') => {
                self.context = InputContext::Queue;
                InputAction::None
            }
            _ => InputAction::None,
        }
    }

    /// Handle input in configuration context
    fn handle_config_input(&mut self, key: &KeyEvent) -> InputAction {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.context = InputContext::Queue;
                InputAction::None
            }
            _ => InputAction::None,
        }
    }

    /// Set input context
    pub fn set_context(&mut self, context: InputContext) {
        self.context = context;
    }

    /// Get current context
    pub fn get_context(&self) -> InputContext {
        self.context
    }

    /// Check if currently in filter mode
    pub fn is_filtering(&self) -> bool {
        self.context == InputContext::Filter
    }

    /// Get current filter buffer
    pub fn get_filter_buffer(&self) -> &str {
        &self.filter_buffer
    }
}

/// Actions that can be performed on tracks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackAction {
    Retry,
    Skip,
    Remove,
    ShowDetails,
}

impl Default for QueueInputHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::time::SystemTime;

    fn create_test_key_event(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn test_queue_navigation_input() {
        let mut handler = QueueInputHandler::new();

        // Test up navigation
        let action = handler.handle_input(&create_test_key_event(KeyCode::Up));
        matches!(action, InputAction::Navigate(ScrollDirection::Up));

        // Test down navigation
        let action = handler.handle_input(&create_test_key_event(KeyCode::Down));
        matches!(action, InputAction::Navigate(ScrollDirection::Down));

        // Test vim-style navigation
        let action = handler.handle_input(&create_test_key_event(KeyCode::Char('j')));
        matches!(action, InputAction::Navigate(ScrollDirection::Down));
    }

    #[test]
    fn test_filter_mode_toggle() {
        let mut handler = QueueInputHandler::new();

        // Start in queue context
        assert_eq!(handler.get_context(), InputContext::Queue);

        // Press '/' to enter filter mode
        let action = handler.handle_input(&create_test_key_event(KeyCode::Char('/')));
        matches!(action, InputAction::ToggleFilter);
        assert_eq!(handler.get_context(), InputContext::Filter);
    }

    #[test]
    fn test_filter_input() {
        let mut handler = QueueInputHandler::new();
        handler.set_context(InputContext::Filter);

        // Type some characters
        handler.handle_input(&create_test_key_event(KeyCode::Char('t')));
        handler.handle_input(&create_test_key_event(KeyCode::Char('e')));
        handler.handle_input(&create_test_key_event(KeyCode::Char('s')));
        handler.handle_input(&create_test_key_event(KeyCode::Char('t')));

        assert_eq!(handler.get_filter_buffer(), "test");

        // Test backspace
        handler.handle_input(&create_test_key_event(KeyCode::Backspace));
        assert_eq!(handler.get_filter_buffer(), "tes");
    }

    #[test]
    fn test_context_switching() {
        let mut handler = QueueInputHandler::new();

        // Switch to help
        handler.handle_input(&create_test_key_event(KeyCode::Char('h')));
        assert_eq!(handler.get_context(), InputContext::Help);

        // Exit help
        handler.handle_input(&create_test_key_event(KeyCode::Esc));
        assert_eq!(handler.get_context(), InputContext::Queue);
    }

}