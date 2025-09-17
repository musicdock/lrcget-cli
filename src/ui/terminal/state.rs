//! Application state management for terminal UI
//!
//! Manages the state of the terminal UI application including download progress,
//! queue status, performance metrics, and UI state.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant, SystemTime};
use sysinfo::System;
use serde::Serialize;

/// Main application state for terminal UI
#[derive(Debug, Clone)]
pub struct AppState {
    /// Current operation mode
    pub mode: AppMode,
    /// Download queue with track information
    pub queue: TrackQueue,
    /// Performance and system metrics
    pub metrics: PerformanceMetrics,
    /// Application statistics
    pub stats: AppStatistics,
    /// Log entries for display
    pub logs: LogBuffer,
    /// UI state (focus, visibility, etc.)
    pub ui_state: UiState,
    /// Configuration state
    pub config: AppConfig,
}

/// Different modes the application can be in
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    /// Normal download operation
    Downloading,
    /// Operation is paused
    Paused,
    /// Showing configuration
    Configuration,
    /// Showing help
    Help,
    /// Application is shutting down
    Shutdown,
}

/// Track queue management
#[derive(Debug, Clone)]
pub struct TrackQueue {
    pub items: VecDeque<TrackQueueItem>,
    pub current_index: Option<usize>,
    pub total_tracks: usize,
    pub filter: Option<String>,
}

/// Individual track in the queue
#[derive(Debug, Clone)]
pub struct TrackQueueItem {
    pub id: u64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub status: TrackStatus,
    pub progress: f64,
    pub error_message: Option<String>,
    pub download_speed: Option<f64>,
    pub timestamp: SystemTime,
    pub started_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
}

/// Status of individual tracks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackStatus {
    Pending,
    Downloading,
    Completed,
    Failed,
    Skipped,
    Processing,
}

/// Performance metrics and system information
#[derive(Debug)]
pub struct PerformanceMetrics {
    /// Current download speed (songs per minute)
    pub current_speed: f64,
    /// Average speed over session
    pub average_speed: f64,
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage in MB
    pub memory_usage: f64,
    /// Network activity indicator
    pub network_active: bool,
    /// Historical data for charts
    pub speed_history: VecDeque<(Instant, f64)>,
    pub cpu_history: VecDeque<(Instant, f64)>,
    pub memory_history: VecDeque<(Instant, f64)>,
    /// System info provider
    system: System,
    /// Last update time
    last_update: Instant,
}

/// Application statistics
#[derive(Debug, Clone, Serialize)]
pub struct AppStatistics {
    pub total_processed: u64,
    pub completed: u64,
    pub failed: u64,
    pub skipped: u64,
    pub synced_lyrics: u64,
    pub plain_lyrics: u64,
    pub instrumental: u64,
    pub session_start: SystemTime,
    pub last_update: SystemTime,
}

/// Log entry for display
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: SystemTime,
    pub level: LogLevel,
    pub message: String,
    pub context: Option<String>,
}

/// Log levels for filtering and display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warning,
    Info,
    Debug,
}

/// Log buffer with circular storage
#[derive(Debug, Clone)]
pub struct LogBuffer {
    pub entries: VecDeque<LogEntry>,
    pub max_entries: usize,
    pub filter_level: LogLevel,
    pub auto_scroll: bool,
}

/// UI state management
#[derive(Debug, Clone)]
pub struct UiState {
    pub focused_panel: Option<crate::ui::terminal::events::PanelId>,
    pub help_visible: bool,
    pub config_visible: bool,
    pub scroll_positions: HashMap<String, usize>,
    pub last_user_interaction: Instant,
}

/// Application configuration state
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub theme: String,
    pub refresh_rate: Duration,
    pub auto_scroll_logs: bool,
    pub show_performance_charts: bool,
    pub panel_sizes: HashMap<String, u16>,
}

/// Messages for updating application state
#[derive(Debug, Clone)]
pub enum UpdateMessage {
    /// Track status changed
    TrackStatusChanged { track_id: u64, status: TrackStatus, message: Option<String> },
    /// Progress update for current track
    ProgressUpdate { track_id: u64, progress: f64 },
    /// New track added to queue
    TrackAdded(TrackQueueItem),
    /// Track removed from queue
    TrackRemoved { track_id: u64 },
    /// Queue order changed
    QueueReordered(Vec<u64>),
    /// Log entry added
    LogAdded(LogEntry),
    /// Statistics updated
    StatsUpdated(AppStatistics),
    /// Performance metrics updated
    MetricsUpdated,
    /// UI state changed
    UiStateChanged(UiState),
    /// Application mode changed
    ModeChanged(AppMode),
}

impl AppState {
    pub fn new() -> Self {
        Self {
            mode: AppMode::Downloading,
            queue: TrackQueue::new(),
            metrics: PerformanceMetrics::new(),
            stats: AppStatistics::new(),
            logs: LogBuffer::new(),
            ui_state: UiState::new(),
            config: AppConfig::default(),
        }
    }

    /// Update application state with a message
    pub fn update(&mut self, message: UpdateMessage) {
        match message {
            UpdateMessage::TrackStatusChanged { track_id, status, message } => {
                self.update_track_status(track_id, status, message);
            }
            UpdateMessage::ProgressUpdate { track_id, progress } => {
                self.update_track_progress(track_id, progress);
            }
            UpdateMessage::TrackAdded(track) => {
                self.queue.add_track(track);
            }
            UpdateMessage::TrackRemoved { track_id } => {
                self.queue.remove_track(track_id);
            }
            UpdateMessage::LogAdded(entry) => {
                self.logs.add_entry(entry);
            }
            UpdateMessage::StatsUpdated(stats) => {
                self.stats = stats;
            }
            UpdateMessage::MetricsUpdated => {
                self.metrics.update();
            }
            UpdateMessage::ModeChanged(mode) => {
                self.mode = mode;
            }
            UpdateMessage::UiStateChanged(ui_state) => {
                self.ui_state = ui_state;
            }
            _ => {} // Handle other message types as needed
        }
    }

    fn update_track_status(&mut self, track_id: u64, status: TrackStatus, message: Option<String>) {
        let (track_info, should_update_stats) = if let Some(track) = self.queue.find_track_mut(track_id) {
            track.status = status;
            track.error_message = message.clone();

            let track_info = (track.artist.clone(), track.title.clone());

            let should_update_stats = match status {
                TrackStatus::Downloading => {
                    track.started_at = Some(SystemTime::now());
                    // Handle current_index separately
                    false
                }
                TrackStatus::Completed | TrackStatus::Failed | TrackStatus::Skipped => {
                    track.completed_at = Some(SystemTime::now());
                    true
                }
                _ => false
            };

            (Some(track_info), should_update_stats)
        } else {
            (None, false)
        };

        // Handle current_index update after borrowing track
        if status == TrackStatus::Downloading {
            self.queue.current_index = self.queue.find_track_index(track_id);
        }

        // Update statistics if needed
        if should_update_stats {
            self.update_statistics(status);
        }

        // Add log entry for status change
        if let Some((artist, title)) = track_info {
            let log_message = match status {
                TrackStatus::Downloading => format!("Started: {} - {}", artist, title),
                TrackStatus::Completed => format!("✅ Completed: {} - {}", artist, title),
                TrackStatus::Failed => format!("❌ Failed: {} - {} ({})", artist, title,
                                              message.unwrap_or_else(|| "Unknown error".to_string())),
                TrackStatus::Skipped => format!("⏭️ Skipped: {} - {}", artist, title),
                _ => format!("Status changed: {} - {} -> {:?}", artist, title, status),
            };

            self.logs.add_entry(LogEntry {
                timestamp: SystemTime::now(),
                level: match status {
                    TrackStatus::Failed => LogLevel::Error,
                    TrackStatus::Skipped => LogLevel::Warning,
                    _ => LogLevel::Info,
                },
                message: log_message,
                context: Some(format!("Track ID: {}", track_id)),
            });
        }
    }

    fn update_track_progress(&mut self, track_id: u64, progress: f64) {
        if let Some(track) = self.queue.find_track_mut(track_id) {
            track.progress = progress.clamp(0.0, 1.0);
        }
    }

    fn update_statistics(&mut self, status: TrackStatus) {
        self.stats.total_processed += 1;
        match status {
            TrackStatus::Completed => self.stats.completed += 1,
            TrackStatus::Failed => self.stats.failed += 1,
            TrackStatus::Skipped => self.stats.skipped += 1,
            _ => {}
        }
        self.stats.last_update = SystemTime::now();
    }

    /// Get current download progress as percentage
    pub fn overall_progress(&self) -> f64 {
        if self.queue.total_tracks == 0 {
            return 0.0;
        }

        let completed = self.queue.items.iter()
            .filter(|item| matches!(item.status, TrackStatus::Completed | TrackStatus::Failed | TrackStatus::Skipped))
            .count();

        (completed as f64 / self.queue.total_tracks as f64) * 100.0
    }

    /// Get ETA for completion
    pub fn estimated_time_remaining(&self) -> Option<Duration> {
        let remaining = self.queue.items.iter()
            .filter(|item| matches!(item.status, TrackStatus::Pending | TrackStatus::Downloading))
            .count();

        if remaining == 0 || self.metrics.current_speed <= 0.0 {
            return None;
        }

        let minutes_remaining = remaining as f64 / self.metrics.current_speed;
        Some(Duration::from_secs((minutes_remaining * 60.0) as u64))
    }
}

impl TrackQueue {
    pub fn new() -> Self {
        Self {
            items: VecDeque::new(),
            current_index: None,
            total_tracks: 0,
            filter: None,
        }
    }

    fn add_track(&mut self, track: TrackQueueItem) {
        self.items.push_back(track);
        self.total_tracks = self.items.len();
    }

    fn remove_track(&mut self, track_id: u64) {
        self.items.retain(|item| item.id != track_id);
        self.total_tracks = self.items.len();
    }

    fn find_track_mut(&mut self, track_id: u64) -> Option<&mut TrackQueueItem> {
        self.items.iter_mut().find(|item| item.id == track_id)
    }

    fn find_track_index(&self, track_id: u64) -> Option<usize> {
        self.items.iter().position(|item| item.id == track_id)
    }

    /// Get filtered items based on current filter
    pub fn filtered_items(&self) -> Vec<&TrackQueueItem> {
        match &self.filter {
            Some(filter) => {
                let filter_lower = filter.to_lowercase();
                self.items.iter()
                    .filter(|item| {
                        item.title.to_lowercase().contains(&filter_lower) ||
                        item.artist.to_lowercase().contains(&filter_lower) ||
                        item.album.to_lowercase().contains(&filter_lower)
                    })
                    .collect()
            }
            None => self.items.iter().collect(),
        }
    }
}

impl PerformanceMetrics {
    fn new() -> Self {
        Self {
            current_speed: 0.0,
            average_speed: 0.0,
            cpu_usage: 0.0,
            memory_usage: 0.0,
            network_active: false,
            speed_history: VecDeque::new(),
            cpu_history: VecDeque::new(),
            memory_history: VecDeque::new(),
            system: System::new_all(),
            last_update: Instant::now(),
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();

        // Update system info
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();

        // Calculate CPU usage (use average of all CPUs)
        self.cpu_usage = self.system.cpus().iter()
            .map(|cpu| cpu.cpu_usage() as f64)
            .sum::<f64>() / self.system.cpus().len().max(1) as f64;

        // Calculate memory usage in MB
        self.memory_usage = (self.system.used_memory() as f64) / (1024.0 * 1024.0);

        // Add to history (keep last 60 seconds of data)
        self.cpu_history.push_back((now, self.cpu_usage));
        self.memory_history.push_back((now, self.memory_usage));

        // Keep only recent history
        let cutoff = now - Duration::from_secs(60);
        self.cpu_history.retain(|(time, _)| *time > cutoff);
        self.memory_history.retain(|(time, _)| *time > cutoff);

        self.last_update = now;
    }

    /// Update download speed
    pub fn update_speed(&mut self, songs_per_minute: f64) {
        let now = Instant::now();
        self.current_speed = songs_per_minute;
        self.speed_history.push_back((now, songs_per_minute));

        // Keep only recent history
        let cutoff = now - Duration::from_secs(60);
        self.speed_history.retain(|(time, _)| *time > cutoff);

        // Calculate average speed
        if !self.speed_history.is_empty() {
            let sum: f64 = self.speed_history.iter().map(|(_, speed)| speed).sum();
            self.average_speed = sum / self.speed_history.len() as f64;
        }
    }
}

impl AppStatistics {
    fn new() -> Self {
        let now = SystemTime::now();
        Self {
            total_processed: 0,
            completed: 0,
            failed: 0,
            skipped: 0,
            synced_lyrics: 0,
            plain_lyrics: 0,
            instrumental: 0,
            session_start: now,
            last_update: now,
        }
    }

    /// Calculate success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_processed == 0 {
            return 0.0;
        }
        (self.completed as f64 / self.total_processed as f64) * 100.0
    }

    /// Get session duration
    pub fn session_duration(&self) -> Duration {
        SystemTime::now().duration_since(self.session_start).unwrap_or_default()
    }

    /// Calculate overall progress as percentage
    pub fn overall_progress(&self) -> f64 {
        if self.total_processed == 0 {
            return 0.0;
        }
        ((self.completed + self.failed + self.skipped) as f64 / self.total_processed as f64) * 100.0
    }
}

impl LogBuffer {
    fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries: 1000,
            filter_level: LogLevel::Info,
            auto_scroll: true,
        }
    }

    pub fn add_entry(&mut self, entry: LogEntry) {
        self.entries.push_back(entry);

        // Keep buffer size under limit
        while self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }

    /// Get filtered log entries
    pub fn filtered_entries(&self) -> Vec<&LogEntry> {
        self.entries.iter()
            .filter(|entry| self.should_show_level(entry.level))
            .collect()
    }

    fn should_show_level(&self, level: LogLevel) -> bool {
        match (self.filter_level, level) {
            (LogLevel::Debug, _) => true,
            (LogLevel::Info, LogLevel::Debug) => false,
            (LogLevel::Info, _) => true,
            (LogLevel::Warning, LogLevel::Debug | LogLevel::Info) => false,
            (LogLevel::Warning, _) => true,
            (LogLevel::Error, LogLevel::Error) => true,
            (LogLevel::Error, _) => false,
        }
    }
}

impl UiState {
    fn new() -> Self {
        Self {
            focused_panel: Some(crate::ui::terminal::events::PanelId::Queue),
            help_visible: false,
            config_visible: false,
            scroll_positions: HashMap::new(),
            last_user_interaction: Instant::now(),
        }
    }
}

impl AppConfig {
    fn default() -> Self {
        Self {
            theme: "auto".to_string(),
            refresh_rate: Duration::from_millis(100), // 10 FPS
            auto_scroll_logs: true,
            show_performance_charts: true,
            panel_sizes: HashMap::new(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for AppStatistics {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for TrackQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for PerformanceMetrics {
    fn clone(&self) -> Self {
        Self {
            current_speed: self.current_speed,
            average_speed: self.average_speed,
            cpu_usage: self.cpu_usage,
            memory_usage: self.memory_usage,
            network_active: self.network_active,
            speed_history: self.speed_history.clone(),
            cpu_history: self.cpu_history.clone(),
            memory_history: self.memory_history.clone(),
            system: System::new(),
            last_update: self.last_update,
        }
    }
}

impl PartialEq for AppStatistics {
    fn eq(&self, other: &Self) -> bool {
        self.total_processed == other.total_processed &&
        self.completed == other.completed &&
        self.failed == other.failed &&
        self.skipped == other.skipped
    }
}

impl PartialEq for PerformanceMetrics {
    fn eq(&self, other: &Self) -> bool {
        self.current_speed == other.current_speed &&
        self.average_speed == other.average_speed &&
        self.cpu_usage == other.cpu_usage &&
        self.memory_usage == other.memory_usage
    }
}

impl Serialize for PerformanceMetrics {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("PerformanceMetrics", 7)?;
        state.serialize_field("current_speed", &self.current_speed)?;
        state.serialize_field("average_speed", &self.average_speed)?;
        state.serialize_field("cpu_usage", &self.cpu_usage)?;
        state.serialize_field("memory_usage", &self.memory_usage)?;
        state.serialize_field("network_active", &self.network_active)?;
        // Skip serializing history data containing Instant (not serializable)
        let speed_history_len = self.speed_history.len();
        let cpu_history_len = self.cpu_history.len();
        state.serialize_field("speed_history_len", &speed_history_len)?;
        state.serialize_field("cpu_history_len", &cpu_history_len)?;
        state.end()
    }
}