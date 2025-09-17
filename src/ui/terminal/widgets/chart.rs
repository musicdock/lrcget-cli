//! Chart widgets for displaying time-series data and performance metrics

use ratatui::{
    layout::Rect,
    style::{Color, Style, Modifier},
    widgets::{
        Widget as RatatuiWidget,
        Chart as RatatuiChart,
        Axis,
        Dataset,
        GraphType,
        Block,
        Borders
    },
    buffer::Buffer,
    symbols,
    text::Span,
};
use std::collections::VecDeque;
use std::time::{SystemTime, Duration, UNIX_EPOCH};

use super::{Widget, WidgetState};

/// Chart time range for X-axis
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimeRange {
    LastMinute,
    Last5Minutes,
    Last15Minutes,
    LastHour,
    Custom(Duration),
}

impl TimeRange {
    fn to_duration(&self) -> Duration {
        match self {
            TimeRange::LastMinute => Duration::from_secs(60),
            TimeRange::Last5Minutes => Duration::from_secs(300),
            TimeRange::Last15Minutes => Duration::from_secs(900),
            TimeRange::LastHour => Duration::from_secs(3600),
            TimeRange::Custom(duration) => *duration,
        }
    }

    fn to_label(&self) -> &'static str {
        match self {
            TimeRange::LastMinute => "1m",
            TimeRange::Last5Minutes => "5m",
            TimeRange::Last15Minutes => "15m",
            TimeRange::LastHour => "1h",
            TimeRange::Custom(_) => "custom",
        }
    }
}

/// Data point with timestamp for time-series charts
#[derive(Debug, Clone, Copy)]
pub struct DataPoint {
    pub timestamp: SystemTime,
    pub value: f64,
}

impl DataPoint {
    pub fn new(value: f64) -> Self {
        Self {
            timestamp: SystemTime::now(),
            value,
        }
    }

    pub fn with_timestamp(timestamp: SystemTime, value: f64) -> Self {
        Self { timestamp, value }
    }

    /// Get age of this data point
    pub fn age(&self) -> Duration {
        SystemTime::now().duration_since(self.timestamp).unwrap_or_default()
    }

    /// Convert timestamp to seconds since epoch for charting
    pub fn timestamp_secs(&self) -> f64 {
        self.timestamp
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
    }
}

/// Line chart widget for time-series data
#[derive(Debug)]
pub struct LineChart {
    data: VecDeque<DataPoint>,
    title: String,
    time_range: TimeRange,
    y_min: Option<f64>,
    y_max: Option<f64>,
    style: Style,
    line_style: Style,
    state: WidgetState,
    show_axes: bool,
    show_legend: bool,
    widget_id: String,
    y_label: String,
    auto_scale: bool,
}

impl LineChart {
    /// Create a new line chart
    pub fn new<S: Into<String>>(title: S) -> Self {
        Self {
            data: VecDeque::new(),
            title: title.into(),
            time_range: TimeRange::LastMinute,
            y_min: None,
            y_max: None,
            style: Style::default(),
            line_style: Style::default().fg(Color::Cyan),
            state: WidgetState::Normal,
            show_axes: true,
            show_legend: false,
            widget_id: "line_chart".to_string(),
            y_label: "Value".to_string(),
            auto_scale: true,
        }
    }

    /// Create a chart with custom ID
    pub fn with_id<S: Into<String>>(title: S, id: S) -> Self {
        Self {
            widget_id: id.into(),
            ..Self::new(title)
        }
    }

    /// Set the time range for the chart
    pub fn time_range(mut self, range: TimeRange) -> Self {
        self.time_range = range;
        self.cleanup_old_data();
        self
    }

    /// Set Y-axis bounds
    pub fn y_bounds(mut self, min: f64, max: f64) -> Self {
        self.y_min = Some(min);
        self.y_max = Some(max);
        self.auto_scale = false;
        self
    }

    /// Set Y-axis label
    pub fn y_label<S: Into<String>>(mut self, label: S) -> Self {
        self.y_label = label.into();
        self
    }

    /// Set chart and line styles
    pub fn style(mut self, chart_style: Style, line_style: Style) -> Self {
        self.style = chart_style;
        self.line_style = line_style;
        self
    }

    /// Enable or disable axes
    pub fn show_axes(mut self, show: bool) -> Self {
        self.show_axes = show;
        self
    }

    /// Enable or disable legend
    pub fn show_legend(mut self, show: bool) -> Self {
        self.show_legend = show;
        self
    }

    /// Enable or disable auto-scaling
    pub fn auto_scale(mut self, enable: bool) -> Self {
        self.auto_scale = enable;
        self
    }

    /// Add a new data point
    pub fn add_data_point(&mut self, value: f64) {
        let point = DataPoint::new(value);
        self.data.push_back(point);
        self.cleanup_old_data();
    }

    /// Add a data point with specific timestamp
    pub fn add_data_point_with_time(&mut self, timestamp: SystemTime, value: f64) {
        let point = DataPoint::with_timestamp(timestamp, value);
        self.data.push_back(point);
        self.cleanup_old_data();
    }

    /// Set all data points at once
    pub fn set_data(&mut self, data: Vec<DataPoint>) {
        self.data.clear();
        for point in data {
            self.data.push_back(point);
        }
        self.cleanup_old_data();
    }

    /// Clear all data
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Get current data points
    pub fn data(&self) -> Vec<DataPoint> {
        self.data.iter().copied().collect()
    }

    /// Get the latest value
    pub fn latest_value(&self) -> Option<f64> {
        self.data.back().map(|p| p.value)
    }

    /// Get value statistics
    pub fn value_stats(&self) -> ValueStats {
        if self.data.is_empty() {
            return ValueStats::default();
        }

        let values: Vec<f64> = self.data.iter().map(|p| p.value).collect();
        let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let sum: f64 = values.iter().sum();
        let avg = sum / values.len() as f64;

        ValueStats { min, max, avg, count: values.len() }
    }

    /// Remove data points outside the time range
    fn cleanup_old_data(&mut self) {
        let cutoff = SystemTime::now() - self.time_range.to_duration();
        while let Some(point) = self.data.front() {
            if point.timestamp < cutoff {
                self.data.pop_front();
            } else {
                break;
            }
        }
    }

    /// Calculate Y-axis bounds
    fn calculate_y_bounds(&self) -> (f64, f64) {
        if !self.auto_scale {
            return (
                self.y_min.unwrap_or(0.0),
                self.y_max.unwrap_or(100.0),
            );
        }

        if self.data.is_empty() {
            return (0.0, 100.0);
        }

        let stats = self.value_stats();
        let padding = (stats.max - stats.min) * 0.1; // 10% padding
        let min = (stats.min - padding).max(0.0); // Don't go below 0
        let max = stats.max + padding;

        (min, max)
    }

    /// Calculate X-axis bounds (time range)
    fn calculate_x_bounds(&self) -> (f64, f64) {
        let now = SystemTime::now();
        let start = now - self.time_range.to_duration();

        (
            start.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64(),
            now.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64(),
        )
    }

    /// Convert data to chart format
    fn prepare_chart_data(&self) -> Vec<(f64, f64)> {
        self.data
            .iter()
            .map(|point| (point.timestamp_secs(), point.value))
            .collect()
    }

    /// Format time labels for X-axis
    fn format_time_labels(&self) -> Vec<Span> {
        let (x_min, x_max) = self.calculate_x_bounds();
        let range = x_max - x_min;
        let intervals = 4; // Show 4 time labels

        (0..=intervals)
            .map(|i| {
                let time = x_min + (range * i as f64 / intervals as f64);
                let duration = Duration::from_secs_f64(time);
                let timestamp = UNIX_EPOCH + duration;

                // Format as MM:SS for short ranges, HH:MM for longer
                let formatted = if range < 3600.0 {
                    format_timestamp_mmss(timestamp)
                } else {
                    format_timestamp_hhmm(timestamp)
                };

                Span::styled(formatted, Style::default().fg(Color::Gray))
            })
            .collect()
    }
}

impl Widget for LineChart {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 5 {
            return; // Too small to render meaningfully
        }

        let (y_min, y_max) = self.calculate_y_bounds();
        let (x_min, x_max) = self.calculate_x_bounds();

        // Prepare chart data
        let chart_data = self.prepare_chart_data();

        // Apply style based on widget state
        let border_style = match self.state {
            WidgetState::Normal => self.style,
            WidgetState::Focused => self.style.add_modifier(Modifier::BOLD),
            WidgetState::Selected => self.style.fg(Color::Yellow),
            WidgetState::Disabled => self.style.fg(Color::DarkGray),
        };

        // Create the chart
        let datasets = vec![
            Dataset::default()
                .name(&self.title)
                .marker(symbols::Marker::Braille)
                .style(self.line_style)
                .graph_type(GraphType::Line)
                .data(&chart_data)
        ];

        let mut chart = RatatuiChart::new(datasets)
            .x_axis(
                Axis::default()
                    .title("Time")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([x_min, x_max])
            )
            .y_axis(
                Axis::default()
                    .title(self.y_label.as_str())
                    .style(Style::default().fg(Color::Gray))
                    .bounds([y_min, y_max])
            );

        if self.show_axes {
            chart = chart.block(
                Block::default()
                    .title(self.title.as_str())
                    .borders(Borders::ALL)
                    .border_style(border_style)
            );
        }

        RatatuiWidget::render(chart, area, buf);
    }

    fn widget_id(&self) -> &str {
        &self.widget_id
    }
}

/// Statistics for chart values
#[derive(Debug, Clone, Copy, Default)]
pub struct ValueStats {
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub count: usize,
}

/// Multi-line chart for comparing multiple metrics
#[derive(Debug)]
pub struct MultiLineChart {
    datasets: Vec<(String, VecDeque<DataPoint>, Style)>,
    title: String,
    time_range: TimeRange,
    y_bounds: Option<(f64, f64)>,
    style: Style,
    state: WidgetState,
    widget_id: String,
    y_label: String,
    auto_scale: bool,
}

impl MultiLineChart {
    /// Create a new multi-line chart
    pub fn new<S: Into<String>>(title: S) -> Self {
        Self {
            datasets: Vec::new(),
            title: title.into(),
            time_range: TimeRange::LastMinute,
            y_bounds: None,
            style: Style::default(),
            state: WidgetState::Normal,
            widget_id: "multi_line_chart".to_string(),
            y_label: "Value".to_string(),
            auto_scale: true,
        }
    }

    /// Add a dataset to the chart
    pub fn add_dataset<S: Into<String>>(&mut self, name: S, style: Style) {
        self.datasets.push((name.into(), VecDeque::new(), style));
    }

    /// Add data point to a specific dataset
    pub fn add_data_point(&mut self, dataset_name: &str, value: f64) {
        let point = DataPoint::new(value);
        if let Some((_, data, _)) = self.datasets.iter_mut().find(|(name, _, _)| name == dataset_name) {
            data.push_back(point);
            self.cleanup_old_data();
        }
    }

    /// Set time range for all datasets
    pub fn time_range(mut self, range: TimeRange) -> Self {
        self.time_range = range;
        self.cleanup_old_data();
        self
    }

    /// Clear all datasets
    pub fn clear_all(&mut self) {
        for (_, data, _) in &mut self.datasets {
            data.clear();
        }
    }

    /// Remove old data from all datasets
    fn cleanup_old_data(&mut self) {
        let cutoff = SystemTime::now() - self.time_range.to_duration();
        for (_, data, _) in &mut self.datasets {
            while let Some(point) = data.front() {
                if point.timestamp < cutoff {
                    data.pop_front();
                } else {
                    break;
                }
            }
        }
    }

    /// Calculate Y bounds across all datasets
    fn calculate_y_bounds(&self) -> (f64, f64) {
        if let Some(bounds) = self.y_bounds {
            return bounds;
        }

        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;

        for (_, data, _) in &self.datasets {
            for point in data {
                min = min.min(point.value);
                max = max.max(point.value);
            }
        }

        if min.is_infinite() || max.is_infinite() {
            (0.0, 100.0)
        } else {
            let padding = (max - min) * 0.1;
            ((min - padding).max(0.0), max + padding)
        }
    }
}

impl Widget for MultiLineChart {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 5 {
            return;
        }

        let (y_min, y_max) = self.calculate_y_bounds();
        let now = SystemTime::now();
        let start = now - self.time_range.to_duration();
        let x_min = start.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64();
        let x_max = now.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64();

        // Convert data to the format needed for datasets
        let chart_data_sets: Vec<(String, Vec<(f64, f64)>, Style)> = self.datasets
            .iter()
            .map(|(name, data, style)| {
                let chart_data: Vec<(f64, f64)> = data
                    .iter()
                    .map(|point| (point.timestamp_secs(), point.value))
                    .collect();
                (name.clone(), chart_data, *style)
            })
            .collect();

        // Create datasets for the chart
        let datasets: Vec<Dataset> = chart_data_sets
            .iter()
            .map(|(name, data, style)| {
                Dataset::default()
                    .name(name)
                    .marker(symbols::Marker::Braille)
                    .style(*style)
                    .graph_type(GraphType::Line)
                    .data(data)
            })
            .collect();

        let border_style = match self.state {
            WidgetState::Normal => self.style,
            WidgetState::Focused => self.style.add_modifier(Modifier::BOLD),
            WidgetState::Selected => self.style.fg(Color::Yellow),
            WidgetState::Disabled => self.style.fg(Color::DarkGray),
        };

        let chart = RatatuiChart::new(datasets)
            .block(
                Block::default()
                    .title(self.title.as_str())
                    .borders(Borders::ALL)
                    .border_style(border_style)
            )
            .x_axis(
                Axis::default()
                    .title("Time")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([x_min, x_max])
            )
            .y_axis(
                Axis::default()
                    .title(self.y_label.as_str())
                    .style(Style::default().fg(Color::Gray))
                    .bounds([y_min, y_max])
            );

        RatatuiWidget::render(chart, area, buf);
    }

    fn widget_id(&self) -> &str {
        &self.widget_id
    }
}

/// Format timestamp as MM:SS
fn format_timestamp_mmss(timestamp: SystemTime) -> String {
    match timestamp.duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            let total_secs = duration.as_secs();
            let minutes = (total_secs / 60) % 60;
            let seconds = total_secs % 60;
            format!("{:02}:{:02}", minutes, seconds)
        }
        Err(_) => "--:--".to_string(),
    }
}

/// Format timestamp as HH:MM
fn format_timestamp_hhmm(timestamp: SystemTime) -> String {
    match timestamp.duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            let total_secs = duration.as_secs();
            let hours = (total_secs / 3600) % 24;
            let minutes = (total_secs / 60) % 60;
            format!("{:02}:{:02}", hours, minutes)
        }
        Err(_) => "--:--".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_point_creation() {
        let point = DataPoint::new(42.0);
        assert_eq!(point.value, 42.0);
        assert!(point.age() < Duration::from_secs(1));
    }

    #[test]
    fn test_line_chart_creation() {
        let chart = LineChart::new("Test Chart");
        assert_eq!(chart.title, "Test Chart");
        assert_eq!(chart.time_range, TimeRange::LastMinute);
        assert!(chart.auto_scale);
    }

    #[test]
    fn test_line_chart_data_management() {
        let mut chart = LineChart::new("Test").time_range(TimeRange::LastMinute);

        chart.add_data_point(10.0);
        chart.add_data_point(20.0);
        chart.add_data_point(30.0);

        assert_eq!(chart.latest_value(), Some(30.0));

        let stats = chart.value_stats();
        assert_eq!(stats.min, 10.0);
        assert_eq!(stats.max, 30.0);
        assert_eq!(stats.avg, 20.0);
        assert_eq!(stats.count, 3);
    }

    #[test]
    fn test_multi_line_chart() {
        let mut chart = MultiLineChart::new("Multi Test");

        chart.add_dataset("Dataset 1", Style::default().fg(Color::Red));
        chart.add_dataset("Dataset 2", Style::default().fg(Color::Blue));

        chart.add_data_point("Dataset 1", 10.0);
        chart.add_data_point("Dataset 2", 20.0);

        assert_eq!(chart.datasets.len(), 2);
    }

    #[test]
    fn test_time_range_conversion() {
        assert_eq!(TimeRange::LastMinute.to_duration(), Duration::from_secs(60));
        assert_eq!(TimeRange::Last5Minutes.to_duration(), Duration::from_secs(300));
        assert_eq!(TimeRange::LastHour.to_duration(), Duration::from_secs(3600));
    }

    #[test]
    fn test_chart_bounds_calculation() {
        let mut chart = LineChart::new("Test").auto_scale(true);

        // Empty chart should have default bounds
        let (min, max) = chart.calculate_y_bounds();
        assert_eq!(min, 0.0);
        assert_eq!(max, 100.0);

        // Chart with data should calculate appropriate bounds
        chart.add_data_point(50.0);
        chart.add_data_point(100.0);

        let (min, max) = chart.calculate_y_bounds();
        assert!(min < 50.0); // Should have padding below minimum
        assert!(max > 100.0); // Should have padding above maximum
    }

    #[test]
    fn test_chart_data_cleanup() {
        let mut chart = LineChart::new("Test").time_range(TimeRange::Custom(Duration::from_secs(1)));

        // Add data point that should be kept
        chart.add_data_point(10.0);

        // Add old data point (simulate by setting timestamp)
        let old_time = SystemTime::now() - Duration::from_secs(10);
        chart.add_data_point_with_time(old_time, 20.0);

        // The cleanup should remove old data
        chart.cleanup_old_data();

        // Should only have the recent data point
        assert_eq!(chart.data().len(), 1);
        assert_eq!(chart.latest_value(), Some(10.0));
    }
}