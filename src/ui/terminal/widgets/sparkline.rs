//! Sparkline widget for displaying mini-graphs of historical data

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Widget as RatatuiWidget, Sparkline as RatatuiSparkline},
    buffer::Buffer,
    symbols,
};
use std::collections::VecDeque;

use super::{Widget, WidgetState};

/// Direction for sparkline rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparklineDirection {
    LeftToRight,
    RightToLeft,
}

/// Sparkline widget for showing trends in numerical data
#[derive(Debug, Clone)]
pub struct Sparkline {
    data: VecDeque<u64>,
    max_data_points: usize,
    style: Style,
    direction: SparklineDirection,
    state: WidgetState,
    max_value: Option<u64>,
    widget_id: String,
}

impl Sparkline {
    /// Create a new sparkline with default settings
    pub fn new() -> Self {
        Self {
            data: VecDeque::new(),
            max_data_points: 60, // Default to 60 data points (1 minute at 1 second intervals)
            style: Style::default().fg(Color::Green),
            direction: SparklineDirection::LeftToRight,
            state: WidgetState::Normal,
            max_value: None,
            widget_id: "sparkline".to_string(),
        }
    }

    /// Create a sparkline with specified ID
    pub fn with_id<S: Into<String>>(id: S) -> Self {
        Self {
            widget_id: id.into(),
            ..Self::new()
        }
    }

    /// Set the maximum number of data points to retain
    pub fn max_data_points(mut self, max: usize) -> Self {
        self.max_data_points = max;
        // Trim existing data if necessary
        while self.data.len() > max {
            self.data.pop_front();
        }
        self
    }

    /// Set the style for the sparkline
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set the rendering direction
    pub fn direction(mut self, direction: SparklineDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Set a fixed maximum value for scaling
    pub fn max_value(mut self, max: u64) -> Self {
        self.max_value = Some(max);
        self
    }

    /// Add a new data point
    pub fn add_data_point(&mut self, value: u64) {
        self.data.push_back(value);

        // Remove oldest point if we exceed max_data_points
        if self.data.len() > self.max_data_points {
            self.data.pop_front();
        }
    }

    /// Set all data points at once
    pub fn set_data(&mut self, data: Vec<u64>) {
        self.data.clear();
        for value in data {
            self.add_data_point(value);
        }
    }

    /// Clear all data points
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Get current data as a vector
    pub fn data(&self) -> Vec<u64> {
        self.data.iter().copied().collect()
    }

    /// Get the latest data point
    pub fn latest_value(&self) -> Option<u64> {
        self.data.back().copied()
    }

    /// Get the maximum value in current data
    pub fn current_max(&self) -> Option<u64> {
        self.data.iter().max().copied()
    }

    /// Get the minimum value in current data
    pub fn current_min(&self) -> Option<u64> {
        self.data.iter().min().copied()
    }

    /// Get the average of current data
    pub fn current_average(&self) -> f64 {
        if self.data.is_empty() {
            0.0
        } else {
            let sum: u64 = self.data.iter().sum();
            sum as f64 / self.data.len() as f64
        }
    }

    /// Check if sparkline has data
    pub fn has_data(&self) -> bool {
        !self.data.is_empty()
    }

    /// Get data point count
    pub fn data_count(&self) -> usize {
        self.data.len()
    }
}

impl Default for Sparkline {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Sparkline {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if self.data.is_empty() {
            return;
        }

        // Prepare data for ratatui sparkline
        let mut data: Vec<u64> = self.data.iter().copied().collect();

        // Reverse data if rendering right to left
        if self.direction == SparklineDirection::RightToLeft {
            data.reverse();
        }

        // Determine the maximum value for scaling
        let max = self.max_value.unwrap_or_else(|| {
            self.data.iter().max().copied().unwrap_or(1)
        });

        // Apply style based on widget state
        let style = match self.state {
            WidgetState::Normal => self.style,
            WidgetState::Focused => self.style.fg(Color::Yellow),
            WidgetState::Selected => self.style.fg(Color::Cyan),
            WidgetState::Disabled => self.style.fg(Color::DarkGray),
        };

        // Create and render the sparkline
        let sparkline = RatatuiSparkline::default()
            .data(&data)
            .max(max)
            .style(style);

        RatatuiWidget::render(sparkline, area, buf);
    }

    fn widget_id(&self) -> &str {
        &self.widget_id
    }
}

/// Specialized sparkline for performance metrics
#[derive(Debug)]
pub struct PerformanceSparkline {
    sparkline: Sparkline,
    metric_name: String,
    unit: String,
    warning_threshold: Option<u64>,
    critical_threshold: Option<u64>,
}

impl PerformanceSparkline {
    /// Create a new performance sparkline
    pub fn new<S: Into<String>>(metric_name: S, unit: S) -> Self {
        Self {
            sparkline: Sparkline::new(),
            metric_name: metric_name.into(),
            unit: unit.into(),
            warning_threshold: None,
            critical_threshold: None,
        }
    }

    /// Set warning threshold (colors will change when exceeded)
    pub fn warning_threshold(mut self, threshold: u64) -> Self {
        self.warning_threshold = Some(threshold);
        self
    }

    /// Set critical threshold (colors will change when exceeded)
    pub fn critical_threshold(mut self, threshold: u64) -> Self {
        self.critical_threshold = Some(threshold);
        self
    }

    /// Add a data point and automatically adjust color based on thresholds
    pub fn add_data_point(&mut self, value: u64) {
        self.sparkline.add_data_point(value);

        // Adjust color based on thresholds
        let color = if let Some(critical) = self.critical_threshold {
            if value >= critical {
                Color::Red
            } else if let Some(warning) = self.warning_threshold {
                if value >= warning {
                    Color::Yellow
                } else {
                    Color::Green
                }
            } else {
                Color::Green
            }
        } else if let Some(warning) = self.warning_threshold {
            if value >= warning {
                Color::Yellow
            } else {
                Color::Green
            }
        } else {
            Color::Green
        };

        self.sparkline.style = self.sparkline.style.fg(color);
    }

    /// Get the metric name
    pub fn metric_name(&self) -> &str {
        &self.metric_name
    }

    /// Get the unit
    pub fn unit(&self) -> &str {
        &self.unit
    }

    /// Get a formatted current value string
    pub fn formatted_current_value(&self) -> String {
        match self.sparkline.latest_value() {
            Some(value) => format!("{} {}", value, self.unit),
            None => format!("-- {}", self.unit),
        }
    }

    /// Get access to the underlying sparkline
    pub fn sparkline(&self) -> &Sparkline {
        &self.sparkline
    }

    /// Get mutable access to the underlying sparkline
    pub fn sparkline_mut(&mut self) -> &mut Sparkline {
        &mut self.sparkline
    }
}

impl Widget for PerformanceSparkline {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.sparkline.render(area, buf);
    }

    fn widget_id(&self) -> &str {
        self.sparkline.widget_id()
    }
}

/// Collection of sparklines for dashboard-style display
#[derive(Debug)]
pub struct SparklineGroup {
    sparklines: Vec<(String, PerformanceSparkline)>,
    layout_direction: SparklineDirection,
}

impl SparklineGroup {
    /// Create a new sparkline group
    pub fn new() -> Self {
        Self {
            sparklines: Vec::new(),
            layout_direction: SparklineDirection::LeftToRight,
        }
    }

    /// Add a sparkline to the group
    pub fn add_sparkline<S: Into<String>>(&mut self, id: S, sparkline: PerformanceSparkline) {
        self.sparklines.push((id.into(), sparkline));
    }

    /// Get a mutable reference to a sparkline by ID
    pub fn get_sparkline_mut(&mut self, id: &str) -> Option<&mut PerformanceSparkline> {
        self.sparklines.iter_mut()
            .find(|(sparkline_id, _)| sparkline_id == id)
            .map(|(_, sparkline)| sparkline)
    }

    /// Get a reference to a sparkline by ID
    pub fn get_sparkline(&self, id: &str) -> Option<&PerformanceSparkline> {
        self.sparklines.iter()
            .find(|(sparkline_id, _)| sparkline_id == id)
            .map(|(_, sparkline)| sparkline)
    }

    /// Get all sparkline IDs
    pub fn sparkline_ids(&self) -> Vec<&str> {
        self.sparklines.iter().map(|(id, _)| id.as_str()).collect()
    }

    /// Clear all sparklines
    pub fn clear_all(&mut self) {
        for (_, sparkline) in &mut self.sparklines {
            sparkline.sparkline_mut().clear();
        }
    }

    /// Get the number of sparklines
    pub fn len(&self) -> usize {
        self.sparklines.len()
    }

    /// Check if the group is empty
    pub fn is_empty(&self) -> bool {
        self.sparklines.is_empty()
    }
}

impl Default for SparklineGroup {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparkline_creation() {
        let sparkline = Sparkline::new();
        assert_eq!(sparkline.max_data_points, 60);
        assert_eq!(sparkline.data_count(), 0);
        assert!(!sparkline.has_data());
    }

    #[test]
    fn test_sparkline_data_management() {
        let mut sparkline = Sparkline::new().max_data_points(3);

        sparkline.add_data_point(10);
        sparkline.add_data_point(20);
        sparkline.add_data_point(30);
        sparkline.add_data_point(40); // Should remove the first point (10)

        assert_eq!(sparkline.data_count(), 3);
        assert_eq!(sparkline.data(), vec![20, 30, 40]);
        assert_eq!(sparkline.latest_value(), Some(40));
        assert_eq!(sparkline.current_max(), Some(40));
        assert_eq!(sparkline.current_min(), Some(20));
    }

    #[test]
    fn test_sparkline_statistics() {
        let mut sparkline = Sparkline::new();
        sparkline.set_data(vec![10, 20, 30]);

        assert_eq!(sparkline.current_average(), 20.0);
        assert_eq!(sparkline.current_max(), Some(30));
        assert_eq!(sparkline.current_min(), Some(10));
        assert!(sparkline.has_data());
    }

    #[test]
    fn test_performance_sparkline() {
        let mut perf_sparkline = PerformanceSparkline::new("CPU", "%")
            .warning_threshold(70)
            .critical_threshold(90);

        assert_eq!(perf_sparkline.metric_name(), "CPU");
        assert_eq!(perf_sparkline.unit(), "%");
        assert_eq!(perf_sparkline.formatted_current_value(), "-- %");

        perf_sparkline.add_data_point(50);
        assert_eq!(perf_sparkline.formatted_current_value(), "50 %");
    }

    #[test]
    fn test_sparkline_group() {
        let mut group = SparklineGroup::new();
        let cpu_sparkline = PerformanceSparkline::new("CPU", "%");
        let mem_sparkline = PerformanceSparkline::new("Memory", "MB");

        group.add_sparkline("cpu", cpu_sparkline);
        group.add_sparkline("memory", mem_sparkline);

        assert_eq!(group.len(), 2);
        assert!(!group.is_empty());
        assert!(group.get_sparkline("cpu").is_some());
        assert!(group.get_sparkline("memory").is_some());
        assert!(group.get_sparkline("disk").is_none());
    }

    #[test]
    fn test_sparkline_clear() {
        let mut sparkline = Sparkline::new();
        sparkline.add_data_point(10);
        sparkline.add_data_point(20);

        assert!(sparkline.has_data());
        sparkline.clear();
        assert!(!sparkline.has_data());
        assert_eq!(sparkline.data_count(), 0);
    }

    #[test]
    fn test_sparkline_direction() {
        let sparkline = Sparkline::new()
            .direction(SparklineDirection::RightToLeft);

        assert_eq!(sparkline.direction, SparklineDirection::RightToLeft);
    }
}