//! Example application using the focus management system

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use super::{
    focus::{FocusManager, FocusDirection},
    widgets::{InputField, SearchBox, Table, TableRow, Widget, WidgetState},
};

/// Example application demonstrating focus management
pub struct FocusApp {
    pub focus_manager: FocusManager,
    pub search_box: SearchBox,
    pub input_field: InputField,
    pub table: Table,
    should_quit: bool,
}

impl FocusApp {
    pub fn new() -> Self {
        let mut focus_manager = FocusManager::new();

        // Create widgets with unique IDs
        let search_box = SearchBox::new().with_id("search");
        let input_field = InputField::new()
            .with_id("input")
            .placeholder("Enter text...");
        let table = Table::new(vec!["ID".to_string(), "Name".to_string(), "Status".to_string()])
            .with_id("table")
            .rows(vec![
                TableRow::new(vec!["1".to_string(), "Track 1".to_string(), "Downloaded".to_string()]),
                TableRow::new(vec!["2".to_string(), "Track 2".to_string(), "Pending".to_string()]),
                TableRow::new(vec!["3".to_string(), "Track 3".to_string(), "Failed".to_string()]),
            ]);

        // Register widgets with focus manager
        focus_manager.add_widget("search");
        focus_manager.add_widget("input");
        focus_manager.add_widget("table");

        // Focus the first widget
        focus_manager.focus_first();

        Self {
            focus_manager,
            search_box,
            input_field,
            table,
            should_quit: false,
        }
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn handle_event(&mut self, event: Event) -> bool {
        match event {
            Event::Key(key_event) => self.handle_key_event(key_event),
            _ => false,
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> bool {
        // Handle global keys first
        match (key_event.code, key_event.modifiers) {
            (KeyCode::Char('q'), KeyModifiers::NONE) | (KeyCode::Esc, KeyModifiers::NONE) => {
                self.should_quit = true;
                return true;
            }
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.should_quit = true;
                return true;
            }
            _ => {}
        }

        // Try to handle navigation first
        if self.focus_manager.handle_navigation_key(&key_event) {
            self.update_widget_focus();
            return true;
        }

        // If navigation didn't handle it, pass to focused widget
        if let Some(focused_widget_id) = self.focus_manager.focused_widget() {
            let handled = match focused_widget_id.as_str() {
                "search" => self.search_box.handle_input(&key_event),
                "input" => self.input_field.handle_input(&key_event),
                "table" => self.table.handle_input(&key_event),
                _ => false,
            };

            if handled {
                return true;
            }
        }

        false
    }

    pub fn render(&mut self, frame: &mut Frame<'_>) {
        self.update_widget_focus();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search box
                Constraint::Length(3), // Input field
                Constraint::Min(0),    // Table
            ])
            .split(frame.size());

        // Render widgets
        self.search_box.render(chunks[0], frame.buffer_mut());
        self.input_field.render(chunks[1], frame.buffer_mut());
        self.table.render(chunks[2], frame.buffer_mut());
    }

    fn update_widget_focus(&mut self) {
        // Update focus state for all widgets
        let search_focused = self.focus_manager.is_focused("search");
        let input_focused = self.focus_manager.is_focused("input");
        let table_focused = self.focus_manager.is_focused("table");

        self.search_box.set_focus(search_focused);
        self.input_field.set_focus(input_focused);
        self.table.set_focus(table_focused);
    }

    /// Get status information for display
    pub fn get_status(&self) -> String {
        let focused = self.focus_manager.focused_widget()
            .map(|w| w.as_str())
            .unwrap_or("none");

        format!(
            "Focused: {} | Widgets: {} | Use Tab/Shift+Tab to navigate, q to quit",
            focused,
            self.focus_manager.widget_count()
        )
    }

    /// Example of adding a widget dynamically
    pub fn add_dynamic_widget(&mut self, id: &str) {
        self.focus_manager.add_widget(id);
    }

    /// Example of removing a widget dynamically
    pub fn remove_dynamic_widget(&mut self, id: &str) {
        self.focus_manager.remove_widget(id);
        self.update_widget_focus();
    }

    /// Example of programmatically focusing a specific widget
    pub fn focus_widget(&mut self, id: &str) -> bool {
        let result = self.focus_manager.focus_widget(id);
        if result {
            self.update_widget_focus();
        }
        result
    }

    /// Example of handling custom focus movements
    pub fn move_focus_custom(&mut self, direction: FocusDirection) -> bool {
        let result = self.focus_manager.move_focus(direction);
        if result {
            self.update_widget_focus();
        }
        result
    }
}

impl Default for FocusApp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;

    #[test]
    fn test_focus_app_creation() {
        let app = FocusApp::new();
        assert_eq!(app.focus_manager.widget_count(), 3);
        assert!(app.focus_manager.focused_widget().is_some());
    }

    #[test]
    fn test_focus_navigation() {
        let mut app = FocusApp::new();

        // Should start with search focused
        assert_eq!(app.focus_manager.focused_widget().unwrap().as_str(), "search");

        // Navigate to next widget
        let tab_event = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        assert!(app.handle_key_event(tab_event));
        assert_eq!(app.focus_manager.focused_widget().unwrap().as_str(), "input");

        // Navigate to next widget
        assert!(app.handle_key_event(tab_event));
        assert_eq!(app.focus_manager.focused_widget().unwrap().as_str(), "table");
    }

    #[test]
    fn test_quit_functionality() {
        let mut app = FocusApp::new();
        assert!(!app.should_quit());

        let quit_event = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        app.handle_key_event(quit_event);
        assert!(app.should_quit());
    }

    #[test]
    fn test_dynamic_widget_management() {
        let mut app = FocusApp::new();
        let initial_count = app.focus_manager.widget_count();

        app.add_dynamic_widget("new_widget");
        assert_eq!(app.focus_manager.widget_count(), initial_count + 1);

        app.remove_dynamic_widget("new_widget");
        assert_eq!(app.focus_manager.widget_count(), initial_count);
    }

    #[test]
    fn test_programmatic_focus() {
        let mut app = FocusApp::new();

        assert!(app.focus_widget("table"));
        assert_eq!(app.focus_manager.focused_widget().unwrap().as_str(), "table");

        assert!(!app.focus_widget("nonexistent"));
    }
}