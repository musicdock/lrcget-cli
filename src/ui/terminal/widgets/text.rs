//! Text display widgets with styling support

use ratatui::{
    layout::Rect,
    style::{Color, Style, Modifier},
    widgets::{Paragraph, Wrap, Widget as RatatuiWidget},
    buffer::Buffer,
    text::{Line, Span, Text},
};

use super::{Widget, WidgetState};

/// A styled text widget
#[derive(Debug, Clone)]
pub struct StyledText {
    content: String,
    style: Style,
    wrap: bool,
    state: WidgetState,
}

impl StyledText {
    pub fn new<S: Into<String>>(content: S) -> Self {
        Self {
            content: content.into(),
            style: Style::default(),
            wrap: false,
            state: WidgetState::Normal,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn bold(mut self) -> Self {
        self.style = self.style.add_modifier(Modifier::BOLD);
        self
    }

    pub fn italic(mut self) -> Self {
        self.style = self.style.add_modifier(Modifier::ITALIC);
        self
    }

    pub fn underlined(mut self) -> Self {
        self.style = self.style.add_modifier(Modifier::UNDERLINED);
        self
    }

    pub fn fg(mut self, color: Color) -> Self {
        self.style = self.style.fg(color);
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.style = self.style.bg(color);
        self
    }
}

impl Widget for StyledText {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let text = Text::from(Line::from(Span::styled(&self.content, self.style)));

        let paragraph = if self.wrap {
            Paragraph::new(text).wrap(Wrap { trim: true })
        } else {
            Paragraph::new(text)
        };

        RatatuiWidget::render(paragraph, area, buf);
    }
}

/// A text block widget with multiple lines and advanced formatting
#[derive(Debug, Clone)]
pub struct TextBlock {
    lines: Vec<Line<'static>>,
    wrap: bool,
    alignment: ratatui::layout::Alignment,
    state: WidgetState,
}

impl TextBlock {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            wrap: false,
            alignment: ratatui::layout::Alignment::Left,
            state: WidgetState::Normal,
        }
    }

    pub fn line<S: Into<String>>(mut self, content: S) -> Self {
        self.lines.push(Line::from(content.into()));
        self
    }

    pub fn styled_line<S: Into<String>>(mut self, content: S, style: Style) -> Self {
        self.lines.push(Line::from(Span::styled(content.into(), style)));
        self
    }

    pub fn spans(mut self, spans: Vec<Span<'static>>) -> Self {
        self.lines.push(Line::from(spans));
        self
    }

    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn alignment(mut self, alignment: ratatui::layout::Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn center(mut self) -> Self {
        self.alignment = ratatui::layout::Alignment::Center;
        self
    }

    pub fn right(mut self) -> Self {
        self.alignment = ratatui::layout::Alignment::Right;
        self
    }
}

impl Widget for TextBlock {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let text = Text::from(self.lines.clone());

        let mut paragraph = Paragraph::new(text).alignment(self.alignment);

        if self.wrap {
            paragraph = paragraph.wrap(Wrap { trim: true });
        }

        RatatuiWidget::render(paragraph, area, buf);
    }
}

impl Default for TextBlock {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for common text styling
pub mod presets {
    use super::*;

    pub fn title<S: Into<String>>(content: S) -> StyledText {
        StyledText::new(content)
            .bold()
            .fg(Color::Cyan)
    }

    pub fn subtitle<S: Into<String>>(content: S) -> StyledText {
        StyledText::new(content)
            .fg(Color::Yellow)
    }

    pub fn error<S: Into<String>>(content: S) -> StyledText {
        StyledText::new(content)
            .bold()
            .fg(Color::Red)
    }

    pub fn warning<S: Into<String>>(content: S) -> StyledText {
        StyledText::new(content)
            .bold()
            .fg(Color::Yellow)
    }

    pub fn success<S: Into<String>>(content: S) -> StyledText {
        StyledText::new(content)
            .bold()
            .fg(Color::Green)
    }

    pub fn info<S: Into<String>>(content: S) -> StyledText {
        StyledText::new(content)
            .fg(Color::Blue)
    }

    pub fn muted<S: Into<String>>(content: S) -> StyledText {
        StyledText::new(content)
            .fg(Color::DarkGray)
    }

    pub fn highlight<S: Into<String>>(content: S) -> StyledText {
        StyledText::new(content)
            .bold()
            .bg(Color::Yellow)
            .fg(Color::Black)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_styled_text_creation() {
        let text = StyledText::new("Hello World")
            .bold()
            .fg(Color::Red);

        assert_eq!(text.content, "Hello World");
        assert!(text.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_text_block_creation() {
        let block = TextBlock::new()
            .line("Line 1")
            .line("Line 2")
            .center();

        assert_eq!(block.lines.len(), 2);
        assert_eq!(block.alignment, ratatui::layout::Alignment::Center);
    }

    #[test]
    fn test_presets() {
        let title = presets::title("Test Title");
        assert!(title.style.add_modifier.contains(Modifier::BOLD));

        let error = presets::error("Error Message");
        assert_eq!(title.style.fg, Some(Color::Cyan));
    }
}