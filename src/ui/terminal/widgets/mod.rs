//! Reusable UI widgets for terminal interface
//!
//! This module provides a collection of reusable widgets that can be composed
//! to build complex terminal user interfaces.

pub mod block;
pub mod progress;
pub mod text;
pub mod table;
pub mod input;
pub mod sparkline;
pub mod chart;

// Re-export main widget types
pub use block::Block;
pub use progress::{ProgressBar, Gauge, AdvancedGauge, MultiSegmentProgress, ProgressSegment};
pub use text::{StyledText, TextBlock};
pub use table::{Table, TableRow};
pub use input::{InputField, SearchBox};
pub use sparkline::{Sparkline, PerformanceSparkline, SparklineGroup};
pub use chart::{LineChart, MultiLineChart, DataPoint, TimeRange};

// Re-export focus types for convenience
pub use super::focus::{Focusable, FocusManager, FocusDirection};

/// Common widget trait for all UI components
pub trait Widget {
    /// Render the widget to the given area
    fn render(&self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer);

    /// Handle input events (optional)
    fn handle_input(&mut self, _event: &crossterm::event::KeyEvent) -> bool {
        false
    }

    /// Check if widget can receive focus
    fn can_focus(&self) -> bool {
        false
    }

    /// Set focus state
    fn set_focus(&mut self, _focused: bool) {}

    /// Get widget identifier for focus management
    fn widget_id(&self) -> &str {
        "widget"
    }
}

/// Blanket implementation of Focusable for all Widget implementations
impl<T: Widget> Focusable for T {
    fn can_focus(&self) -> bool {
        Widget::can_focus(self)
    }

    fn set_focus(&mut self, focused: bool) {
        Widget::set_focus(self, focused)
    }

    fn handle_input(&mut self, event: &crossterm::event::KeyEvent) -> bool {
        Widget::handle_input(self, event)
    }

    fn widget_id(&self) -> &str {
        Widget::widget_id(self)
    }
}

/// Widget state for interactive components
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetState {
    Normal,
    Focused,
    Selected,
    Disabled,
}

impl Default for WidgetState {
    fn default() -> Self {
        Self::Normal
    }
}