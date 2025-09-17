//! Focus management system for TUI widgets

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::VecDeque;

/// Trait for widgets that can be focused
pub trait Focusable {
    /// Returns true if this widget can receive focus
    fn can_focus(&self) -> bool;

    /// Set focus state for this widget
    fn set_focus(&mut self, focused: bool);

    /// Handle input event when this widget is focused
    /// Returns true if the event was handled
    fn handle_input(&mut self, event: &KeyEvent) -> bool;

    /// Get a unique identifier for this widget
    fn widget_id(&self) -> &str;

    /// Returns true if this widget wants to handle navigation keys itself
    fn handles_navigation(&self) -> bool {
        false
    }
}

/// Widget identifier for focus management
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WidgetId(String);

impl WidgetId {
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for WidgetId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for WidgetId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Focus direction for navigation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusDirection {
    Next,
    Previous,
    Up,
    Down,
    Left,
    Right,
}

/// Focus manager for handling widget focus and navigation
#[derive(Debug)]
pub struct FocusManager {
    /// List of focusable widget IDs in order
    focus_order: VecDeque<WidgetId>,
    /// Currently focused widget index
    current_focus: Option<usize>,
    /// Whether focus wrapping is enabled (go to first after last)
    wrap_focus: bool,
    /// Whether focus is enabled globally
    focus_enabled: bool,
}

impl FocusManager {
    /// Create a new focus manager
    pub fn new() -> Self {
        Self {
            focus_order: VecDeque::new(),
            current_focus: None,
            wrap_focus: true,
            focus_enabled: true,
        }
    }

    /// Add a widget to the focus order
    pub fn add_widget<T: Into<WidgetId>>(&mut self, widget_id: T) {
        let id = widget_id.into();
        if !self.focus_order.contains(&id) {
            self.focus_order.push_back(id);
        }
    }

    /// Remove a widget from the focus order
    pub fn remove_widget<T: Into<WidgetId>>(&mut self, widget_id: T) -> bool {
        let id = widget_id.into();
        if let Some(pos) = self.focus_order.iter().position(|x| x == &id) {
            self.focus_order.remove(pos);

            // Adjust current focus if necessary
            if let Some(current) = self.current_focus {
                if current == pos {
                    // Currently focused widget was removed
                    self.current_focus = if self.focus_order.is_empty() {
                        None
                    } else if current >= self.focus_order.len() {
                        Some(self.focus_order.len() - 1)
                    } else {
                        Some(current)
                    };
                } else if current > pos {
                    // Adjust index for widgets after the removed one
                    self.current_focus = Some(current - 1);
                }
            }
            true
        } else {
            false
        }
    }

    /// Clear all widgets from focus order
    pub fn clear(&mut self) {
        self.focus_order.clear();
        self.current_focus = None;
    }

    /// Get the currently focused widget ID
    pub fn focused_widget(&self) -> Option<&WidgetId> {
        self.current_focus.and_then(|idx| self.focus_order.get(idx))
    }

    /// Check if a specific widget is focused
    pub fn is_focused<T: Into<WidgetId>>(&self, widget_id: T) -> bool {
        let id = widget_id.into();
        self.focused_widget().map_or(false, |focused| focused == &id)
    }

    /// Set focus to a specific widget
    pub fn focus_widget<T: Into<WidgetId>>(&mut self, widget_id: T) -> bool {
        let id = widget_id.into();
        if let Some(pos) = self.focus_order.iter().position(|x| x == &id) {
            self.current_focus = Some(pos);
            true
        } else {
            false
        }
    }

    /// Move focus in the specified direction
    pub fn move_focus(&mut self, direction: FocusDirection) -> bool {
        if !self.focus_enabled || self.focus_order.is_empty() {
            return false;
        }

        let new_index = match direction {
            FocusDirection::Next | FocusDirection::Down | FocusDirection::Right => {
                self.next_focus_index()
            }
            FocusDirection::Previous | FocusDirection::Up | FocusDirection::Left => {
                self.previous_focus_index()
            }
        };

        if let Some(index) = new_index {
            self.current_focus = Some(index);
            true
        } else {
            false
        }
    }

    /// Move to next focusable widget
    pub fn focus_next(&mut self) -> bool {
        self.move_focus(FocusDirection::Next)
    }

    /// Move to previous focusable widget
    pub fn focus_previous(&mut self) -> bool {
        self.move_focus(FocusDirection::Previous)
    }

    /// Set the first widget as focused if none is focused
    pub fn focus_first(&mut self) -> bool {
        if self.focus_enabled && !self.focus_order.is_empty() && self.current_focus.is_none() {
            self.current_focus = Some(0);
            true
        } else {
            false
        }
    }

    /// Remove focus from all widgets
    pub fn blur_all(&mut self) {
        self.current_focus = None;
    }

    /// Enable or disable focus management
    pub fn set_enabled(&mut self, enabled: bool) {
        self.focus_enabled = enabled;
        if !enabled {
            self.current_focus = None;
        }
    }

    /// Check if focus management is enabled
    pub fn is_enabled(&self) -> bool {
        self.focus_enabled
    }

    /// Enable or disable focus wrapping
    pub fn set_wrap_focus(&mut self, wrap: bool) {
        self.wrap_focus = wrap;
    }

    /// Handle a key event for focus navigation
    /// Returns true if the event was handled by focus management
    pub fn handle_navigation_key(&mut self, event: &KeyEvent) -> bool {
        if !self.focus_enabled {
            return false;
        }

        match (event.code, event.modifiers) {
            (KeyCode::Tab, KeyModifiers::NONE) => {
                self.focus_next()
            }
            (KeyCode::Tab, KeyModifiers::SHIFT) | (KeyCode::BackTab, KeyModifiers::NONE) => {
                self.focus_previous()
            }
            (KeyCode::Up, KeyModifiers::NONE) => {
                self.move_focus(FocusDirection::Up)
            }
            (KeyCode::Down, KeyModifiers::NONE) => {
                self.move_focus(FocusDirection::Down)
            }
            (KeyCode::Left, KeyModifiers::NONE) => {
                self.move_focus(FocusDirection::Left)
            }
            (KeyCode::Right, KeyModifiers::NONE) => {
                self.move_focus(FocusDirection::Right)
            }
            _ => false
        }
    }

    /// Get number of focusable widgets
    pub fn widget_count(&self) -> usize {
        self.focus_order.len()
    }

    /// Get all widget IDs in focus order
    pub fn widget_ids(&self) -> impl Iterator<Item = &WidgetId> {
        self.focus_order.iter()
    }

    fn next_focus_index(&self) -> Option<usize> {
        if self.focus_order.is_empty() {
            return None;
        }

        match self.current_focus {
            None => Some(0),
            Some(current) => {
                let next = current + 1;
                if next >= self.focus_order.len() {
                    if self.wrap_focus {
                        Some(0)
                    } else {
                        Some(current) // Stay at current position
                    }
                } else {
                    Some(next)
                }
            }
        }
    }

    fn previous_focus_index(&self) -> Option<usize> {
        if self.focus_order.is_empty() {
            return None;
        }

        match self.current_focus {
            None => Some(self.focus_order.len() - 1),
            Some(current) => {
                if current == 0 {
                    if self.wrap_focus {
                        Some(self.focus_order.len() - 1)
                    } else {
                        Some(0) // Stay at current position
                    }
                } else {
                    Some(current - 1)
                }
            }
        }
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper trait to make widgets compatible with focus manager
pub trait FocusManagerExt {
    /// Update focus state for a widget based on focus manager
    fn update_widget_focus<T: Focusable>(&self, widget: &mut T, widget_id: &str);
}

impl FocusManagerExt for FocusManager {
    fn update_widget_focus<T: Focusable>(&self, widget: &mut T, widget_id: &str) {
        let is_focused = self.is_focused(widget_id);
        widget.set_focus(is_focused);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_manager_creation() {
        let manager = FocusManager::new();
        assert_eq!(manager.widget_count(), 0);
        assert!(manager.focused_widget().is_none());
        assert!(manager.is_enabled());
    }

    #[test]
    fn test_add_remove_widgets() {
        let mut manager = FocusManager::new();

        manager.add_widget("widget1");
        manager.add_widget("widget2");
        assert_eq!(manager.widget_count(), 2);

        assert!(manager.remove_widget("widget1"));
        assert_eq!(manager.widget_count(), 1);

        assert!(!manager.remove_widget("nonexistent"));
    }

    #[test]
    fn test_focus_navigation() {
        let mut manager = FocusManager::new();
        manager.add_widget("widget1");
        manager.add_widget("widget2");
        manager.add_widget("widget3");

        // Initially no focus
        assert!(manager.focused_widget().is_none());

        // Focus first widget
        assert!(manager.focus_first());
        assert_eq!(manager.focused_widget().unwrap().as_str(), "widget1");

        // Move to next
        assert!(manager.focus_next());
        assert_eq!(manager.focused_widget().unwrap().as_str(), "widget2");

        // Move to previous
        assert!(manager.focus_previous());
        assert_eq!(manager.focused_widget().unwrap().as_str(), "widget1");
    }

    #[test]
    fn test_focus_wrapping() {
        let mut manager = FocusManager::new();
        manager.add_widget("widget1");
        manager.add_widget("widget2");

        manager.focus_first();

        // Move to last widget and then next (should wrap to first)
        assert!(manager.focus_next());
        assert_eq!(manager.focused_widget().unwrap().as_str(), "widget2");

        assert!(manager.focus_next());
        assert_eq!(manager.focused_widget().unwrap().as_str(), "widget1");

        // Test wrapping disabled
        manager.set_wrap_focus(false);
        manager.focus_widget("widget2");
        assert!(manager.focus_next()); // Should stay at widget2
        assert_eq!(manager.focused_widget().unwrap().as_str(), "widget2");
    }

    #[test]
    fn test_focus_specific_widget() {
        let mut manager = FocusManager::new();
        manager.add_widget("widget1");
        manager.add_widget("widget2");

        assert!(manager.focus_widget("widget2"));
        assert_eq!(manager.focused_widget().unwrap().as_str(), "widget2");

        assert!(!manager.focus_widget("nonexistent"));
    }

    #[test]
    fn test_navigation_keys() {
        let mut manager = FocusManager::new();
        manager.add_widget("widget1");
        manager.add_widget("widget2");

        let tab_event = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        let shift_tab_event = KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT);

        // Tab should focus first widget if none focused
        assert!(manager.handle_navigation_key(&tab_event));
        assert_eq!(manager.focused_widget().unwrap().as_str(), "widget1");

        // Tab should move to next widget
        assert!(manager.handle_navigation_key(&tab_event));
        assert_eq!(manager.focused_widget().unwrap().as_str(), "widget2");

        // Shift+Tab should move to previous widget
        assert!(manager.handle_navigation_key(&shift_tab_event));
        assert_eq!(manager.focused_widget().unwrap().as_str(), "widget1");
    }

    #[test]
    fn test_disabled_focus() {
        let mut manager = FocusManager::new();
        manager.add_widget("widget1");
        manager.set_enabled(false);

        let tab_event = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        assert!(!manager.handle_navigation_key(&tab_event));
        assert!(manager.focused_widget().is_none());
    }
}