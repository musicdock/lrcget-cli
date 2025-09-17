//! State buffer system for smooth UI updates and efficient rendering

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant, SystemTime};
use std::sync::{Arc, RwLock};
use serde::Serialize;

use super::state::{AppState, TrackStatus, LogLevel, AppStatistics, PerformanceMetrics};

/// Buffered state for smooth animations and transitions
#[derive(Debug, Clone)]
pub struct StateBuffer {
    /// Current state (most recent)
    current: Arc<RwLock<AppState>>,
    /// Previous state for comparison
    previous: Option<AppState>,
    /// State history for animations and transitions
    history: VecDeque<StateSnapshot>,
    /// Configuration for buffering behavior
    config: BufferConfig,
    /// Pending state changes
    pending_changes: HashMap<String, StateChange>,
    /// Last update timestamp
    last_update: Instant,
}

/// Configuration for state buffering
#[derive(Debug, Clone)]
pub struct BufferConfig {
    /// Maximum number of state snapshots to keep
    pub max_history_size: usize,
    /// How often to take state snapshots
    pub snapshot_interval: Duration,
    /// How long to keep state history
    pub history_retention: Duration,
    /// Enable smooth transitions between states
    pub enable_transitions: bool,
    /// Transition duration for smooth changes
    pub transition_duration: Duration,
    /// Enable state diffing to minimize updates
    pub enable_diffing: bool,
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            max_history_size: 100,
            snapshot_interval: Duration::from_millis(100),
            history_retention: Duration::from_secs(30),
            enable_transitions: true,
            transition_duration: Duration::from_millis(200),
            enable_diffing: true,
        }
    }
}

impl BufferConfig {
    /// Create config optimized for performance
    pub fn performance_optimized() -> Self {
        Self {
            max_history_size: 50,
            snapshot_interval: Duration::from_millis(50),
            history_retention: Duration::from_secs(10),
            enable_transitions: false,
            transition_duration: Duration::from_millis(100),
            enable_diffing: true,
        }
    }

    /// Create config optimized for smooth animations
    pub fn animation_optimized() -> Self {
        Self {
            max_history_size: 200,
            snapshot_interval: Duration::from_millis(16), // ~60 FPS
            history_retention: Duration::from_secs(60),
            enable_transitions: true,
            transition_duration: Duration::from_millis(300),
            enable_diffing: true,
        }
    }
}

/// A snapshot of application state at a specific time
#[derive(Debug, Clone, Serialize)]
pub struct StateSnapshot {
    pub timestamp: SystemTime,
    pub statistics: AppStatistics,
    pub performance_metrics: PerformanceMetrics,
    pub track_count: usize,
    pub log_count: usize,
    pub mode: String,
    /// Hash of the state for quick comparison
    pub state_hash: u64,
}

/// Represents a pending state change
#[derive(Debug, Clone)]
pub struct StateChange {
    pub id: String,
    pub change_type: StateChangeType,
    pub target_value: StateValue,
    pub start_time: Instant,
    pub duration: Duration,
    pub easing: EasingFunction,
}

/// Types of state changes that can be animated
#[derive(Debug, Clone, PartialEq)]
pub enum StateChangeType {
    Progress,
    Count,
    Percentage,
    Color,
    Position,
}

/// Values that can be animated
#[derive(Debug, Clone)]
pub enum StateValue {
    Float(f64),
    Integer(i64),
    Color(u8, u8, u8), // RGB
    Position(f64, f64), // X, Y
}

/// Easing functions for smooth animations
#[derive(Debug, Clone, Copy)]
pub enum EasingFunction {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Bounce,
    Elastic,
}

impl EasingFunction {
    /// Apply the easing function to a progress value (0.0 to 1.0)
    pub fn apply(self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            EasingFunction::Linear => t,
            EasingFunction::EaseIn => t * t,
            EasingFunction::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            EasingFunction::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - 2.0 * (1.0 - t) * (1.0 - t)
                }
            }
            EasingFunction::Bounce => {
                if t < 1.0 / 2.75 {
                    7.5625 * t * t
                } else if t < 2.0 / 2.75 {
                    let t = t - 1.5 / 2.75;
                    7.5625 * t * t + 0.75
                } else if t < 2.5 / 2.75 {
                    let t = t - 2.25 / 2.75;
                    7.5625 * t * t + 0.9375
                } else {
                    let t = t - 2.625 / 2.75;
                    7.5625 * t * t + 0.984375
                }
            }
            EasingFunction::Elastic => {
                if t == 0.0 || t == 1.0 {
                    t
                } else {
                    let p = 0.3;
                    let s = p / 4.0;
                    -(2.0_f64.powf(10.0 * (t - 1.0)) * ((t - 1.0 - s) * (2.0 * std::f64::consts::PI) / p).sin())
                }
            }
        }
    }
}

impl StateBuffer {
    /// Create a new state buffer
    pub fn new(initial_state: AppState, config: BufferConfig) -> Self {
        let current = Arc::new(RwLock::new(initial_state));
        let mut history = VecDeque::new();

        // Create initial snapshot
        if let Ok(state) = current.read() {
            let snapshot = StateSnapshot::from_state(&state);
            history.push_back(snapshot);
        }

        Self {
            current,
            previous: None,
            history,
            config,
            pending_changes: HashMap::new(),
            last_update: Instant::now(),
        }
    }

    /// Update the state buffer with new state
    pub fn update_state(&mut self, new_state: AppState) {
        let now = Instant::now();

        // Store previous state for comparison
        if let Ok(current_state) = self.current.read() {
            self.previous = Some(current_state.clone());
        }

        // Update current state
        if let Ok(mut current) = self.current.write() {
            *current = new_state.clone();
        }

        // Take snapshot if enough time has passed
        if now.duration_since(self.last_update) >= self.config.snapshot_interval {
            self.take_snapshot(new_state);
            self.last_update = now;
        }

        // Process pending changes
        self.update_transitions();

        // Cleanup old history
        self.cleanup_history();
    }

    /// Get the current state
    pub fn current_state(&self) -> Option<AppState> {
        self.current.read().ok().map(|state| state.clone())
    }

    /// Get the previous state for comparison
    pub fn previous_state(&self) -> Option<&AppState> {
        self.previous.as_ref()
    }

    /// Check if state has changed significantly
    pub fn has_significant_change(&self) -> bool {
        if !self.config.enable_diffing {
            return true; // Always consider changes significant if diffing is disabled
        }

        if let (Some(current), Some(previous)) = (self.current_state(), &self.previous) {
            self.calculate_state_diff(&current, previous).is_significant()
        } else {
            true
        }
    }

    /// Get interpolated state for smooth animations
    pub fn interpolated_state(&self, progress: f64) -> Option<AppState> {
        if !self.config.enable_transitions || self.pending_changes.is_empty() {
            return self.current_state();
        }

        let mut state = self.current_state()?;

        // Apply pending transitions
        for change in self.pending_changes.values() {
            let elapsed = change.start_time.elapsed();
            let change_progress = (elapsed.as_secs_f64() / change.duration.as_secs_f64()).clamp(0.0, 1.0);
            let eased_progress = change.easing.apply(change_progress);

            // Apply the interpolated change to the state
            self.apply_interpolated_change(&mut state, change, eased_progress);
        }

        Some(state)
    }

    /// Start a smooth transition for a state property
    pub fn start_transition(
        &mut self,
        id: String,
        change_type: StateChangeType,
        target_value: StateValue,
        duration: Option<Duration>,
        easing: Option<EasingFunction>,
    ) {
        let change = StateChange {
            id: id.clone(),
            change_type,
            target_value,
            start_time: Instant::now(),
            duration: duration.unwrap_or(self.config.transition_duration),
            easing: easing.unwrap_or(EasingFunction::EaseOut),
        };

        self.pending_changes.insert(id, change);
    }

    /// Get state history within a time range
    pub fn state_history(&self, duration: Duration) -> Vec<&StateSnapshot> {
        let cutoff_time = SystemTime::now() - duration;
        self.history
            .iter()
            .filter(|snapshot| snapshot.timestamp > cutoff_time)
            .collect()
    }

    /// Get recent performance metrics from history
    pub fn recent_metrics(&self, duration: Duration) -> Vec<&PerformanceMetrics> {
        self.state_history(duration)
            .iter()
            .map(|snapshot| &snapshot.performance_metrics)
            .collect()
    }

    /// Calculate state difference between two states
    fn calculate_state_diff(&self, current: &AppState, previous: &AppState) -> StateDiff {
        StateDiff {
            statistics_changed: current.stats != previous.stats,
            metrics_changed: current.metrics != previous.metrics,
            queue_changed: current.queue.items.len() != previous.queue.items.len(),
            logs_changed: current.logs.entries.len() != previous.logs.entries.len(),
            mode_changed: current.mode != previous.mode,
        }
    }

    /// Take a snapshot of the current state
    fn take_snapshot(&mut self, state: AppState) {
        let snapshot = StateSnapshot::from_state(&state);
        self.history.push_back(snapshot);

        // Limit history size
        while self.history.len() > self.config.max_history_size {
            self.history.pop_front();
        }
    }

    /// Update ongoing transitions
    fn update_transitions(&mut self) {
        let now = Instant::now();
        let mut completed_transitions = Vec::new();

        for (id, change) in &self.pending_changes {
            let elapsed = now.duration_since(change.start_time);
            if elapsed >= change.duration {
                completed_transitions.push(id.clone());
            }
        }

        // Remove completed transitions
        for id in completed_transitions {
            self.pending_changes.remove(&id);
        }
    }

    /// Apply an interpolated change to a state
    fn apply_interpolated_change(&self, state: &mut AppState, change: &StateChange, progress: f64) {
        // This is a simplified implementation - in practice, you'd need to handle
        // different types of state changes based on the change_type and target_value
        match (&change.change_type, &change.target_value) {
            (StateChangeType::Progress, StateValue::Float(target)) => {
                // Example: interpolate overall progress
                let current_progress = state.stats.overall_progress();
                let new_progress = current_progress + (target - current_progress) * progress;
                // Note: You'd need to update the actual progress in the state
            }
            (StateChangeType::Count, StateValue::Integer(target)) => {
                // Example: interpolate counters
                let current_count = state.stats.total_processed as f64;
                let new_count = current_count + (*target as f64 - current_count) * progress;
                // Note: You'd need to update the actual count in the state
            }
            _ => {
                // Handle other types of changes
            }
        }
    }

    /// Cleanup old history entries
    fn cleanup_history(&mut self) {
        let cutoff_time = SystemTime::now() - self.config.history_retention;
        while let Some(front) = self.history.front() {
            if front.timestamp < cutoff_time {
                self.history.pop_front();
            } else {
                break;
            }
        }
    }

    /// Get buffer statistics
    pub fn buffer_stats(&self) -> BufferStats {
        let pending_transitions = self.pending_changes.len();
        let history_size = self.history.len();
        let memory_usage = self.estimate_memory_usage();

        BufferStats {
            pending_transitions,
            history_size,
            memory_usage,
            last_update: self.last_update,
        }
    }

    /// Estimate memory usage of the buffer
    fn estimate_memory_usage(&self) -> usize {
        // Rough estimation of memory usage
        let snapshot_size = std::mem::size_of::<StateSnapshot>();
        let history_size = self.history.len() * snapshot_size;
        let pending_changes_size = self.pending_changes.len() * std::mem::size_of::<StateChange>();

        history_size + pending_changes_size
    }
}

impl StateSnapshot {
    /// Create a snapshot from an AppState
    pub fn from_state(state: &AppState) -> Self {
        Self {
            timestamp: SystemTime::now(),
            statistics: state.stats.clone(),
            performance_metrics: state.metrics.clone(),
            track_count: state.queue.items.len(),
            log_count: state.logs.entries.len(),
            mode: format!("{:?}", state.mode),
            state_hash: Self::calculate_state_hash(state),
        }
    }

    /// Calculate a simple hash of the state for quick comparison
    fn calculate_state_hash(state: &AppState) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        state.stats.total_processed.hash(&mut hasher);
        state.stats.completed.hash(&mut hasher);
        state.stats.failed.hash(&mut hasher);
        state.queue.items.len().hash(&mut hasher);
        state.logs.entries.len().hash(&mut hasher);
        hasher.finish()
    }
}

/// Statistics about state differences
#[derive(Debug, Clone)]
pub struct StateDiff {
    pub statistics_changed: bool,
    pub metrics_changed: bool,
    pub queue_changed: bool,
    pub logs_changed: bool,
    pub mode_changed: bool,
}

impl StateDiff {
    /// Check if the difference is significant enough to warrant a UI update
    pub fn is_significant(&self) -> bool {
        self.statistics_changed || self.queue_changed || self.logs_changed || self.mode_changed
    }

    /// Get a score indicating how much has changed (0.0 to 1.0)
    pub fn change_score(&self) -> f64 {
        let changes = [
            self.statistics_changed,
            self.metrics_changed,
            self.queue_changed,
            self.logs_changed,
            self.mode_changed,
        ];

        let total_changes = changes.iter().filter(|&&changed| changed).count();
        total_changes as f64 / changes.len() as f64
    }
}

/// Statistics about the state buffer
#[derive(Debug, Clone)]
pub struct BufferStats {
    pub pending_transitions: usize,
    pub history_size: usize,
    pub memory_usage: usize,
    pub last_update: Instant,
}

impl BufferStats {
    /// Check if the buffer is performing well
    pub fn is_healthy(&self) -> bool {
        // Buffer is healthy if:
        // - Not too many pending transitions
        // - Memory usage is reasonable
        // - Recent updates
        self.pending_transitions < 10 &&
        self.memory_usage < 1024 * 1024 && // < 1MB
        self.last_update.elapsed() < Duration::from_secs(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_state() -> AppState {
        AppState {
            stats: AppStatistics::default(),
            metrics: PerformanceMetrics::default(),
            queue: super::state::TrackQueue::new(),
            logs: super::state::LogBuffer::new(),
            mode: super::super::state::AppMode::Downloading,
        }
    }

    #[test]
    fn test_buffer_creation() {
        let state = create_test_state();
        let config = BufferConfig::default();
        let buffer = StateBuffer::new(state, config);

        assert_eq!(buffer.history.len(), 1); // Initial snapshot
        assert!(buffer.pending_changes.is_empty());
    }

    #[test]
    fn test_state_update() {
        let initial_state = create_test_state();
        let config = BufferConfig::default();
        let mut buffer = StateBuffer::new(initial_state, config);

        let mut new_state = create_test_state();
        new_state.stats.completed = 5;

        buffer.update_state(new_state);

        let current = buffer.current_state().unwrap();
        assert_eq!(current.stats.completed, 5);
    }

    #[test]
    fn test_state_diff() {
        let initial_state = create_test_state();
        let config = BufferConfig::default();
        let buffer = StateBuffer::new(initial_state, config);

        let mut changed_state = create_test_state();
        changed_state.stats.completed = 10;

        let diff = buffer.calculate_state_diff(&changed_state, &create_test_state());
        assert!(diff.statistics_changed);
        assert!(diff.is_significant());
        assert!(diff.change_score() > 0.0);
    }

    #[test]
    fn test_easing_functions() {
        assert_eq!(EasingFunction::Linear.apply(0.5), 0.5);
        assert!(EasingFunction::EaseIn.apply(0.5) < 0.5);
        assert!(EasingFunction::EaseOut.apply(0.5) > 0.5);

        // Test bounds
        assert_eq!(EasingFunction::Linear.apply(-1.0), 0.0);
        assert_eq!(EasingFunction::Linear.apply(2.0), 1.0);
    }

    #[test]
    fn test_snapshot_creation() {
        let state = create_test_state();
        let snapshot = StateSnapshot::from_state(&state);

        assert_eq!(snapshot.track_count, 0);
        assert_eq!(snapshot.log_count, 0);
    }

    #[test]
    fn test_buffer_stats() {
        let state = create_test_state();
        let config = BufferConfig::default();
        let buffer = StateBuffer::new(state, config);

        let stats = buffer.buffer_stats();
        assert_eq!(stats.pending_transitions, 0);
        assert_eq!(stats.history_size, 1);
        assert!(stats.is_healthy());
    }

    #[test]
    fn test_history_cleanup() {
        let state = create_test_state();
        let mut config = BufferConfig::default();
        config.max_history_size = 2;
        config.history_retention = Duration::from_millis(1);

        let mut buffer = StateBuffer::new(state.clone(), config);

        // Add more snapshots than max size
        for _ in 0..5 {
            buffer.take_snapshot(state.clone());
        }

        // Should be limited to max_history_size
        assert!(buffer.history.len() <= 2);
    }
}