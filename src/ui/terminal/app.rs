//! Main application coordinator for terminal UI
//!
//! Coordinates between event handling, state management, and rendering
//! to provide a cohesive terminal user interface experience.

use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::Stdout;

use super::{
    state::{AppState, UpdateMessage},
    events::{EventHandler, AppEvent, UpdateType},
    layout::LayoutManager,
    themes::ThemeManager,
    renderer::Renderer,
};

/// Main application modes
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

/// Main terminal application coordinator
pub struct TerminalApp {
    /// Application state
    state: AppState,
    /// Event handler
    event_handler: EventHandler,
    /// Layout manager
    layout_manager: LayoutManager,
    /// Theme manager
    theme_manager: ThemeManager,
    /// Renderer
    renderer: Renderer,
    /// Update channel for external communication
    update_sender: mpsc::UnboundedSender<UpdateMessage>,
    update_receiver: mpsc::UnboundedReceiver<UpdateMessage>,
    /// Application should quit
    should_quit: bool,
    /// Last render time for performance
    last_render: Instant,
    /// Minimum time between renders
    render_interval: Duration,
}

impl TerminalApp {
    /// Create new terminal application
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let state = AppState::new();
        let (event_handler, _event_sender) = EventHandler::new();
        let layout_manager = LayoutManager::new();
        let theme_manager = ThemeManager::new();
        let renderer = Renderer::new(theme_manager.current_theme().clone());

        let (update_sender, update_receiver) = mpsc::unbounded_channel();

        Ok(Self {
            state,
            event_handler,
            layout_manager,
            theme_manager,
            renderer,
            update_sender,
            update_receiver,
            should_quit: false,
            last_render: Instant::now(),
            render_interval: Duration::from_millis(50), // 20 FPS max
        })
    }

    /// Get update sender for external communication
    pub fn update_sender(&self) -> mpsc::UnboundedSender<UpdateMessage> {
        self.update_sender.clone()
    }

    /// Main application loop
    pub async fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), Box<dyn std::error::Error>> {
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(100);

        while !self.should_quit {
            // Handle events
            if let Some(event) = self.event_handler.next_event(Duration::from_millis(10)).await {
                self.handle_event(event).await?;
            }

            // Handle state updates from external sources
            while let Ok(update) = self.update_receiver.try_recv() {
                self.state.update(update);
            }

            // Periodic updates
            if last_tick.elapsed() >= tick_rate {
                self.tick().await?;
                last_tick = Instant::now();
            }

            // Render if needed
            if self.should_render() {
                self.render(terminal)?;
                self.last_render = Instant::now();
            }

            // Small sleep to prevent busy waiting
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        Ok(())
    }

    /// Handle application events
    async fn handle_event(&mut self, event: AppEvent) -> Result<(), Box<dyn std::error::Error>> {
        match event {
            AppEvent::Quit => {
                self.should_quit = true;
            }
            AppEvent::TogglePause => {
                match self.state.mode {
                    super::state::AppMode::Downloading => {
                        self.state.mode = super::state::AppMode::Paused;
                    }
                    super::state::AppMode::Paused => {
                        self.state.mode = super::state::AppMode::Downloading;
                    }
                    _ => {}
                }
            }
            AppEvent::ShowHelp => {
                self.state.mode = super::state::AppMode::Help;
            }
            AppEvent::ShowConfig => {
                self.state.mode = super::state::AppMode::Configuration;
            }
            AppEvent::Resize(width, height) => {
                self.layout_manager.update_size(width, height);
            }
            AppEvent::FocusPanel(panel_id) => {
                self.state.ui_state.focused_panel = Some(panel_id);
            }
            AppEvent::AppUpdate(update_type) => {
                self.handle_app_update(update_type).await?;
            }
            _ => {
                // Handle other events as needed
            }
        }

        Ok(())
    }

    /// Handle updates from background tasks
    async fn handle_app_update(&mut self, update_type: UpdateType) -> Result<(), Box<dyn std::error::Error>> {
        match update_type {
            UpdateType::Progress { current, total } => {
                // Update overall progress
                if let Some(track) = self.state.queue.items.front_mut() {
                    track.progress = if total > 0 {
                        (current as f64) / (total as f64)
                    } else {
                        0.0
                    };
                }
            }
            UpdateType::SongStarted { song, artist } => {
                // Find and update track status
                if let Some(track) = self.state.queue.items.iter_mut()
                    .find(|t| t.title == song && t.artist == artist) {
                    track.status = super::state::TrackStatus::Downloading;
                    track.started_at = Some(std::time::SystemTime::now());
                }
            }
            UpdateType::SongCompleted { song, success, message } => {
                // Find and update track status
                if let Some(track) = self.state.queue.items.iter_mut()
                    .find(|t| t.title == song) {
                    track.status = if success {
                        super::state::TrackStatus::Completed
                    } else {
                        super::state::TrackStatus::Failed
                    };
                    track.completed_at = Some(std::time::SystemTime::now());
                    track.error_message = message;
                }
            }
            UpdateType::StatsUpdate { songs_per_min, success_rate: _ } => {
                self.state.metrics.update_speed(songs_per_min);
            }
            UpdateType::Error { message } => {
                self.state.logs.add_entry(super::state::LogEntry {
                    timestamp: std::time::SystemTime::now(),
                    level: super::state::LogLevel::Error,
                    message,
                    context: None,
                });
            }
        }

        Ok(())
    }

    /// Periodic tick for updates
    async fn tick(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Update performance metrics
        self.state.metrics.update();

        // Update UI state
        self.state.ui_state.last_user_interaction =
            if self.event_handler.time_since_last_input() < Duration::from_secs(1) {
                Instant::now()
            } else {
                self.state.ui_state.last_user_interaction
            };

        Ok(())
    }

    /// Check if we should render the UI
    fn should_render(&self) -> bool {
        self.last_render.elapsed() >= self.render_interval
    }

    /// Render the UI
    fn render(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), Box<dyn std::error::Error>> {
        terminal.draw(|frame| {
            let layout = self.layout_manager.calculate_layout(frame.size());
            self.renderer.render(frame, &layout, &self.state);
        })?;

        Ok(())
    }

    /// Check if application should quit
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Get current application state (for external monitoring)
    pub fn state(&self) -> &AppState {
        &self.state
    }

    /// Force a quit
    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

impl Default for TerminalApp {
    fn default() -> Self {
        Self::new().expect("Failed to create terminal app")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let app = TerminalApp::new();
        assert!(app.is_ok());
    }

    #[test]
    fn test_should_quit_default() {
        let app = TerminalApp::new().unwrap();
        assert!(!app.should_quit());
    }

    #[test]
    fn test_quit_functionality() {
        let mut app = TerminalApp::new().unwrap();
        app.quit();
        assert!(app.should_quit());
    }
}