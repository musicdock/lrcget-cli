//! Real-time UI refresh system with rate limiting and smooth updates

use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers};
use futures::StreamExt;

/// Configuration for UI refresh behavior
#[derive(Debug, Clone)]
pub struct RefreshConfig {
    /// Target frames per second for UI updates
    pub target_fps: u32,
    /// Maximum time to wait between forced refreshes
    pub max_refresh_interval: Duration,
    /// Minimum time between refreshes to prevent excessive updates
    pub min_refresh_interval: Duration,
    /// Enable adaptive refresh rate based on activity
    pub adaptive_refresh: bool,
    /// Debounce time for rapid state changes
    pub debounce_duration: Duration,
}

impl Default for RefreshConfig {
    fn default() -> Self {
        Self {
            target_fps: 30,
            max_refresh_interval: Duration::from_millis(100),
            min_refresh_interval: Duration::from_millis(16), // ~60 FPS max
            adaptive_refresh: true,
            debounce_duration: Duration::from_millis(50),
        }
    }
}

impl RefreshConfig {
    /// Create config optimized for high performance
    pub fn high_performance() -> Self {
        Self {
            target_fps: 60,
            max_refresh_interval: Duration::from_millis(50),
            min_refresh_interval: Duration::from_millis(8), // ~120 FPS max
            adaptive_refresh: true,
            debounce_duration: Duration::from_millis(25),
        }
    }

    /// Create config optimized for low resource usage
    pub fn low_resource() -> Self {
        Self {
            target_fps: 15,
            max_refresh_interval: Duration::from_millis(200),
            min_refresh_interval: Duration::from_millis(33), // ~30 FPS max
            adaptive_refresh: true,
            debounce_duration: Duration::from_millis(100),
        }
    }

    /// Calculate target refresh interval
    pub fn target_interval(&self) -> Duration {
        Duration::from_millis(1000 / self.target_fps as u64)
    }
}

/// Types of refresh triggers
#[derive(Debug, Clone, PartialEq)]
pub enum RefreshTrigger {
    /// Periodic refresh based on timer
    Periodic,
    /// State change triggered refresh
    StateChange,
    /// User input triggered refresh
    UserInput,
    /// Forced refresh (highest priority)
    Forced,
    /// Animation frame update
    Animation,
}

/// Refresh event containing trigger type and optional data
#[derive(Debug, Clone)]
pub struct RefreshEvent {
    pub trigger: RefreshTrigger,
    pub timestamp: Instant,
    pub priority: RefreshPriority,
}

/// Priority levels for refresh events
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RefreshPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl RefreshEvent {
    pub fn new(trigger: RefreshTrigger, priority: RefreshPriority) -> Self {
        Self {
            trigger,
            timestamp: Instant::now(),
            priority,
        }
    }

    pub fn periodic() -> Self {
        Self::new(RefreshTrigger::Periodic, RefreshPriority::Low)
    }

    pub fn state_change() -> Self {
        Self::new(RefreshTrigger::StateChange, RefreshPriority::Normal)
    }

    pub fn user_input() -> Self {
        Self::new(RefreshTrigger::UserInput, RefreshPriority::High)
    }

    pub fn forced() -> Self {
        Self::new(RefreshTrigger::Forced, RefreshPriority::Critical)
    }

    pub fn animation() -> Self {
        Self::new(RefreshTrigger::Animation, RefreshPriority::Normal)
    }
}

/// Manages UI refresh timing and rate limiting
pub struct RefreshManager {
    config: RefreshConfig,
    last_refresh: Instant,
    last_state_change: Option<Instant>,
    pending_refresh: Option<RefreshEvent>,
    refresh_sender: mpsc::UnboundedSender<RefreshEvent>,
    frame_count: u64,
    start_time: Instant,

    // Performance tracking
    frame_times: Vec<Duration>,
    max_frame_time_samples: usize,
}

impl RefreshManager {
    /// Create a new refresh manager
    pub fn new(config: RefreshConfig) -> (Self, mpsc::UnboundedReceiver<RefreshEvent>) {
        let (refresh_sender, refresh_receiver) = mpsc::unbounded_channel();

        let manager = Self {
            config,
            last_refresh: Instant::now(),
            last_state_change: None,
            pending_refresh: None,
            refresh_sender,
            frame_count: 0,
            start_time: Instant::now(),
            frame_times: Vec::new(),
            max_frame_time_samples: 100,
        };

        (manager, refresh_receiver)
    }

    /// Request a refresh with the given trigger
    pub fn request_refresh(&mut self, trigger: RefreshTrigger, priority: RefreshPriority) {
        let event = RefreshEvent::new(trigger, priority);

        // Apply rate limiting
        if self.should_throttle_refresh(&event) {
            // Store as pending if it's higher priority than current pending
            if let Some(ref pending) = self.pending_refresh {
                if event.priority > pending.priority {
                    self.pending_refresh = Some(event);
                }
            } else {
                self.pending_refresh = Some(event);
            }
            return;
        }

        // Send the refresh event
        let _ = self.refresh_sender.send(event);
        self.update_refresh_timing();
    }

    /// Check if a refresh should be throttled based on rate limiting
    fn should_throttle_refresh(&self, event: &RefreshEvent) -> bool {
        let now = Instant::now();
        let time_since_last = now.duration_since(self.last_refresh);

        match event.priority {
            RefreshPriority::Critical => false, // Never throttle critical refreshes
            RefreshPriority::High => time_since_last < self.config.min_refresh_interval / 2,
            RefreshPriority::Normal => time_since_last < self.config.min_refresh_interval,
            RefreshPriority::Low => time_since_last < self.config.target_interval(),
        }
    }

    /// Update refresh timing after a refresh occurs
    fn update_refresh_timing(&mut self) {
        self.last_refresh = Instant::now();
        self.frame_count += 1;

        // Process any pending refresh if enough time has passed
        if let Some(pending_event) = self.pending_refresh.take() {
            if !self.should_throttle_refresh(&pending_event) {
                let _ = self.refresh_sender.send(pending_event);
            } else {
                // Put it back if still throttled
                self.pending_refresh = Some(pending_event);
            }
        }
    }

    /// Record frame timing for performance analysis
    pub fn record_frame_time(&mut self, duration: Duration) {
        self.frame_times.push(duration);

        // Keep only recent samples
        if self.frame_times.len() > self.max_frame_time_samples {
            self.frame_times.remove(0);
        }

        // Adapt refresh rate if enabled
        if self.config.adaptive_refresh {
            self.adapt_refresh_rate();
        }
    }

    /// Adapt refresh rate based on performance
    fn adapt_refresh_rate(&mut self) {
        if self.frame_times.len() < 10 {
            return; // Need enough samples
        }

        let avg_frame_time = self.frame_times.iter().sum::<Duration>() / self.frame_times.len() as u32;
        let target_frame_time = self.config.target_interval();

        // If we're consistently over target, reduce refresh rate
        if avg_frame_time > target_frame_time * 2 {
            // Reduce target FPS
            if self.config.target_fps > 15 {
                self.config.target_fps = (self.config.target_fps * 8 / 10).max(15);
            }
        }
        // If we're consistently under target, we could increase rate (but be conservative)
        else if avg_frame_time < target_frame_time / 2 && self.config.target_fps < 60 {
            self.config.target_fps = (self.config.target_fps * 11 / 10).min(60);
        }
    }

    /// Get current performance metrics
    pub fn performance_metrics(&self) -> RefreshMetrics {
        let current_fps = if self.start_time.elapsed().as_secs() > 0 {
            self.frame_count as f64 / self.start_time.elapsed().as_secs_f64()
        } else {
            0.0
        };

        let avg_frame_time = if !self.frame_times.is_empty() {
            self.frame_times.iter().sum::<Duration>() / self.frame_times.len() as u32
        } else {
            Duration::ZERO
        };

        let max_frame_time = self.frame_times.iter().max().copied().unwrap_or(Duration::ZERO);

        RefreshMetrics {
            current_fps,
            target_fps: self.config.target_fps,
            avg_frame_time,
            max_frame_time,
            total_frames: self.frame_count,
            dropped_frames: 0, // Would need to track this separately
        }
    }

    /// Start the refresh timer task
    pub fn start_refresh_timer(sender: mpsc::UnboundedSender<RefreshEvent>, config: RefreshConfig) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.target_interval());

            loop {
                interval.tick().await;
                let _ = sender.send(RefreshEvent::periodic());
            }
        });
    }

    /// Handle state changes with debouncing
    pub fn handle_state_change(&mut self) {
        let now = Instant::now();

        // Update last state change time
        self.last_state_change = Some(now);

        // If we have a recent state change, wait for debounce period
        if let Some(last_change) = self.last_state_change {
            if now.duration_since(last_change) < self.config.debounce_duration {
                // Schedule a debounced refresh
                let sender = self.refresh_sender.clone();
                let debounce_duration = self.config.debounce_duration;

                tokio::spawn(async move {
                    tokio::time::sleep(debounce_duration).await;
                    let _ = sender.send(RefreshEvent::state_change());
                });
                return;
            }
        }

        // Immediate refresh for state changes
        self.request_refresh(RefreshTrigger::StateChange, RefreshPriority::Normal);
    }

    /// Force an immediate refresh
    pub fn force_refresh(&mut self) {
        self.request_refresh(RefreshTrigger::Forced, RefreshPriority::Critical);
    }

    /// Check if we should refresh based on max interval
    pub fn should_force_periodic_refresh(&self) -> bool {
        self.last_refresh.elapsed() > self.config.max_refresh_interval
    }
}

/// Performance metrics for the refresh system
#[derive(Debug, Clone)]
pub struct RefreshMetrics {
    pub current_fps: f64,
    pub target_fps: u32,
    pub avg_frame_time: Duration,
    pub max_frame_time: Duration,
    pub total_frames: u64,
    pub dropped_frames: u64,
}

impl RefreshMetrics {
    /// Check if performance is acceptable
    pub fn is_performance_good(&self) -> bool {
        // Performance is good if we're close to target FPS and frame times are reasonable
        let fps_ratio = self.current_fps / self.target_fps as f64;
        let frame_time_acceptable = self.avg_frame_time < Duration::from_millis(50);

        fps_ratio > 0.8 && frame_time_acceptable
    }

    /// Get performance status as a string
    pub fn status_string(&self) -> String {
        if self.is_performance_good() {
            "Good".to_string()
        } else if self.current_fps < self.target_fps as f64 * 0.5 {
            "Poor".to_string()
        } else {
            "Fair".to_string()
        }
    }
}

/// Event loop for handling terminal events and UI refreshes
pub struct EventLoop {
    refresh_manager: RefreshManager,
    refresh_receiver: mpsc::UnboundedReceiver<RefreshEvent>,
    event_sender: mpsc::UnboundedSender<Event>,
    should_quit: Arc<Mutex<bool>>,
}

impl EventLoop {
    /// Create a new event loop
    pub fn new(config: RefreshConfig) -> Self {
        let (refresh_manager, refresh_receiver) = RefreshManager::new(config.clone());
        let (event_sender, _) = mpsc::unbounded_channel();
        let should_quit = Arc::new(Mutex::new(false));

        // Start the refresh timer
        RefreshManager::start_refresh_timer(refresh_manager.refresh_sender.clone(), config);

        Self {
            refresh_manager,
            refresh_receiver,
            event_sender,
            should_quit,
        }
    }

    /// Start the event loop (non-blocking)
    pub async fn run<F>(&mut self, mut on_event: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(Event) -> Result<bool, Box<dyn std::error::Error>>,
    {
        let mut event_stream = EventStream::new();

        loop {
            tokio::select! {
                // Handle terminal events
                maybe_event = event_stream.next() => {
                    if let Some(Ok(event)) = maybe_event {
                        // Handle quit conditions
                        if let Event::Key(key) = &event {
                            if self.should_quit_on_key(key) {
                                break;
                            }
                        }

                        // Trigger refresh for user input
                        self.refresh_manager.request_refresh(
                            RefreshTrigger::UserInput,
                            RefreshPriority::High
                        );

                        // Call the event handler
                        if on_event(event)? {
                            break;
                        }
                    }
                }

                // Handle refresh events
                Some(refresh_event) = self.refresh_receiver.recv() => {
                    // Create a synthetic refresh event
                    let refresh_event_wrapper = Event::Key(KeyEvent::new(
                        KeyCode::F(1), // Use F1 as a marker for refresh events
                        KeyModifiers::NONE
                    ));

                    if on_event(refresh_event_wrapper)? {
                        break;
                    }
                }

                // Force periodic refresh if needed
                _ = tokio::time::sleep(self.refresh_manager.config.max_refresh_interval) => {
                    if self.refresh_manager.should_force_periodic_refresh() {
                        self.refresh_manager.force_refresh();
                    }
                }
            }

            // Check quit condition
            if *self.should_quit.lock().unwrap() {
                break;
            }
        }

        Ok(())
    }

    /// Check if we should quit on this key event
    fn should_quit_on_key(&self, key: &KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::NONE,
                ..
            } |
            KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } |
            KeyEvent {
                code: KeyCode::Esc,
                ..
            }
        )
    }

    /// Request to quit the event loop
    pub fn request_quit(&self) {
        *self.should_quit.lock().unwrap() = true;
    }

    /// Get refresh manager for manual control
    pub fn refresh_manager(&mut self) -> &mut RefreshManager {
        &mut self.refresh_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refresh_config_creation() {
        let config = RefreshConfig::default();
        assert_eq!(config.target_fps, 30);
        assert!(config.adaptive_refresh);

        let high_perf = RefreshConfig::high_performance();
        assert_eq!(high_perf.target_fps, 60);

        let low_resource = RefreshConfig::low_resource();
        assert_eq!(low_resource.target_fps, 15);
    }

    #[test]
    fn test_refresh_event_priorities() {
        let forced = RefreshEvent::forced();
        let user_input = RefreshEvent::user_input();
        let state_change = RefreshEvent::state_change();
        let periodic = RefreshEvent::periodic();

        assert!(forced.priority > user_input.priority);
        assert!(user_input.priority > state_change.priority);
        assert!(state_change.priority > periodic.priority);
    }

    #[test]
    fn test_refresh_manager_creation() {
        let config = RefreshConfig::default();
        let (manager, _receiver) = RefreshManager::new(config);

        assert_eq!(manager.frame_count, 0);
        assert!(manager.frame_times.is_empty());
    }

    #[test]
    fn test_performance_metrics() {
        let metrics = RefreshMetrics {
            current_fps: 28.5,
            target_fps: 30,
            avg_frame_time: Duration::from_millis(35),
            max_frame_time: Duration::from_millis(50),
            total_frames: 1000,
            dropped_frames: 5,
        };

        assert!(metrics.is_performance_good());
        assert_eq!(metrics.status_string(), "Good");
    }

    #[test]
    fn test_refresh_throttling() {
        let config = RefreshConfig::default();
        let (mut manager, _receiver) = RefreshManager::new(config);

        // First refresh should not be throttled
        let event = RefreshEvent::state_change();
        assert!(!manager.should_throttle_refresh(&event));

        // Update timing to simulate recent refresh
        manager.update_refresh_timing();

        // Immediate second refresh should be throttled for normal priority
        let event = RefreshEvent::state_change();
        assert!(manager.should_throttle_refresh(&event));

        // But critical priority should never be throttled
        let critical_event = RefreshEvent::forced();
        assert!(!manager.should_throttle_refresh(&critical_event));
    }
}