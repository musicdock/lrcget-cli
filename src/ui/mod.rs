pub mod docker_ui;
pub mod terminal_ui;
pub mod progress_state;

use async_trait::async_trait;
use std::time::Instant;
use crate::ui::progress_state::{ProgressState, TrackResult, FinalStats};

/// Detect the UI mode based on environment
#[derive(Debug, Clone, PartialEq)]
pub enum UiMode {
    Docker,    // Structured logging for containers/CI
    Terminal,  // Interactive TUI for normal terminals
}

impl UiMode {
    pub fn detect() -> Self {
        use tracing::debug;

        // Allow forcing Terminal UI for testing
        if std::env::var("LRCGET_FORCE_TERMINAL_UI").is_ok() {
            debug!("UI Mode: Terminal (forced by LRCGET_FORCE_TERMINAL_UI)");
            return UiMode::Terminal;
        }

        let has_docker = std::env::var("DOCKER").is_ok();
        let has_ci = std::env::var("CI").is_ok();
        let has_github_actions = std::env::var("GITHUB_ACTIONS").is_ok();
        let is_tty = atty::is(atty::Stream::Stdout);

        debug!("UI Mode detection - DOCKER: {}, CI: {}, GITHUB_ACTIONS: {}, TTY: {}",
               has_docker, has_ci, has_github_actions, is_tty);

        // Check if we're in Docker/CI environment
        if has_docker || has_ci || has_github_actions || !is_tty {
            debug!("UI Mode: Docker (non-interactive environment detected)");
            UiMode::Docker
        } else {
            debug!("UI Mode: Terminal (interactive TTY detected)");
            UiMode::Terminal
        }
    }
}

/// Common interface for both UI modes
#[async_trait]
pub trait ProgressInterface: Send + Sync {
    /// Initialize the progress display
    async fn start(&mut self, total: usize);

    /// Update the progress with current state
    async fn update_progress(&mut self, state: &ProgressState);

    /// Update the progress with current state and optional controls text
    async fn update_progress_with_controls(&mut self, state: &ProgressState, controls_text: Option<&str>) {
        // Default implementation just calls update_progress
        self.update_progress(state).await;
    }

    /// Set the current operation being performed
    async fn set_operation(&mut self, operation: String);

    /// Report that a track has been completed
    async fn track_completed(&mut self, track: &str, result: TrackResult);

    /// Finish and show final statistics
    async fn finish(&mut self, final_stats: &FinalStats);

    /// Handle errors during processing
    async fn handle_error(&mut self, track: &str, error: &str);
}

/// Factory function to create the appropriate UI interface
pub fn create_progress_interface() -> Box<dyn ProgressInterface> {
    match UiMode::detect() {
        UiMode::Docker => {
            Box::new(docker_ui::DockerUi::new())
        }
        UiMode::Terminal => {
            Box::new(terminal_ui::TerminalUi::new())
        }
    }
}

/// Utility function to format duration
pub fn format_duration(duration: std::time::Duration) -> String {
    let total_secs = duration.as_secs();
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

/// Calculate ETA based on current progress
pub fn calculate_eta(start_time: Instant, processed: usize, total: usize) -> Option<std::time::Duration> {
    if processed == 0 {
        return None;
    }

    let elapsed = start_time.elapsed();
    let rate = processed as f64 / elapsed.as_secs_f64();
    let remaining = total - processed;

    if rate > 0.0 {
        Some(std::time::Duration::from_secs_f64(remaining as f64 / rate))
    } else {
        None
    }
}