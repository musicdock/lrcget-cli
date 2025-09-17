//! Input field widgets for user interaction

use ratatui::{
    layout::Rect,
    style::{Color, Style, Modifier},
    widgets::{Paragraph, Block, Borders, Widget as RatatuiWidget},
    buffer::Buffer,
    text::{Line, Span},
};
use crossterm::event::{KeyEvent, KeyCode, KeyModifiers};

use super::{Widget, WidgetState};

/// An input field widget
#[derive(Debug, Clone)]
pub struct InputField {
    value: String,
    placeholder: String,
    cursor_position: usize,
    style: Style,
    focused_style: Style,
    placeholder_style: Style,
    state: WidgetState,
    max_length: Option<usize>,
    password: bool,
    id: String,
}

impl InputField {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            placeholder: String::new(),
            cursor_position: 0,
            style: Style::default(),
            focused_style: Style::default().fg(Color::Yellow),
            placeholder_style: Style::default().fg(Color::DarkGray),
            state: WidgetState::Normal,
            max_length: None,
            password: false,
            id: "input_field".to_string(),
        }
    }

    pub fn with_id<S: Into<String>>(mut self, id: S) -> Self {
        self.id = id.into();
        self
    }

    pub fn value<S: Into<String>>(mut self, value: S) -> Self {
        self.value = value.into();
        self.cursor_position = self.value.len();
        self
    }

    pub fn placeholder<S: Into<String>>(mut self, placeholder: S) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn focused_style(mut self, style: Style) -> Self {
        self.focused_style = style;
        self
    }

    pub fn max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }

    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    pub fn get_value(&self) -> &str {
        &self.value
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor_position = 0;
    }

    fn insert_char(&mut self, c: char) {
        if let Some(max_len) = self.max_length {
            if self.value.len() >= max_len {
                return;
            }
        }

        self.value.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.value.remove(self.cursor_position);
        }
    }

    fn delete_char_forward(&mut self) {
        if self.cursor_position < self.value.len() {
            self.value.remove(self.cursor_position);
        }
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    fn move_cursor_right(&mut self) {
        if self.cursor_position < self.value.len() {
            self.cursor_position += 1;
        }
    }

    fn move_cursor_home(&mut self) {
        self.cursor_position = 0;
    }

    fn move_cursor_end(&mut self) {
        self.cursor_position = self.value.len();
    }

    fn get_display_value(&self) -> String {
        if self.password {
            "*".repeat(self.value.len())
        } else {
            self.value.clone()
        }
    }

    fn get_display_text(&self) -> Line {
        let display_value = self.get_display_value();

        if display_value.is_empty() && !self.placeholder.is_empty() {
            Line::from(Span::styled(&self.placeholder, self.placeholder_style))
        } else if self.state == WidgetState::Focused {
            // Show cursor when focused
            let before_cursor = display_value[..self.cursor_position].to_string();
            let cursor_char = if self.cursor_position < display_value.len() {
                display_value.chars().nth(self.cursor_position).unwrap()
            } else {
                ' '
            };
            let after_cursor = display_value[self.cursor_position + 1..].to_string();

            Line::from(vec![
                Span::styled(before_cursor, self.style),
                Span::styled(cursor_char.to_string(), self.style.add_modifier(Modifier::REVERSED)),
                Span::styled(after_cursor, self.style),
            ])
        } else {
            Line::from(Span::styled(display_value, self.style))
        }
    }
}

impl Widget for InputField {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let text = self.get_display_text();

        let border_style = match self.state {
            WidgetState::Focused => self.focused_style,
            _ => self.style,
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);

        let paragraph = Paragraph::new(text).block(block);
        RatatuiWidget::render(paragraph, area, buf);
    }

    fn handle_input(&mut self, event: &KeyEvent) -> bool {
        match (event.code, event.modifiers) {
            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                self.insert_char(c);
                true
            }
            (KeyCode::Backspace, KeyModifiers::NONE) => {
                self.delete_char();
                true
            }
            (KeyCode::Delete, KeyModifiers::NONE) => {
                self.delete_char_forward();
                true
            }
            (KeyCode::Left, KeyModifiers::NONE) => {
                self.move_cursor_left();
                true
            }
            (KeyCode::Right, KeyModifiers::NONE) => {
                self.move_cursor_right();
                true
            }
            (KeyCode::Home, KeyModifiers::NONE) => {
                self.move_cursor_home();
                true
            }
            (KeyCode::End, KeyModifiers::NONE) => {
                self.move_cursor_end();
                true
            }
            (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.move_cursor_home();
                true
            }
            (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.move_cursor_end();
                true
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.clear();
                true
            }
            _ => false,
        }
    }

    fn can_focus(&self) -> bool {
        true
    }

    fn set_focus(&mut self, focused: bool) {
        self.state = if focused {
            WidgetState::Focused
        } else {
            WidgetState::Normal
        };
    }

    fn widget_id(&self) -> &str {
        &self.id
    }
}

impl Default for InputField {
    fn default() -> Self {
        Self::new()
    }
}

/// A specialized search box widget
#[derive(Debug, Clone)]
pub struct SearchBox {
    input: InputField,
    prefix: String,
    id: String,
}

impl SearchBox {
    pub fn new() -> Self {
        Self {
            input: InputField::new()
                .placeholder("Search...")
                .style(Style::default()),
            prefix: "🔍 ".to_string(),
            id: "search_box".to_string(),
        }
    }

    pub fn with_id<S: Into<String>>(mut self, id: S) -> Self {
        self.id = id.into();
        self
    }

    pub fn prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.prefix = prefix.into();
        self
    }

    pub fn placeholder<S: Into<String>>(mut self, placeholder: S) -> Self {
        self.input = self.input.placeholder(placeholder);
        self
    }

    pub fn get_query(&self) -> &str {
        self.input.get_value()
    }

    pub fn clear(&mut self) {
        self.input.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.input.get_value().is_empty()
    }
}

impl Widget for SearchBox {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        // Render prefix + input field
        let display_text = if self.input.get_value().is_empty() {
            Line::from(vec![
                Span::styled(&self.prefix, self.input.style),
                Span::styled("Search...", self.input.placeholder_style),
            ])
        } else {
            Line::from(vec![
                Span::styled(&self.prefix, self.input.style),
                Span::styled(self.input.get_value(), self.input.style),
            ])
        };

        let border_style = match self.input.state {
            WidgetState::Focused => self.input.focused_style,
            _ => self.input.style,
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);

        let paragraph = Paragraph::new(display_text).block(block);
        RatatuiWidget::render(paragraph, area, buf);
    }

    fn handle_input(&mut self, event: &KeyEvent) -> bool {
        self.input.handle_input(event)
    }

    fn can_focus(&self) -> bool {
        true
    }

    fn set_focus(&mut self, focused: bool) {
        self.input.set_focus(focused);
    }

    fn widget_id(&self) -> &str {
        &self.id
    }
}

impl Default for SearchBox {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_field_creation() {
        let input = InputField::new()
            .placeholder("Enter text")
            .max_length(50);

        assert_eq!(input.get_value(), "");
        assert_eq!(input.placeholder, "Enter text");
        assert_eq!(input.max_length, Some(50));
    }

    #[test]
    fn test_input_field_typing() {
        let mut input = InputField::new();

        let key_event = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
        assert!(input.handle_input(&key_event));
        assert_eq!(input.get_value(), "h");

        let key_event = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
        assert!(input.handle_input(&key_event));
        assert_eq!(input.get_value(), "hi");
    }

    #[test]
    fn test_input_field_backspace() {
        let mut input = InputField::new().value("hello");

        let key_event = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        assert!(input.handle_input(&key_event));
        assert_eq!(input.get_value(), "hell");
    }

    #[test]
    fn test_input_field_navigation() {
        let mut input = InputField::new().value("hello");

        let key_event = KeyEvent::new(KeyCode::Home, KeyModifiers::NONE);
        assert!(input.handle_input(&key_event));
        assert_eq!(input.cursor_position, 0);

        let key_event = KeyEvent::new(KeyCode::End, KeyModifiers::NONE);
        assert!(input.handle_input(&key_event));
        assert_eq!(input.cursor_position, 5);
    }

    #[test]
    fn test_search_box() {
        let mut search = SearchBox::new();
        assert!(search.is_empty());

        let key_event = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE);
        assert!(search.handle_input(&key_event));
        assert_eq!(search.get_query(), "t");
        assert!(!search.is_empty());

        search.clear();
        assert!(search.is_empty());
    }

    #[test]
    fn test_password_input() {
        let input = InputField::new()
            .value("secret")
            .password(true);

        assert_eq!(input.get_display_value(), "******");
    }

    #[test]
    fn test_max_length() {
        let mut input = InputField::new().max_length(3);

        let key_event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert!(input.handle_input(&key_event));
        assert!(input.handle_input(&key_event));
        assert!(input.handle_input(&key_event));
        assert_eq!(input.get_value(), "aaa");

        // Should not accept more characters
        assert!(input.handle_input(&key_event));
        assert_eq!(input.get_value(), "aaa");
    }
}