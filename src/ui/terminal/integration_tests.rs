//! Integration tests for the terminal UI system

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{
        backend::TestBackend,
        buffer::Buffer,
        layout::{Constraint, Direction, Layout, Rect},
        Terminal,
    };

    use crate::ui::terminal::{
        app_with_focus::FocusApp,
        focus::{FocusManager, FocusDirection},
        widgets::{
            InputField, SearchBox, Table, TableRow, Widget,
        },
    };

    /// Test basic widget creation and rendering
    #[test]
    fn test_widget_creation_and_rendering() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // Create widgets
        let search_box = SearchBox::new().with_id("search");
        let mut input_field = InputField::new()
            .with_id("input")
            .placeholder("Enter text...");
        let table = Table::new(vec!["ID".to_string(), "Name".to_string()])
            .with_id("table")
            .rows(vec![
                TableRow::new(vec!["1".to_string(), "Track 1".to_string()]),
            ]);

        // Test rendering
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(0),
                ])
                .split(frame.size());

            search_box.render(chunks[0], frame.buffer_mut());
            input_field.render(chunks[1], frame.buffer_mut());
            table.render(chunks[2], frame.buffer_mut());
        }).unwrap();

        // Verify widget can receive input
        let key_event = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
        assert!(input_field.handle_input(&key_event));
        assert_eq!(input_field.get_value(), "h");
    }

    /// Test focus management system
    #[test]
    fn test_focus_management_integration() {
        let mut focus_manager = FocusManager::new();

        // Add widgets to focus manager
        focus_manager.add_widget("search");
        focus_manager.add_widget("input");
        focus_manager.add_widget("table");

        // Test initial state
        assert_eq!(focus_manager.widget_count(), 3);
        assert!(focus_manager.focused_widget().is_none());

        // Focus first widget
        assert!(focus_manager.focus_first());
        assert_eq!(focus_manager.focused_widget().unwrap().as_str(), "search");

        // Test navigation
        assert!(focus_manager.focus_next());
        assert_eq!(focus_manager.focused_widget().unwrap().as_str(), "input");

        assert!(focus_manager.focus_next());
        assert_eq!(focus_manager.focused_widget().unwrap().as_str(), "table");

        // Test wrapping
        assert!(focus_manager.focus_next());
        assert_eq!(focus_manager.focused_widget().unwrap().as_str(), "search");

        // Test reverse navigation
        assert!(focus_manager.focus_previous());
        assert_eq!(focus_manager.focused_widget().unwrap().as_str(), "table");
    }

    /// Test widget focus state updates
    #[test]
    fn test_widget_focus_state_updates() {
        let mut focus_manager = FocusManager::new();
        let mut search_box = SearchBox::new().with_id("search");
        let mut input_field = InputField::new().with_id("input");

        focus_manager.add_widget("search");
        focus_manager.add_widget("input");

        // Focus search box
        focus_manager.focus_widget("search");

        // Only search box should be focused
        assert!(focus_manager.is_focused("search"));
        assert!(!focus_manager.is_focused("input"));

        // Switch focus to input field
        focus_manager.focus_widget("input");

        // Only input field should be focused
        assert!(!focus_manager.is_focused("search"));
        assert!(focus_manager.is_focused("input"));

        // Update widget focus states manually
        search_box.set_focus(focus_manager.is_focused("search"));
        input_field.set_focus(focus_manager.is_focused("input"));
    }

    /// Test key navigation handling
    #[test]
    fn test_key_navigation_handling() {
        let mut focus_manager = FocusManager::new();
        focus_manager.add_widget("widget1");
        focus_manager.add_widget("widget2");
        focus_manager.add_widget("widget3");

        focus_manager.focus_first();

        // Test Tab navigation
        let tab_event = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        assert!(focus_manager.handle_navigation_key(&tab_event));
        assert_eq!(focus_manager.focused_widget().unwrap().as_str(), "widget2");

        // Test Shift+Tab navigation
        let shift_tab_event = KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT);
        assert!(focus_manager.handle_navigation_key(&shift_tab_event));
        assert_eq!(focus_manager.focused_widget().unwrap().as_str(), "widget1");

        // Test arrow key navigation
        let down_event = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        assert!(focus_manager.handle_navigation_key(&down_event));
        assert_eq!(focus_manager.focused_widget().unwrap().as_str(), "widget2");

        let up_event = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert!(focus_manager.handle_navigation_key(&up_event));
        assert_eq!(focus_manager.focused_widget().unwrap().as_str(), "widget1");
    }

    /// Test table widget navigation and selection
    #[test]
    fn test_table_widget_integration() {
        let mut table = Table::new(vec!["ID".to_string(), "Name".to_string(), "Status".to_string()])
            .with_id("test_table")
            .rows(vec![
                TableRow::new(vec!["1".to_string(), "Track 1".to_string(), "Downloaded".to_string()]),
                TableRow::new(vec!["2".to_string(), "Track 2".to_string(), "Pending".to_string()]),
                TableRow::new(vec!["3".to_string(), "Track 3".to_string(), "Failed".to_string()]),
            ]);

        // Test initial state
        assert!(table.can_focus());
        assert_eq!(table.widget_id(), "test_table");

        // Test navigation within table
        let down_event = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        assert!(table.handle_input(&down_event));

        let up_event = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert!(table.handle_input(&up_event));

        // Test page navigation
        let page_down_event = KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE);
        assert!(table.handle_input(&page_down_event));

        let home_event = KeyEvent::new(KeyCode::Home, KeyModifiers::NONE);
        assert!(table.handle_input(&home_event));
    }

    /// Test input field functionality
    #[test]
    fn test_input_field_integration() {
        let mut input_field = InputField::new()
            .with_id("test_input")
            .placeholder("Enter search term")
            .max_length(50);

        // Test basic properties
        assert!(input_field.can_focus());
        assert_eq!(input_field.widget_id(), "test_input");
        assert_eq!(input_field.get_value(), "");

        // Test text input
        let chars = ['h', 'e', 'l', 'l', 'o'];
        for ch in chars {
            let key_event = KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE);
            assert!(input_field.handle_input(&key_event));
        }
        assert_eq!(input_field.get_value(), "hello");

        // Test backspace
        let backspace_event = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        assert!(input_field.handle_input(&backspace_event));
        assert_eq!(input_field.get_value(), "hell");

        // Test cursor navigation
        let home_event = KeyEvent::new(KeyCode::Home, KeyModifiers::NONE);
        assert!(input_field.handle_input(&home_event));

        let end_event = KeyEvent::new(KeyCode::End, KeyModifiers::NONE);
        assert!(input_field.handle_input(&end_event));

        // Test clear (Ctrl+U)
        let clear_event = KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL);
        assert!(input_field.handle_input(&clear_event));
        assert_eq!(input_field.get_value(), "");
    }

    /// Test search box functionality
    #[test]
    fn test_search_box_integration() {
        let mut search_box = SearchBox::new()
            .with_id("test_search")
            .placeholder("Search tracks...");

        // Test basic properties
        assert!(search_box.can_focus());
        assert_eq!(search_box.widget_id(), "test_search");
        assert!(search_box.is_empty());

        // Test text input
        let search_term = "queen";
        for ch in search_term.chars() {
            let key_event = KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE);
            assert!(search_box.handle_input(&key_event));
        }
        assert_eq!(search_box.get_query(), "queen");
        assert!(!search_box.is_empty());

        // Test clear
        search_box.clear();
        assert!(search_box.is_empty());
    }

    /// Test complete application focus integration
    #[test]
    fn test_focus_app_integration() {
        let mut app = FocusApp::new();

        // Test initial state
        assert_eq!(app.focus_manager.widget_count(), 3);
        assert!(app.focus_manager.focused_widget().is_some());

        // Test navigation between widgets
        let tab_event = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);

        // Navigate through all widgets
        let initial_focus = app.focus_manager.focused_widget().unwrap().as_str().to_string();

        assert!(app.handle_key_event(tab_event));
        let second_focus = app.focus_manager.focused_widget().unwrap().as_str().to_string();
        assert_ne!(initial_focus, second_focus);

        assert!(app.handle_key_event(tab_event));
        let third_focus = app.focus_manager.focused_widget().unwrap().as_str().to_string();
        assert_ne!(second_focus, third_focus);

        // Test wrapping back to first
        assert!(app.handle_key_event(tab_event));
        let wrapped_focus = app.focus_manager.focused_widget().unwrap().as_str().to_string();
        assert_eq!(initial_focus, wrapped_focus);

        // Test quit functionality
        assert!(!app.should_quit());
        let quit_event = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        app.handle_key_event(quit_event);
        assert!(app.should_quit());
    }

    /// Test dynamic widget management
    #[test]
    fn test_dynamic_widget_management() {
        let mut app = FocusApp::new();
        let initial_count = app.focus_manager.widget_count();

        // Add a dynamic widget
        app.add_dynamic_widget("dynamic_widget");
        assert_eq!(app.focus_manager.widget_count(), initial_count + 1);

        // Focus the new widget
        assert!(app.focus_widget("dynamic_widget"));
        assert_eq!(app.focus_manager.focused_widget().unwrap().as_str(), "dynamic_widget");

        // Remove the widget
        app.remove_dynamic_widget("dynamic_widget");
        assert_eq!(app.focus_manager.widget_count(), initial_count);

        // Focus should have moved to another widget
        assert_ne!(app.focus_manager.focused_widget().unwrap().as_str(), "dynamic_widget");
    }

    /// Test custom focus movements
    #[test]
    fn test_custom_focus_movements() {
        let mut app = FocusApp::new();

        // Test directional focus movements
        assert!(app.move_focus_custom(FocusDirection::Next));
        let focused_after_next = app.focus_manager.focused_widget().unwrap().as_str().to_string();

        assert!(app.move_focus_custom(FocusDirection::Previous));
        let focused_after_previous = app.focus_manager.focused_widget().unwrap().as_str().to_string();

        assert_ne!(focused_after_next, focused_after_previous);

        // Test directional movements
        assert!(app.move_focus_custom(FocusDirection::Down));
        assert!(app.move_focus_custom(FocusDirection::Up));
        assert!(app.move_focus_custom(FocusDirection::Right));
        assert!(app.move_focus_custom(FocusDirection::Left));
    }

    /// Test rendering with focus states
    #[test]
    fn test_rendering_with_focus_states() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = FocusApp::new();

        // Test that rendering works without errors
        terminal.draw(|frame| {
            app.render(frame);
        }).unwrap();

        // Change focus and render again
        let tab_event = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        app.handle_key_event(tab_event);

        terminal.draw(|frame| {
            app.render(frame);
        }).unwrap();

        // Verify status information is available
        let status = app.get_status();
        assert!(status.contains("Focused:"));
        assert!(status.contains("Widgets:"));
    }

    /// Test widget state transitions
    #[test]
    fn test_widget_state_transitions() {
        let mut input_field = InputField::new().with_id("state_test");

        // Test initial state
        input_field.set_focus(false);
        // Note: We can't directly access internal state, but we can test behavior

        // Test focus state
        input_field.set_focus(true);
        // Focused widgets should handle input differently

        // Test input handling when focused vs not focused
        let key_event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);

        input_field.set_focus(true);
        assert!(input_field.handle_input(&key_event));

        // Should still handle input even when not focused (for this widget type)
        input_field.set_focus(false);
        let key_event2 = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE);
        assert!(input_field.handle_input(&key_event2));
    }

    /// Test error handling and edge cases
    #[test]
    fn test_error_handling_edge_cases() {
        let mut focus_manager = FocusManager::new();

        // Test with no widgets
        assert!(!focus_manager.focus_next());
        assert!(!focus_manager.focus_previous());
        assert!(focus_manager.focused_widget().is_none());

        // Test focusing non-existent widget
        assert!(!focus_manager.focus_widget("nonexistent"));

        // Test removing non-existent widget
        assert!(!focus_manager.remove_widget("nonexistent"));

        // Test with disabled focus
        focus_manager.add_widget("test_widget");
        focus_manager.set_enabled(false);

        let tab_event = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        assert!(!focus_manager.handle_navigation_key(&tab_event));
        assert!(focus_manager.focused_widget().is_none());

        // Re-enable and test
        focus_manager.set_enabled(true);
        assert!(focus_manager.handle_navigation_key(&tab_event));
    }
}