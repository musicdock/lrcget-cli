//! Block widget for creating bordered containers

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block as RatatuiBlock, Borders, Widget as RatatuiWidget},
    buffer::Buffer,
};

use super::{Widget, WidgetState};

/// A bordered container widget
#[derive(Debug, Clone)]
pub struct Block {
    title: Option<String>,
    borders: Borders,
    border_style: Style,
    title_style: Style,
    state: WidgetState,
}

impl Block {
    pub fn new() -> Self {
        Self {
            title: None,
            borders: Borders::ALL,
            border_style: Style::default(),
            title_style: Style::default(),
            state: WidgetState::Normal,
        }
    }

    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn borders(mut self, borders: Borders) -> Self {
        self.borders = borders;
        self
    }

    pub fn border_style(mut self, style: Style) -> Self {
        self.border_style = style;
        self
    }

    pub fn title_style(mut self, style: Style) -> Self {
        self.title_style = style;
        self
    }

    pub fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Get the inner area (content area) of the block
    pub fn inner(&self, area: Rect) -> Rect {
        let mut inner = area;

        if self.borders.intersects(Borders::LEFT) {
            inner.x += 1;
            inner.width = inner.width.saturating_sub(1);
        }
        if self.borders.intersects(Borders::RIGHT) {
            inner.width = inner.width.saturating_sub(1);
        }
        if self.borders.intersects(Borders::TOP) {
            inner.y += 1;
            inner.height = inner.height.saturating_sub(1);
        }
        if self.borders.intersects(Borders::BOTTOM) {
            inner.height = inner.height.saturating_sub(1);
        }

        inner
    }

    /// Convert to ratatui Block
    fn to_ratatui_block(&self) -> RatatuiBlock {
        let mut block = RatatuiBlock::default()
            .borders(self.borders);

        // Apply style based on state
        let border_style = match self.state {
            WidgetState::Focused => self.border_style.fg(Color::Yellow),
            WidgetState::Selected => self.border_style.fg(Color::Green),
            WidgetState::Disabled => self.border_style.fg(Color::DarkGray),
            WidgetState::Normal => self.border_style,
        };

        block = block.border_style(border_style);

        if let Some(ref title) = self.title {
            block = block.title(title.as_str()).title_style(self.title_style);
        }

        block
    }
}

impl Widget for Block {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let block = self.to_ratatui_block();
        RatatuiWidget::render(block, area, buf);
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
}

impl Default for Block {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_creation() {
        let block = Block::new()
            .title("Test Block")
            .borders(Borders::ALL);

        assert!(block.title.is_some());
        assert_eq!(block.borders, Borders::ALL);
    }

    #[test]
    fn test_inner_area_calculation() {
        let block = Block::new().borders(Borders::ALL);
        let area = Rect::new(0, 0, 10, 10);
        let inner = block.inner(area);

        assert_eq!(inner, Rect::new(1, 1, 8, 8));
    }

    #[test]
    fn test_focus_state() {
        let mut block = Block::new();
        assert_eq!(block.state, WidgetState::Normal);

        block.set_focus(true);
        assert_eq!(block.state, WidgetState::Focused);
    }
}