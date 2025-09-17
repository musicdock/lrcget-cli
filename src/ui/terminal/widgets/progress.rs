//! Progress bar and gauge widgets

use ratatui::{
    layout::Rect,
    style::{Color, Style, Modifier},
    widgets::{Gauge as RatatuiGauge, LineGauge, Widget as RatatuiWidget, Block, Borders},
    buffer::Buffer,
    text::{Span, Line},
};
use std::time::{Duration, Instant};

use super::{Widget, WidgetState};

/// A progress bar widget
#[derive(Debug, Clone)]
pub struct ProgressBar {
    progress: f64,
    label: Option<String>,
    style: Style,
    filled_style: Style,
    state: WidgetState,
    show_percentage: bool,
}

impl ProgressBar {
    pub fn new() -> Self {
        Self {
            progress: 0.0,
            label: None,
            style: Style::default(),
            filled_style: Style::default().bg(Color::Blue),
            state: WidgetState::Normal,
            show_percentage: true,
        }
    }

    pub fn progress(mut self, progress: f64) -> Self {
        self.progress = progress.clamp(0.0, 1.0);
        self
    }

    pub fn label<S: Into<String>>(mut self, label: S) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn filled_style(mut self, style: Style) -> Self {
        self.filled_style = style;
        self
    }

    pub fn show_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }

    fn get_display_label(&self) -> String {
        match (&self.label, self.show_percentage) {
            (Some(label), true) => format!("{} {:.1}%", label, self.progress * 100.0),
            (Some(label), false) => label.clone(),
            (None, true) => format!("{:.1}%", self.progress * 100.0),
            (None, false) => String::new(),
        }
    }
}

impl Widget for ProgressBar {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let gauge = LineGauge::default()
            .ratio(self.progress)
            .label(self.get_display_label())
            .style(self.style)
            .gauge_style(self.filled_style);

        RatatuiWidget::render(gauge, area, buf);
    }
}

impl Default for ProgressBar {
    fn default() -> Self {
        Self::new()
    }
}

/// A circular/block gauge widget
#[derive(Debug, Clone)]
pub struct Gauge {
    progress: f64,
    label: Option<String>,
    style: Style,
    gauge_style: Style,
    state: WidgetState,
}

impl Gauge {
    pub fn new() -> Self {
        Self {
            progress: 0.0,
            label: None,
            style: Style::default(),
            gauge_style: Style::default().bg(Color::Blue),
            state: WidgetState::Normal,
        }
    }

    pub fn progress(mut self, progress: f64) -> Self {
        self.progress = progress.clamp(0.0, 1.0);
        self
    }

    pub fn label<S: Into<String>>(mut self, label: S) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn gauge_style(mut self, style: Style) -> Self {
        self.gauge_style = style;
        self
    }

    fn get_display_label(&self) -> String {
        match &self.label {
            Some(label) => format!("{} ({:.1}%)", label, self.progress * 100.0),
            None => format!("{:.1}%", self.progress * 100.0),
        }
    }
}

impl Widget for Gauge {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let gauge = RatatuiGauge::default()
            .ratio(self.progress)
            .label(self.get_display_label())
            .style(self.style)
            .gauge_style(self.gauge_style);

        RatatuiWidget::render(gauge, area, buf);
    }
}

impl Default for Gauge {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to create ASCII progress bar
pub fn ascii_progress_bar(progress: f64, width: usize) -> String {
    let filled = (progress * width as f64) as usize;
    let empty = width.saturating_sub(filled);

    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// Helper function to create Unicode progress bar
pub fn unicode_progress_bar(progress: f64, width: usize) -> String {
    let filled = (progress * width as f64) as usize;
    let empty = width.saturating_sub(filled);

    format!("{}{}", "▰".repeat(filled), "▱".repeat(empty))
}

/// Advanced gauge widget with additional features
#[derive(Debug, Clone)]
pub struct AdvancedGauge {
    progress: f64,
    label: String,
    style: Style,
    gauge_style: Style,
    state: WidgetState,
    widget_id: String,

    // Enhanced features
    show_percentage: bool,
    show_value: bool,
    min_value: f64,
    max_value: f64,
    current_value: f64,
    unit: String,

    // Visual enhancements
    use_gradient: bool,
    warning_threshold: Option<f64>,
    critical_threshold: Option<f64>,

    // Animation support
    animated: bool,
    last_update: Option<Instant>,
    animation_duration: Duration,
    target_progress: f64,
}

impl AdvancedGauge {
    /// Create a new advanced gauge
    pub fn new<S: Into<String>>(label: S) -> Self {
        Self {
            progress: 0.0,
            label: label.into(),
            style: Style::default(),
            gauge_style: Style::default().bg(Color::Blue),
            state: WidgetState::Normal,
            widget_id: "advanced_gauge".to_string(),

            show_percentage: true,
            show_value: false,
            min_value: 0.0,
            max_value: 100.0,
            current_value: 0.0,
            unit: String::new(),

            use_gradient: false,
            warning_threshold: None,
            critical_threshold: None,

            animated: false,
            last_update: None,
            animation_duration: Duration::from_millis(200),
            target_progress: 0.0,
        }
    }

    /// Create gauge with custom ID
    pub fn with_id<S: Into<String>>(label: S, id: S) -> Self {
        Self {
            widget_id: id.into(),
            ..Self::new(label)
        }
    }

    /// Set the current value (will calculate progress automatically)
    pub fn value(mut self, value: f64) -> Self {
        self.current_value = value.clamp(self.min_value, self.max_value);
        self.update_progress_from_value();
        self
    }

    /// Set the progress directly (0.0 to 1.0)
    pub fn progress(mut self, progress: f64) -> Self {
        let new_progress = progress.clamp(0.0, 1.0);

        if self.animated {
            self.target_progress = new_progress;
            self.last_update = Some(Instant::now());
        } else {
            self.progress = new_progress;
            self.target_progress = new_progress;
        }

        // Update current value based on progress
        self.current_value = self.min_value + (self.max_value - self.min_value) * self.progress;
        self
    }

    /// Set the value range
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min_value = min;
        self.max_value = max;
        self.update_progress_from_value();
        self
    }

    /// Set the unit for value display
    pub fn unit<S: Into<String>>(mut self, unit: S) -> Self {
        self.unit = unit.into();
        self
    }

    /// Configure what to show in the label
    pub fn show_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }

    /// Show the actual value in addition to percentage
    pub fn show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }

    /// Set warning threshold (0.0 to 1.0)
    pub fn warning_threshold(mut self, threshold: f64) -> Self {
        self.warning_threshold = Some(threshold.clamp(0.0, 1.0));
        self
    }

    /// Set critical threshold (0.0 to 1.0)
    pub fn critical_threshold(mut self, threshold: f64) -> Self {
        self.critical_threshold = Some(threshold.clamp(0.0, 1.0));
        self
    }

    /// Enable gradient coloring
    pub fn use_gradient(mut self, enable: bool) -> Self {
        self.use_gradient = enable;
        self
    }

    /// Enable smooth animations
    pub fn animated(mut self, enable: bool) -> Self {
        self.animated = enable;
        if !enable {
            self.progress = self.target_progress;
        }
        self
    }

    /// Set animation duration
    pub fn animation_duration(mut self, duration: Duration) -> Self {
        self.animation_duration = duration;
        self
    }

    /// Set styles
    pub fn style(mut self, style: Style, gauge_style: Style) -> Self {
        self.style = style;
        self.gauge_style = gauge_style;
        self
    }

    /// Update the gauge (call this regularly for animations)
    pub fn update(&mut self) {
        if !self.animated {
            return;
        }

        if let Some(last_update) = self.last_update {
            let elapsed = last_update.elapsed();
            if elapsed < self.animation_duration {
                // Calculate interpolated progress
                let t = elapsed.as_secs_f64() / self.animation_duration.as_secs_f64();
                let t = t.clamp(0.0, 1.0);

                // Use ease-out animation curve
                let ease_t = 1.0 - (1.0 - t).powi(3);

                let start_progress = self.progress;
                self.progress = start_progress + (self.target_progress - start_progress) * ease_t;
            } else {
                // Animation complete
                self.progress = self.target_progress;
                self.last_update = None;
            }
        }
    }

    /// Check if animation is running
    pub fn is_animating(&self) -> bool {
        self.animated && self.last_update.is_some()
    }

    /// Get the current progress value
    pub fn current_progress(&self) -> f64 {
        self.progress
    }

    /// Get the current actual value
    pub fn current_value(&self) -> f64 {
        self.current_value
    }

    /// Update progress based on current value
    fn update_progress_from_value(&mut self) {
        let range = self.max_value - self.min_value;
        if range > 0.0 {
            let new_progress = (self.current_value - self.min_value) / range;
            if self.animated {
                self.target_progress = new_progress;
                self.last_update = Some(Instant::now());
            } else {
                self.progress = new_progress;
                self.target_progress = new_progress;
            }
        }
    }

    /// Get the appropriate color based on thresholds
    fn get_threshold_color(&self) -> Color {
        if let Some(critical) = self.critical_threshold {
            if self.progress >= critical {
                return Color::Red;
            }
        }

        if let Some(warning) = self.warning_threshold {
            if self.progress >= warning {
                return Color::Yellow;
            }
        }

        Color::Green
    }

    /// Get the gauge style with appropriate coloring
    fn get_gauge_style(&self) -> Style {
        if self.use_gradient || self.warning_threshold.is_some() || self.critical_threshold.is_some() {
            self.gauge_style.bg(self.get_threshold_color())
        } else {
            self.gauge_style
        }
    }

    /// Build the display label
    fn build_label(&self) -> String {
        let mut parts = vec![self.label.clone()];

        if self.show_value {
            let value_str = if self.unit.is_empty() {
                format!("{:.1}", self.current_value)
            } else {
                format!("{:.1} {}", self.current_value, self.unit)
            };
            parts.push(value_str);
        }

        if self.show_percentage {
            parts.push(format!("{:.1}%", self.progress * 100.0));
        }

        parts.join(" | ")
    }
}

impl Widget for AdvancedGauge {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        // Apply state-specific styling
        let border_style = match self.state {
            WidgetState::Normal => self.style,
            WidgetState::Focused => self.style.add_modifier(Modifier::BOLD),
            WidgetState::Selected => self.style.fg(Color::Yellow),
            WidgetState::Disabled => self.style.fg(Color::DarkGray),
        };

        let gauge = RatatuiGauge::default()
            .ratio(self.progress)
            .label(self.build_label())
            .style(border_style)
            .gauge_style(self.get_gauge_style());

        RatatuiWidget::render(gauge, area, buf);
    }

    fn widget_id(&self) -> &str {
        &self.widget_id
    }

    fn set_focus(&mut self, focused: bool) {
        self.state = if focused {
            WidgetState::Focused
        } else {
            WidgetState::Normal
        };
    }

    fn can_focus(&self) -> bool {
        true
    }
}

/// Multi-segment progress bar for showing multiple related progress indicators
#[derive(Debug, Clone)]
pub struct MultiSegmentProgress {
    segments: Vec<ProgressSegment>,
    total_width: u16,
    label: String,
    style: Style,
    state: WidgetState,
    widget_id: String,
}

#[derive(Debug, Clone)]
pub struct ProgressSegment {
    pub progress: f64,
    pub color: Color,
    pub label: String,
    pub value: f64,
}

impl ProgressSegment {
    pub fn new<S: Into<String>>(progress: f64, color: Color, label: S) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
            color,
            label: label.into(),
            value: progress,
        }
    }

    pub fn with_value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }
}

impl MultiSegmentProgress {
    /// Create a new multi-segment progress bar
    pub fn new<S: Into<String>>(label: S) -> Self {
        Self {
            segments: Vec::new(),
            total_width: 40,
            label: label.into(),
            style: Style::default(),
            state: WidgetState::Normal,
            widget_id: "multi_segment_progress".to_string(),
        }
    }

    /// Add a progress segment
    pub fn add_segment(&mut self, segment: ProgressSegment) {
        self.segments.push(segment);
    }

    /// Set the total width for the progress bar
    pub fn width(mut self, width: u16) -> Self {
        self.total_width = width;
        self
    }

    /// Set the style
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Build the visual representation
    fn build_progress_bar(&self, available_width: u16) -> Line {
        if self.segments.is_empty() {
            return Line::from("□".repeat(available_width as usize));
        }

        let mut spans = Vec::new();
        let width_per_segment = available_width / self.segments.len() as u16;

        for segment in &self.segments {
            let filled_width = (segment.progress * width_per_segment as f64) as u16;
            let empty_width = width_per_segment - filled_width;

            // Add filled portion
            if filled_width > 0 {
                spans.push(Span::styled(
                    "█".repeat(filled_width as usize),
                    Style::default().fg(segment.color)
                ));
            }

            // Add empty portion
            if empty_width > 0 {
                spans.push(Span::styled(
                    "░".repeat(empty_width as usize),
                    Style::default().fg(Color::DarkGray)
                ));
            }
        }

        Line::from(spans)
    }

    /// Get summary statistics
    pub fn summary(&self) -> ProgressSummary {
        if self.segments.is_empty() {
            return ProgressSummary::default();
        }

        let total_progress: f64 = self.segments.iter().map(|s| s.progress).sum();
        let avg_progress = total_progress / self.segments.len() as f64;

        ProgressSummary {
            average_progress: avg_progress,
            total_segments: self.segments.len(),
            completed_segments: self.segments.iter().filter(|s| s.progress >= 1.0).count(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProgressSummary {
    pub average_progress: f64,
    pub total_segments: usize,
    pub completed_segments: usize,
}

impl Widget for MultiSegmentProgress {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.height < 2 {
            return; // Need at least 2 lines
        }

        // Apply state styling
        let style = match self.state {
            WidgetState::Focused => self.style.add_modifier(Modifier::BOLD),
            WidgetState::Selected => self.style.fg(Color::Yellow),
            WidgetState::Disabled => self.style.fg(Color::DarkGray),
            _ => self.style,
        };

        // Create block with border
        let block = Block::default()
            .title(self.label.as_str())
            .borders(Borders::ALL)
            .style(style);

        let inner = block.inner(area);
        RatatuiWidget::render(block, area, buf);

        if inner.height == 0 {
            return;
        }

        // Render progress bar
        let progress_line = self.build_progress_bar(inner.width.saturating_sub(2));

        // Calculate position for centering
        let y = inner.y + inner.height / 2;
        let x = inner.x + 1;

        // Render the progress line
        if y < inner.y + inner.height {
            buf.set_line(x, y, &progress_line, inner.width.saturating_sub(2));
        }

        // Show summary if there's space
        if inner.height > 2 {
            let summary = self.summary();
            let summary_text = format!(
                "{}/{} completed ({:.1}%)",
                summary.completed_segments,
                summary.total_segments,
                summary.average_progress * 100.0
            );

            let summary_span = Span::styled(summary_text, Style::default().fg(Color::Gray));
            let summary_y = y + 1;

            if summary_y < inner.y + inner.height {
                buf.set_span(x, summary_y, &summary_span, inner.width.saturating_sub(2));
            }
        }
    }

    fn widget_id(&self) -> &str {
        &self.widget_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar_creation() {
        let bar = ProgressBar::new()
            .progress(0.5)
            .label("Test");

        assert_eq!(bar.progress, 0.5);
        assert!(bar.label.is_some());
    }

    #[test]
    fn test_progress_clamping() {
        let bar = ProgressBar::new().progress(1.5);
        assert_eq!(bar.progress, 1.0);

        let bar = ProgressBar::new().progress(-0.5);
        assert_eq!(bar.progress, 0.0);
    }

    #[test]
    fn test_ascii_progress_bar() {
        let bar = ascii_progress_bar(0.5, 10);
        assert_eq!(bar, "█████░░░░░");

        let bar = ascii_progress_bar(1.0, 5);
        assert_eq!(bar, "█████");

        let bar = ascii_progress_bar(0.0, 5);
        assert_eq!(bar, "░░░░░");
    }

    #[test]
    fn test_display_label() {
        let bar = ProgressBar::new()
            .progress(0.75)
            .label("Download");
        assert_eq!(bar.get_display_label(), "Download 75.0%");

        let bar = ProgressBar::new()
            .progress(0.5)
            .show_percentage(false);
        assert_eq!(bar.get_display_label(), "");
    }
}