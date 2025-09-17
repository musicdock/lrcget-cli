//! Table widget for displaying structured data

use ratatui::{
    layout::{Rect, Constraint},
    style::{Color, Style, Modifier},
    widgets::{Table as RatatuiTable, Row as RatatuiRow, Cell, Widget as RatatuiWidget},
    buffer::Buffer,
    text::{Line, Span},
};

use super::{Widget, WidgetState};

/// A table row
#[derive(Debug, Clone)]
pub struct TableRow {
    cells: Vec<String>,
    style: Style,
    selected: bool,
}

impl TableRow {
    pub fn new(cells: Vec<String>) -> Self {
        Self {
            cells,
            style: Style::default(),
            selected: false,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn to_ratatui_row(&self) -> RatatuiRow {
        let cells: Vec<Cell> = self.cells.iter()
            .map(|cell| Cell::from(cell.as_str()))
            .collect();

        let mut row = RatatuiRow::new(cells);

        if self.selected {
            row = row.style(self.style.add_modifier(Modifier::REVERSED));
        } else {
            row = row.style(self.style);
        }

        row
    }
}

/// A scrollable table widget
#[derive(Debug, Clone)]
pub struct Table {
    headers: Vec<String>,
    rows: Vec<TableRow>,
    widths: Vec<Constraint>,
    header_style: Style,
    selected_index: Option<usize>,
    scroll_offset: usize,
    state: WidgetState,
    id: String,
}

impl Table {
    pub fn new(headers: Vec<String>) -> Self {
        let widths = headers.iter()
            .map(|_| Constraint::Percentage(100 / headers.len() as u16))
            .collect();

        Self {
            headers,
            rows: Vec::new(),
            widths,
            header_style: Style::default().add_modifier(Modifier::BOLD),
            selected_index: None,
            scroll_offset: 0,
            state: WidgetState::Normal,
            id: "table".to_string(),
        }
    }

    pub fn rows(mut self, rows: Vec<TableRow>) -> Self {
        self.rows = rows;
        self
    }

    pub fn row(mut self, row: TableRow) -> Self {
        self.rows.push(row);
        self
    }

    pub fn widths(mut self, widths: Vec<Constraint>) -> Self {
        self.widths = widths;
        self
    }

    pub fn header_style(mut self, style: Style) -> Self {
        self.header_style = style;
        self
    }

    pub fn with_id<S: Into<String>>(mut self, id: S) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_selected(mut self, index: Option<usize>) -> Self {
        self.selected_index = index;
        if let Some(idx) = index {
            // Update row selection
            for (i, row) in self.rows.iter_mut().enumerate() {
                row.selected = i == idx;
            }
        }
        self
    }

    pub fn scroll_to(mut self, offset: usize) -> Self {
        self.scroll_offset = offset.min(self.rows.len().saturating_sub(1));
        self
    }

    /// Navigate to next row
    pub fn next(&mut self) -> bool {
        if let Some(selected) = self.selected_index {
            if selected < self.rows.len() - 1 {
                self.select(Some(selected + 1));
                return true;
            }
        } else if !self.rows.is_empty() {
            self.select(Some(0));
            return true;
        }
        false
    }

    /// Navigate to previous row
    pub fn previous(&mut self) -> bool {
        if let Some(selected) = self.selected_index {
            if selected > 0 {
                self.select(Some(selected - 1));
                return true;
            }
        } else if !self.rows.is_empty() {
            self.select(Some(self.rows.len() - 1));
            return true;
        }
        false
    }

    /// Select specific row
    pub fn select(&mut self, index: Option<usize>) {
        // Clear previous selection
        for row in &mut self.rows {
            row.selected = false;
        }

        self.selected_index = index;

        // Set new selection
        if let Some(idx) = index {
            if idx < self.rows.len() {
                self.rows[idx].selected = true;
            }
        }
    }

    /// Get currently selected row
    pub fn selected(&self) -> Option<&TableRow> {
        self.selected_index.and_then(|idx| self.rows.get(idx))
    }

    /// Get visible rows based on scroll offset and area height
    fn get_visible_rows(&self, max_rows: usize) -> &[TableRow] {
        let start = self.scroll_offset;
        let end = (start + max_rows).min(self.rows.len());
        &self.rows[start..end]
    }

    /// Update scroll offset to ensure selected row is visible
    fn update_scroll(&mut self, visible_rows: usize) {
        if let Some(selected) = self.selected_index {
            if selected < self.scroll_offset {
                self.scroll_offset = selected;
            } else if selected >= self.scroll_offset + visible_rows {
                self.scroll_offset = selected.saturating_sub(visible_rows - 1);
            }
        }
    }
}

impl Widget for Table {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let visible_rows = area.height.saturating_sub(1) as usize; // -1 for header
        let visible_data = self.get_visible_rows(visible_rows);

        let ratatui_rows: Vec<RatatuiRow> = visible_data.iter()
            .map(|row| row.to_ratatui_row())
            .collect();

        let table = RatatuiTable::new(ratatui_rows, &self.widths)
            .header(
                RatatuiRow::new(
                    self.headers.iter()
                        .map(|h| Cell::from(h.as_str()))
                        .collect::<Vec<_>>()
                ).style(self.header_style)
            );

        RatatuiWidget::render(table, area, buf);
    }

    fn handle_input(&mut self, event: &crossterm::event::KeyEvent) -> bool {
        use crossterm::event::KeyCode;

        match event.code {
            KeyCode::Up => self.previous(),
            KeyCode::Down => self.next(),
            KeyCode::PageUp => {
                if let Some(selected) = self.selected_index {
                    let new_selected = selected.saturating_sub(10);
                    self.select(Some(new_selected));
                    true
                } else {
                    false
                }
            },
            KeyCode::PageDown => {
                if let Some(selected) = self.selected_index {
                    let new_selected = (selected + 10).min(self.rows.len() - 1);
                    self.select(Some(new_selected));
                    true
                } else {
                    false
                }
            },
            KeyCode::Home => {
                if !self.rows.is_empty() {
                    self.select(Some(0));
                    true
                } else {
                    false
                }
            },
            KeyCode::End => {
                if !self.rows.is_empty() {
                    self.select(Some(self.rows.len() - 1));
                    true
                } else {
                    false
                }
            },
            _ => false,
        }
    }

    fn can_focus(&self) -> bool {
        !self.rows.is_empty()
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

impl Default for Table {
    fn default() -> Self {
        Self::new(vec!["Column 1".to_string(), "Column 2".to_string()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_creation() {
        let table = Table::new(vec!["Name".to_string(), "Value".to_string()]);
        assert_eq!(table.headers.len(), 2);
        assert_eq!(table.widths.len(), 2);
    }

    #[test]
    fn test_table_navigation() {
        let mut table = Table::new(vec!["Col1".to_string()])
            .rows(vec![
                TableRow::new(vec!["Row 1".to_string()]),
                TableRow::new(vec!["Row 2".to_string()]),
                TableRow::new(vec!["Row 3".to_string()]),
            ]);

        assert!(table.next()); // Select first row
        assert_eq!(table.selected_index, Some(0));

        assert!(table.next()); // Move to second row
        assert_eq!(table.selected_index, Some(1));

        assert!(table.previous()); // Move back to first row
        assert_eq!(table.selected_index, Some(0));
    }

    #[test]
    fn test_table_row_selection() {
        let mut table = Table::new(vec!["Col1".to_string()])
            .rows(vec![
                TableRow::new(vec!["Row 1".to_string()]),
                TableRow::new(vec!["Row 2".to_string()]),
            ]);

        table.select(Some(1));
        assert_eq!(table.selected_index, Some(1));
        assert!(table.rows[1].selected);
        assert!(!table.rows[0].selected);
    }

    #[test]
    fn test_visible_rows() {
        let table = Table::new(vec!["Col1".to_string()])
            .rows(vec![
                TableRow::new(vec!["Row 1".to_string()]),
                TableRow::new(vec!["Row 2".to_string()]),
                TableRow::new(vec!["Row 3".to_string()]),
                TableRow::new(vec!["Row 4".to_string()]),
            ])
            .scroll_to(1);

        let visible = table.get_visible_rows(2);
        assert_eq!(visible.len(), 2);
        assert_eq!(visible[0].cells[0], "Row 2");
    }
}