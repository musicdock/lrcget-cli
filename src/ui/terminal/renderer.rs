//! Efficient rendering engine for terminal UI
//!
//! Handles the actual drawing of UI components with minimal redraws
//! and proper theme application.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Modifier},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph,
        Tabs, LineGauge, Sparkline,
    },
    Frame,
    buffer::Buffer,
};
use std::collections::HashMap;
use std::time::{Instant, Duration};

use super::{
    layout::AppLayout,
    state::{AppState, TrackStatus, LogLevel},
    themes::Theme,
    state_buffer::{StateBuffer, BufferConfig},
    refresh::{RefreshManager, RefreshConfig},
};

/// Configuration for rendering optimization
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Enable double buffering to reduce flicker
    pub double_buffering: bool,
    /// Enable selective rendering (only update changed areas)
    pub selective_rendering: bool,
    /// Enable render caching for static content
    pub enable_caching: bool,
    /// Maximum FPS for rendering
    pub max_fps: u32,
    /// Enable vsync-like behavior
    pub vsync: bool,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            double_buffering: true,
            selective_rendering: true,
            enable_caching: true,
            max_fps: 60,
            vsync: true,
        }
    }
}

/// Tracks rendering regions that need updates
#[derive(Debug, Clone, PartialEq)]
pub struct DirtyRegion {
    pub area: Rect,
    pub priority: RenderPriority,
    pub timestamp: Instant,
}

/// Priority levels for rendering updates
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Cache entry for rendered content
#[derive(Debug, Clone)]
struct RenderCache {
    content: Vec<u8>, // Simplified - would store actual buffer content
    timestamp: Instant,
    hash: u64,
}

/// Main renderer for terminal UI with flicker optimization
pub struct Renderer {
    theme: Theme,
    scroll_states: HashMap<String, ListState>,
    config: RenderConfig,

    // Anti-flicker features
    previous_buffer: Option<Buffer>,
    dirty_regions: Vec<DirtyRegion>,
    render_cache: HashMap<String, RenderCache>,
    last_render: Instant,
    frame_count: u64,

    // Performance tracking
    render_times: Vec<Duration>,
    max_render_time_samples: usize,
}

impl Renderer {
    pub fn new(theme: Theme) -> Self {
        Self::with_config(theme, RenderConfig::default())
    }

    pub fn with_config(theme: Theme, config: RenderConfig) -> Self {
        Self {
            theme,
            scroll_states: HashMap::new(),
            config,
            previous_buffer: None,
            dirty_regions: Vec::new(),
            render_cache: HashMap::new(),
            last_render: Instant::now(),
            frame_count: 0,
            render_times: Vec::new(),
            max_render_time_samples: 100,
        }
    }

    /// Update theme and invalidate cache
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
        self.invalidate_cache();
        // Note: frame size would be passed from the calling context
        self.invalidate_cache();
    }

    /// Update render configuration
    pub fn set_config(&mut self, config: RenderConfig) {
        self.config = config;
        self.invalidate_cache();
    }

    /// Main render function with flicker optimization
    pub fn render(&mut self, frame: &mut Frame, layout: &AppLayout, state: &AppState) {
        let render_start = Instant::now();

        // Check if we should skip this frame due to rate limiting
        if self.should_skip_frame() {
            return;
        }

        // Prepare for rendering
        let frame_size = frame.size();
        let should_full_render = self.should_full_render(frame_size);

        if self.config.double_buffering && !should_full_render {
            // Selective rendering - only update dirty regions
            self.render_selective(frame, layout, state);
        } else {
            // Full render
            self.render_full(frame, layout, state);
        }

        // Update performance metrics
        self.record_frame_time(render_start.elapsed());
        self.frame_count += 1;
        self.last_render = Instant::now();

        // Cleanup old dirty regions
        self.cleanup_dirty_regions();
    }

    /// Perform a full render of all components
    fn render_full(&mut self, frame: &mut Frame, layout: &AppLayout, state: &AppState) {
        // Clear the frame
        frame.render_widget(Clear, frame.size());

        // Render all components
        self.render_header(frame, layout.header, state);
        self.render_main_content(frame, layout, state);
        self.render_logs(frame, layout.logs, state);
        self.render_footer(frame, layout.footer, state);

        // Render overlays (help, config) if needed
        if state.mode == super::state::AppMode::Help {
            self.render_help_overlay(frame, frame.size());
        } else if state.mode == super::state::AppMode::Configuration {
            self.render_config_overlay(frame, frame.size());
        }

        // Store current buffer for next frame comparison
        if self.config.double_buffering {
            self.previous_buffer = Some(frame.buffer_mut().clone());
        }
    }

    /// Perform selective rendering of only dirty regions
    fn render_selective(&mut self, frame: &mut Frame, layout: &AppLayout, state: &AppState) {
        // Sort dirty regions by priority
        self.dirty_regions.sort_by(|a, b| b.priority.cmp(&a.priority));

        for dirty_region in &self.dirty_regions {
            // Determine which component covers this region and render only that
            if self.region_overlaps(dirty_region.area, layout.header) {
                self.render_header(frame, layout.header, state);
            }

            if self.region_overlaps(dirty_region.area, layout.logs) {
                self.render_logs(frame, layout.logs, state);
            }

            if self.region_overlaps(dirty_region.area, layout.footer) {
                self.render_footer(frame, layout.footer, state);
            }

            // Check main content areas
            if let Some(left_panel) = layout.left_panel {
                if self.region_overlaps(dirty_region.area, left_panel) {
                    self.render_queue_panel(frame, left_panel, state);
                }
            }
            if let Some(center_panel) = layout.center_panel {
                if self.region_overlaps(dirty_region.area, center_panel) {
                    self.render_performance_panel(frame, center_panel, state);
                }
            }
            if let Some(right_panel) = layout.right_panel {
                if self.region_overlaps(dirty_region.area, right_panel) {
                    self.render_statistics_panel(frame, right_panel, state);
                }
            }
        }

        // Clear dirty regions after rendering
        self.dirty_regions.clear();
    }

    /// Check if we should skip this frame due to rate limiting
    fn should_skip_frame(&self) -> bool {
        if !self.config.vsync {
            return false;
        }

        let target_frame_time = Duration::from_millis(1000 / self.config.max_fps as u64);
        self.last_render.elapsed() < target_frame_time
    }

    /// Check if we should perform a full render
    fn should_full_render(&self, frame_size: Rect) -> bool {
        // Force full render if:
        // - No previous buffer
        // - Frame size changed
        // - Too many dirty regions (might as well do full render)
        // - Double buffering is disabled
        self.previous_buffer.is_none() ||
        !self.config.double_buffering ||
        self.dirty_regions.len() > 10 ||
        self.dirty_regions.iter().any(|r| r.priority >= RenderPriority::Critical)
    }

    /// Mark a region as dirty (needs re-rendering)
    pub fn mark_dirty(&mut self, area: Rect, priority: RenderPriority) {
        let dirty_region = DirtyRegion {
            area,
            priority,
            timestamp: Instant::now(),
        };

        // Avoid duplicate dirty regions
        if !self.dirty_regions.iter().any(|r| r.area == area) {
            self.dirty_regions.push(dirty_region);
        }
    }

    /// Mark specific UI components as dirty
    pub fn mark_header_dirty(&mut self, area: Rect) {
        self.mark_dirty(area, RenderPriority::Normal);
    }

    pub fn mark_queue_dirty(&mut self, area: Rect) {
        self.mark_dirty(area, RenderPriority::High);
    }

    pub fn mark_logs_dirty(&mut self, area: Rect) {
        self.mark_dirty(area, RenderPriority::Normal);
    }

    pub fn mark_footer_dirty(&mut self, area: Rect) {
        self.mark_dirty(area, RenderPriority::Low);
    }

    /// Check if two rectangles overlap
    fn region_overlaps(&self, a: Rect, b: Rect) -> bool {
        a.x < b.x + b.width &&
        a.x + a.width > b.x &&
        a.y < b.y + b.height &&
        a.y + a.height > b.y
    }

    /// Cleanup old dirty regions
    fn cleanup_dirty_regions(&mut self) {
        let cutoff_time = Instant::now() - Duration::from_millis(100);
        self.dirty_regions.retain(|region| region.timestamp > cutoff_time);
    }

    /// Record frame rendering time for performance analysis
    fn record_frame_time(&mut self, duration: Duration) {
        self.render_times.push(duration);

        // Keep only recent samples
        if self.render_times.len() > self.max_render_time_samples {
            self.render_times.remove(0);
        }
    }

    /// Get rendering performance metrics
    pub fn performance_metrics(&self) -> RenderPerformance {
        let avg_frame_time = if !self.render_times.is_empty() {
            self.render_times.iter().sum::<Duration>() / self.render_times.len() as u32
        } else {
            Duration::ZERO
        };

        let max_frame_time = self.render_times.iter().max().copied().unwrap_or(Duration::ZERO);
        let current_fps = if self.last_render.elapsed().as_secs() > 0 {
            self.frame_count as f64 / self.last_render.elapsed().as_secs_f64()
        } else {
            0.0
        };

        RenderPerformance {
            avg_frame_time,
            max_frame_time,
            current_fps,
            target_fps: self.config.max_fps,
            total_frames: self.frame_count,
            dirty_regions: self.dirty_regions.len(),
        }
    }

    /// Invalidate render cache
    fn invalidate_cache(&mut self) {
        if self.config.enable_caching {
            self.render_cache.clear();
        }
    }

    /// Get cached content if available and valid
    fn get_cached_content(&self, cache_key: &str) -> Option<&RenderCache> {
        if !self.config.enable_caching {
            return None;
        }

        self.render_cache.get(cache_key).and_then(|cache| {
            // Cache is valid for 100ms
            if cache.timestamp.elapsed() < Duration::from_millis(100) {
                Some(cache)
            } else {
                None
            }
        })
    }

    /// Store content in cache
    fn store_cached_content(&mut self, cache_key: String, content: Vec<u8>, hash: u64) {
        if self.config.enable_caching {
            let cache_entry = RenderCache {
                content,
                timestamp: Instant::now(),
                hash,
            };
            self.render_cache.insert(cache_key, cache_entry);
        }
    }

    /// Force a complete re-render on next frame
    pub fn force_full_render(&mut self) {
        self.previous_buffer = None;
        self.dirty_regions.clear();
        self.invalidate_cache();
    }

    /// Render header section
    fn render_header(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let title = "LRCGet - Lyrics Downloader";
        let subtitle = match state.mode {
            super::state::AppMode::Downloading => "Downloading lyrics...",
            super::state::AppMode::Paused => "Paused",
            super::state::AppMode::Configuration => "Configuration",
            super::state::AppMode::Help => "Help",
            super::state::AppMode::Shutdown => "Shutting down...",
        };

        // Split header into title and status
        let header_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(area);

        // Title section
        let title_block = Block::default()
            .borders(Borders::ALL)
            .style(self.theme.styles.header_background())
            .border_style(self.theme.styles.panel_border());

        let title_paragraph = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(title, self.theme.styles.text_bold()),
            ]),
            Line::from(vec![
                Span::styled(subtitle, self.theme.styles.text_muted()),
            ]),
        ]).block(title_block);

        frame.render_widget(title_paragraph, header_chunks[0]);

        // Status section
        let progress = state.stats.overall_progress();
        let status_text = format!(
            "Progress: {:.1}% | Queue: {} | Speed: {:.1} songs/min",
            progress,
            state.queue.total_tracks,
            state.metrics.current_speed
        );

        let status_block = Block::default()
            .borders(Borders::ALL)
            .style(self.theme.styles.header_background())
            .border_style(self.theme.styles.panel_border());

        let status_paragraph = Paragraph::new(Line::from(vec![
            Span::styled(status_text, self.theme.styles.text_primary()),
        ])).block(status_block);

        frame.render_widget(status_paragraph, header_chunks[1]);

        // Progress bar at bottom of header
        if area.height >= 3 {
            let progress_area = Rect {
                x: area.x + 1,
                y: area.y + area.height - 2,
                width: area.width - 2,
                height: 1,
            };

            let progress_gauge = Gauge::default()
                .block(Block::default())
                .gauge_style(self.theme.styles.progress_bar())
                .percent(progress as u16)
                .label(format!("{:.1}%", progress));

            frame.render_widget(progress_gauge, progress_area);
        }
    }

    /// Render main content area
    fn render_main_content(&self, frame: &mut Frame, layout: &AppLayout, state: &AppState) {
        if let Some(left_panel) = layout.left_panel {
            self.render_queue_panel(frame, left_panel, state);
        }

        if let Some(center_panel) = layout.center_panel {
            self.render_performance_panel(frame, center_panel, state);
        }

        if let Some(right_panel) = layout.right_panel {
            self.render_statistics_panel(frame, right_panel, state);
        }

        if let Some(single_panel) = layout.single_panel {
            // For minimal layouts, show tabbed interface
            self.render_tabbed_panel(frame, single_panel, state);
        }
    }

    /// Render queue panel
    fn render_queue_panel(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let focused = state.ui_state.focused_panel == Some(super::events::PanelId::Queue);
        let border_style = if focused {
            self.theme.styles.list_item_selected()
        } else {
            self.theme.styles.panel_border()
        };

        let block = Block::default()
            .title("Queue")
            .borders(Borders::ALL)
            .border_style(border_style);

        // Create list items
        let items: Vec<ListItem> = state.queue.filtered_items()
            .iter()
            .map(|track| {
                let status_symbol = match track.status {
                    TrackStatus::Pending => "⏳",
                    TrackStatus::Downloading => "⬇️",
                    TrackStatus::Processing => "⚙️",
                    TrackStatus::Completed => "✅",
                    TrackStatus::Failed => "❌",
                    TrackStatus::Skipped => "⏭️",
                };

                let progress_bar = if track.status == TrackStatus::Downloading && track.progress > 0.0 {
                    let width = 20;
                    let filled = ((track.progress * width as f64) as usize).min(width);
                    format!(" [{}{}]", "█".repeat(filled), "░".repeat(width - filled))
                } else {
                    String::new()
                };

                let content = format!(
                    "{} {} - {} ({}){}",
                    status_symbol,
                    track.artist,
                    track.title,
                    track.album,
                    progress_bar
                );

                let style = match track.status {
                    TrackStatus::Completed => self.theme.styles.text_success(),
                    TrackStatus::Failed => self.theme.styles.text_error(),
                    TrackStatus::Skipped => self.theme.styles.text_warning(),
                    TrackStatus::Downloading => self.theme.styles.text_emphasis(),
                    _ => self.theme.styles.text_primary(),
                };

                ListItem::new(Line::from(Span::styled(content, style)))
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(self.theme.styles.list_item_selected());

        frame.render_widget(list, area);
    }

    /// Render performance panel
    fn render_performance_panel(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let focused = state.ui_state.focused_panel == Some(super::events::PanelId::Performance);
        let border_style = if focused {
            self.theme.styles.list_item_selected()
        } else {
            self.theme.styles.panel_border()
        };

        let block = Block::default()
            .title("Performance")
            .borders(Borders::ALL)
            .border_style(border_style);

        // Split into metrics and charts
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(6), Constraint::Min(4)])
            .split(area);

        // Metrics section
        let metrics_text = vec![
            Line::from(vec![
                Span::styled("Current Speed: ", self.theme.styles.text_muted()),
                Span::styled(format!("{:.1} songs/min", state.metrics.current_speed), self.theme.styles.text_primary()),
            ]),
            Line::from(vec![
                Span::styled("Average Speed: ", self.theme.styles.text_muted()),
                Span::styled(format!("{:.1} songs/min", state.metrics.average_speed), self.theme.styles.text_primary()),
            ]),
            Line::from(vec![
                Span::styled("CPU Usage: ", self.theme.styles.text_muted()),
                Span::styled(format!("{:.1}%", state.metrics.cpu_usage), self.theme.styles.text_primary()),
            ]),
            Line::from(vec![
                Span::styled("Memory Usage: ", self.theme.styles.text_muted()),
                Span::styled(format!("{:.0} MB", state.metrics.memory_usage), self.theme.styles.text_primary()),
            ]),
        ];

        let metrics_paragraph = Paragraph::new(metrics_text)
            .block(block.clone());
        frame.render_widget(metrics_paragraph, chunks[0]);

        // Charts section (if there's space)
        if chunks[1].height > 3 {
            self.render_performance_charts(frame, chunks[1], state);
        }
    }

    /// Render performance charts
    fn render_performance_charts(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        // Speed history sparkline
        if !state.metrics.speed_history.is_empty() {
            let speed_data: Vec<u64> = state.metrics.speed_history
                .iter()
                .map(|(_, speed)| (*speed as u64).max(1))
                .collect();

            let sparkline = Sparkline::default()
                .block(Block::default().title("Speed History").borders(Borders::ALL))
                .data(&speed_data)
                .style(self.theme.styles.text_emphasis());

            frame.render_widget(sparkline, area);
        }
    }

    /// Render statistics panel
    fn render_statistics_panel(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let focused = state.ui_state.focused_panel == Some(super::events::PanelId::Statistics);
        let border_style = if focused {
            self.theme.styles.list_item_selected()
        } else {
            self.theme.styles.panel_border()
        };

        let block = Block::default()
            .title("Statistics")
            .borders(Borders::ALL)
            .border_style(border_style);

        let stats_text = vec![
            Line::from(vec![
                Span::styled("Total Processed: ", self.theme.styles.text_muted()),
                Span::styled(state.stats.total_processed.to_string(), self.theme.styles.text_primary()),
            ]),
            Line::from(vec![
                Span::styled("Completed: ", self.theme.styles.text_muted()),
                Span::styled(state.stats.completed.to_string(), self.theme.styles.text_success()),
            ]),
            Line::from(vec![
                Span::styled("Failed: ", self.theme.styles.text_muted()),
                Span::styled(state.stats.failed.to_string(), self.theme.styles.text_error()),
            ]),
            Line::from(vec![
                Span::styled("Skipped: ", self.theme.styles.text_muted()),
                Span::styled(state.stats.skipped.to_string(), self.theme.styles.text_warning()),
            ]),
            Line::from(vec![
                Span::styled("Success Rate: ", self.theme.styles.text_muted()),
                Span::styled(format!("{:.1}%", state.stats.success_rate()), self.theme.styles.text_primary()),
            ]),
            Line::from(vec![
                Span::styled("Session Duration: ", self.theme.styles.text_muted()),
                Span::styled(self.format_duration(state.stats.session_duration()), self.theme.styles.text_primary()),
            ]),
        ];

        let stats_paragraph = Paragraph::new(stats_text)
            .block(block);
        frame.render_widget(stats_paragraph, area);
    }

    /// Render tabbed panel for minimal layouts
    fn render_tabbed_panel(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let tabs = ["Queue", "Performance", "Statistics"];
        let current_tab = match state.ui_state.focused_panel {
            Some(super::events::PanelId::Queue) => 0,
            Some(super::events::PanelId::Performance) => 1,
            Some(super::events::PanelId::Statistics) => 2,
            _ => 0,
        };

        let tab_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        let tabs_widget = Tabs::new(tabs.iter().map(|t| Line::from(*t)).collect())
            .block(Block::default().borders(Borders::ALL))
            .select(current_tab)
            .style(self.theme.styles.text_primary())
            .highlight_style(self.theme.styles.list_item_selected());

        frame.render_widget(tabs_widget, tab_chunks[0]);

        // Render current tab content
        match current_tab {
            0 => self.render_queue_panel(frame, tab_chunks[1], state),
            1 => self.render_performance_panel(frame, tab_chunks[1], state),
            2 => self.render_statistics_panel(frame, tab_chunks[1], state),
            _ => {}
        }
    }

    /// Render logs section
    fn render_logs(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let focused = state.ui_state.focused_panel == Some(super::events::PanelId::Logs);
        let border_style = if focused {
            self.theme.styles.list_item_selected()
        } else {
            self.theme.styles.panel_border()
        };

        let block = Block::default()
            .title("Logs")
            .borders(Borders::ALL)
            .border_style(border_style);

        let log_items: Vec<ListItem> = state.logs.filtered_entries()
            .iter()
            .map(|entry| {
                let timestamp = self.format_timestamp(entry.timestamp);
                let level_symbol = match entry.level {
                    LogLevel::Error => "❌",
                    LogLevel::Warning => "⚠️",
                    LogLevel::Info => "ℹ️",
                    LogLevel::Debug => "🔍",
                };

                let style = match entry.level {
                    LogLevel::Error => self.theme.styles.text_error(),
                    LogLevel::Warning => self.theme.styles.text_warning(),
                    LogLevel::Info => self.theme.styles.text_primary(),
                    LogLevel::Debug => self.theme.styles.text_muted(),
                };

                let content = format!("{} {} {}", timestamp, level_symbol, entry.message);
                ListItem::new(Line::from(Span::styled(content, style)))
            })
            .collect();

        let logs_list = List::new(log_items)
            .block(block);

        frame.render_widget(logs_list, area);
    }

    /// Render footer
    fn render_footer(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let help_text = "Press 'h' for help, 'q' to quit, 'space' to pause/resume, '1-4' to switch panels";

        let eta_text = if state.queue.total_tracks > 0 && state.metrics.current_speed > 0.0 {
            let remaining = state.queue.items.iter()
                .filter(|item| matches!(item.status, super::state::TrackStatus::Pending | super::state::TrackStatus::Downloading))
                .count();

            if remaining > 0 {
                let minutes_remaining = remaining as f64 / state.metrics.current_speed;
                let eta_duration = Duration::from_secs((minutes_remaining * 60.0) as u64);
                format!("ETA: {}", self.format_duration(eta_duration))
            } else {
                "ETA: --:--".to_string()
            }
        } else {
            "ETA: --:--".to_string()
        };

        let footer_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
            .split(area);

        let help_paragraph = Paragraph::new(Line::from(Span::styled(help_text, self.theme.styles.text_muted())))
            .block(Block::default().borders(Borders::ALL).style(self.theme.styles.footer_background()));

        let eta_paragraph = Paragraph::new(Line::from(Span::styled(eta_text, self.theme.styles.text_primary())))
            .block(Block::default().borders(Borders::ALL).style(self.theme.styles.footer_background()));

        frame.render_widget(help_paragraph, footer_chunks[0]);
        frame.render_widget(eta_paragraph, footer_chunks[1]);
    }

    /// Render help overlay
    fn render_help_overlay(&self, frame: &mut Frame, area: Rect) {
        let popup_area = self.centered_rect(60, 70, area);

        frame.render_widget(Clear, popup_area);

        let help_text = vec![
            Line::from("LRCGet Terminal UI - Help"),
            Line::from(""),
            Line::from("Global Commands:"),
            Line::from("  q, Ctrl+C, Esc - Quit application"),
            Line::from("  space - Pause/Resume downloads"),
            Line::from("  r - Refresh display"),
            Line::from("  h, ? - Toggle this help"),
            Line::from("  c - Show configuration"),
            Line::from(""),
            Line::from("Navigation:"),
            Line::from("  Tab - Next panel"),
            Line::from("  Shift+Tab - Previous panel"),
            Line::from("  1 - Queue panel"),
            Line::from("  2 - Performance panel"),
            Line::from("  3 - Statistics panel"),
            Line::from("  4 - Logs panel"),
            Line::from(""),
            Line::from("Press any key to close help"),
        ];

        let help_paragraph = Paragraph::new(help_text)
            .block(Block::default()
                .title("Help")
                .borders(Borders::ALL)
                .style(self.theme.styles.header_background()));

        frame.render_widget(help_paragraph, popup_area);
    }

    /// Render configuration overlay
    fn render_config_overlay(&self, frame: &mut Frame, area: Rect) {
        let popup_area = self.centered_rect(50, 60, area);

        frame.render_widget(Clear, popup_area);

        let config_text = vec![
            Line::from("Configuration"),
            Line::from(""),
            Line::from("Theme: Auto"),
            Line::from("Refresh Rate: 10 FPS"),
            Line::from("Auto-scroll Logs: Yes"),
            Line::from("Show Charts: Yes"),
            Line::from(""),
            Line::from("Press 'c' to close"),
        ];

        let config_paragraph = Paragraph::new(config_text)
            .block(Block::default()
                .title("Configuration")
                .borders(Borders::ALL)
                .style(self.theme.styles.header_background()));

        frame.render_widget(config_paragraph, popup_area);
    }

    /// Helper function to center a rectangle
    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }

    /// Format duration for display
    fn format_duration(&self, duration: std::time::Duration) -> String {
        let total_seconds = duration.as_secs();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        if hours > 0 {
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{:02}:{:02}", minutes, seconds)
        }
    }

    /// Format timestamp for display
    fn format_timestamp(&self, timestamp: std::time::SystemTime) -> String {
        // Simple timestamp formatting - in a real app you'd use chrono
        let duration = timestamp.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        let seconds = duration.as_secs() % 86400; // Seconds since midnight
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        let secs = seconds % 60;

        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_creation() {
        let theme = crate::ui::terminal::Theme::dark();
        let renderer = Renderer::new(theme);
        assert!(renderer.render_cache.is_empty());
        assert_eq!(renderer.frame_count, 0);
    }

    #[test]
    fn test_duration_formatting() {
        let theme = crate::ui::terminal::Theme::dark();
        let renderer = Renderer::new(theme);

        let duration = std::time::Duration::from_secs(3665); // 1:01:05
        assert_eq!(renderer.format_duration(duration), "01:01:05");

        let duration = std::time::Duration::from_secs(125); // 2:05
        assert_eq!(renderer.format_duration(duration), "02:05");
    }

    #[test]
    fn test_dirty_region_management() {
        let theme = crate::ui::terminal::Theme::dark();
        let mut renderer = Renderer::new(theme);

        let area = Rect::new(0, 0, 10, 10);
        renderer.mark_dirty(area, RenderPriority::Normal);

        assert_eq!(renderer.dirty_regions.len(), 1);
        assert_eq!(renderer.dirty_regions[0].area, area);

        // Adding the same area again should not create duplicates
        renderer.mark_dirty(area, RenderPriority::High);
        assert_eq!(renderer.dirty_regions.len(), 1);
    }

    #[test]
    fn test_region_overlap() {
        let theme = crate::ui::terminal::Theme::dark();
        let renderer = Renderer::new(theme);

        let rect1 = Rect::new(0, 0, 10, 10);
        let rect2 = Rect::new(5, 5, 10, 10);
        let rect3 = Rect::new(20, 20, 10, 10);

        assert!(renderer.region_overlaps(rect1, rect2));
        assert!(!renderer.region_overlaps(rect1, rect3));
    }

    #[test]
    fn test_render_config() {
        let config = RenderConfig::default();
        assert!(config.double_buffering);
        assert!(config.selective_rendering);
        assert_eq!(config.max_fps, 60);
    }

    #[test]
    fn test_performance_metrics() {
        let theme = crate::ui::terminal::Theme::dark();
        let mut renderer = Renderer::new(theme);

        // Record some frame times
        renderer.record_frame_time(Duration::from_millis(16));
        renderer.record_frame_time(Duration::from_millis(20));

        let metrics = renderer.performance_metrics();
        assert!(metrics.avg_frame_time > Duration::ZERO);
        assert_eq!(metrics.target_fps, 60);
    }
}

/// Performance metrics for the renderer
#[derive(Debug, Clone)]
pub struct RenderPerformance {
    pub avg_frame_time: Duration,
    pub max_frame_time: Duration,
    pub current_fps: f64,
    pub target_fps: u32,
    pub total_frames: u64,
    pub dirty_regions: usize,
}

impl RenderPerformance {
    /// Check if rendering performance is acceptable
    pub fn is_performance_good(&self) -> bool {
        let fps_ratio = self.current_fps / self.target_fps as f64;
        let frame_time_good = self.avg_frame_time < Duration::from_millis(33); // < 30 FPS equivalent

        fps_ratio > 0.8 && frame_time_good
    }

    /// Get performance status string
    pub fn status_string(&self) -> String {
        if self.is_performance_good() {
            "Good".to_string()
        } else if self.current_fps < self.target_fps as f64 * 0.5 {
            "Poor".to_string()
        } else {
            "Fair".to_string()
        }
    }

    /// Get efficiency ratio (lower is better for dirty regions)
    pub fn efficiency_ratio(&self) -> f64 {
        if self.total_frames == 0 {
            return 1.0;
        }
        self.dirty_regions as f64 / self.total_frames as f64
    }
}