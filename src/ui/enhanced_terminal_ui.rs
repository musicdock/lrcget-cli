//! Enhanced terminal UI using the full TUI system
//!
//! This module provides a rich, interactive terminal interface that integrates
//! with the new TUI system for enhanced download monitoring and control.

use async_trait::async_trait;
use tokio::sync::mpsc;
use std::collections::VecDeque;
use std::time::{SystemTime, Instant};

use crate::ui::{ProgressInterface, ProgressState, TrackResult, FinalStats};
use crate::ui::terminal::{
    app::{TerminalApp, AppMode as TuiAppMode},
    state::{UpdateMessage, TrackQueueItem, TrackStatus, LogEntry, LogLevel, AppState as TuiAppState},
    init_terminal, restore_terminal
};

/// Enhanced terminal UI that uses the full TUI system
pub struct EnhancedTerminalUi {
    app: Option<TerminalApp>,
    update_sender: Option<mpsc::UnboundedSender<UpdateMessage>>,
    terminal: Option<ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>>,
    current_tracks: VecDeque<TrackQueueItem>,
    next_track_id: u64,
    _app_task: Option<tokio::task::JoinHandle<()>>,
}

impl EnhancedTerminalUi {
    pub fn new() -> Self {
        Self {
            app: None,
            update_sender: None,
            terminal: None,
            current_tracks: VecDeque::new(),
            next_track_id: 1,
            _app_task: None,
        }
    }

    /// Initialize the TUI system
    async fn initialize_tui(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Check if we should use the enhanced UI
        if !crate::ui::terminal::should_enable_terminal_ui() {
            // Fall back to simple terminal UI if TUI is not available
            return Err("Terminal UI not available".into());
        }

        // Initialize terminal
        let terminal = init_terminal()?;
        self.terminal = Some(terminal);

        // Create TUI app
        let mut app = TerminalApp::new()?;
        let update_sender = app.update_sender();
        self.update_sender = Some(update_sender);

        // For now, store the app directly instead of spawning a task
        // TODO: Implement proper async task management in Phase 2
        self.app = Some(app);

        Ok(())
    }

    /// Convert TrackResult to TrackStatus
    fn result_to_status(result: &TrackResult) -> TrackStatus {
        match result {
            TrackResult::SyncedLyrics => TrackStatus::Completed,
            TrackResult::PlainLyrics => TrackStatus::Completed,
            TrackResult::NotFound => TrackStatus::Skipped,
            TrackResult::Error(_) => TrackStatus::Failed,
        }
    }

    /// Send update to TUI app
    fn send_update(&self, message: UpdateMessage) {
        if let Some(ref sender) = self.update_sender {
            if let Err(e) = sender.send(message) {
                tracing::warn!("Failed to send TUI update: {}", e);
            }
        }
    }

    /// Add log entry to TUI
    fn add_log(&self, level: LogLevel, message: String, context: Option<String>) {
        let entry = LogEntry {
            timestamp: SystemTime::now(),
            level,
            message,
            context,
        };
        self.send_update(UpdateMessage::LogAdded(entry));
    }
}

#[async_trait]
impl ProgressInterface for EnhancedTerminalUi {
    async fn start(&mut self, total: usize) {
        // Try to initialize the TUI system
        if let Err(e) = self.initialize_tui().await {
            tracing::debug!("Enhanced TUI not available, falling back: {}", e);
            // This would typically fall back to simple terminal UI
            // For now, we'll just log the issue
            return;
        }

        self.add_log(
            LogLevel::Info,
            format!("Starting download session with {} tracks", total),
            None,
        );

        // Initialize statistics
        let stats = crate::ui::terminal::state::AppStatistics {
            total_processed: 0,
            completed: 0,
            failed: 0,
            skipped: 0,
            synced_lyrics: 0,
            plain_lyrics: 0,
            instrumental: 0,
            session_start: SystemTime::now(),
            last_update: SystemTime::now(),
        };

        self.send_update(UpdateMessage::StatsUpdated(stats));
    }

    async fn update_progress(&mut self, state: &ProgressState) {
        // Update performance metrics
        if state.processed_tracks > 0 && state.start_time.elapsed().as_secs() > 0 {
            let elapsed_secs = state.start_time.elapsed().as_secs_f64();
            let songs_per_minute = (state.processed_tracks as f64 / elapsed_secs) * 60.0;

            self.send_update(UpdateMessage::MetricsUpdated);

            // Calculate speeds and update metrics through the TUI system
            // This would normally be done through a metrics update mechanism
        }

        // Update overall statistics
        let stats = crate::ui::terminal::state::AppStatistics {
            total_processed: state.processed_tracks as u64,
            completed: (state.synced_tracks + state.plain_tracks) as u64,
            failed: state.error_tracks as u64,
            skipped: state.missing_tracks as u64,
            synced_lyrics: state.synced_tracks as u64,
            plain_lyrics: state.plain_tracks as u64,
            instrumental: 0, // Would need to track this separately
            session_start: SystemTime::now() - state.start_time.elapsed(),
            last_update: SystemTime::now(),
        };

        self.send_update(UpdateMessage::StatsUpdated(stats));
    }

    async fn update_progress_with_controls(&mut self, state: &ProgressState, controls_text: Option<&str>) {
        // Update progress normally
        self.update_progress(state).await;

        // Add controls information as a log entry if provided
        if let Some(controls) = controls_text {
            self.add_log(
                LogLevel::Info,
                format!("Controls: {}", controls),
                Some("system".to_string()),
            );
        }
    }

    async fn set_operation(&mut self, operation: String) {
        self.add_log(
            LogLevel::Info,
            operation,
            Some("operation".to_string()),
        );
    }

    async fn track_completed(&mut self, track: &str, result: TrackResult) {
        // Parse track name (format: "Artist - Title")
        let parts: Vec<&str> = track.splitn(2, " - ").collect();
        let (artist, title) = if parts.len() == 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            ("Unknown".to_string(), track.to_string())
        };

        // Find existing track or create new one
        let track_id = if let Some(existing) = self.current_tracks.iter().find(|t| t.artist == artist && t.title == title) {
            existing.id
        } else {
            // Create new track
            let track_id = self.next_track_id;
            self.next_track_id += 1;

            let track_item = TrackQueueItem {
                id: track_id,
                title: title.clone(),
                artist: artist.clone(),
                album: "Unknown".to_string(), // We don't have album info in the track string
                status: TrackStatus::Processing,
                progress: 0.0,
                error_message: None,
                download_speed: None,
                timestamp: SystemTime::now(),
                started_at: Some(SystemTime::now()),
                completed_at: None,
            };

            self.current_tracks.push_back(track_item.clone());
            self.send_update(UpdateMessage::TrackAdded(track_item));
            track_id
        };

        // Update track status
        let status = Self::result_to_status(&result);
        let error_message = match &result {
            TrackResult::Error(msg) => Some(msg.clone()),
            _ => None,
        };

        self.send_update(UpdateMessage::TrackStatusChanged {
            track_id,
            status,
            message: error_message,
        });

        // Add log entry for completion
        let (level, symbol, message) = match result {
            TrackResult::SyncedLyrics => (LogLevel::Info, "✅", "Downloaded synced lyrics".to_string()),
            TrackResult::PlainLyrics => (LogLevel::Info, "✅", "Downloaded plain lyrics".to_string()),
            TrackResult::NotFound => (LogLevel::Warning, "⏭️", "No lyrics found".to_string()),
            TrackResult::Error(ref err) => (LogLevel::Error, "❌", format!("Error: {}", err)),
        };

        self.add_log(
            level,
            format!("{} {} - {}: {}", symbol, artist, title, message),
            Some("download".to_string()),
        );
    }

    async fn finish(&mut self, final_stats: &FinalStats) {
        self.add_log(
            LogLevel::Info,
            format!(
                "Download session completed! Success rate: {:.1}% ({}/{} tracks)",
                final_stats.success_rate,
                final_stats.successful_tracks(),
                final_stats.total_tracks
            ),
            Some("session".to_string()),
        );

        // Give the UI some time to display the final state
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // Restore terminal
        if let Err(e) = restore_terminal() {
            tracing::warn!("Failed to restore terminal: {}", e);
        }
    }

    async fn handle_error(&mut self, track: &str, error: &str) {
        self.add_log(
            LogLevel::Error,
            format!("Error processing {}: {}", track, error),
            Some("error".to_string()),
        );
    }
}

impl Drop for EnhancedTerminalUi {
    fn drop(&mut self) {
        // Ensure terminal is restored on drop
        let _ = restore_terminal();

        // Abort the app task if it's still running
        if let Some(task) = self._app_task.take() {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_to_status_conversion() {
        assert_eq!(
            EnhancedTerminalUi::result_to_status(&TrackResult::SyncedLyrics),
            TrackStatus::Completed
        );
        assert_eq!(
            EnhancedTerminalUi::result_to_status(&TrackResult::PlainLyrics),
            TrackStatus::Completed
        );
        assert_eq!(
            EnhancedTerminalUi::result_to_status(&TrackResult::NotFound),
            TrackStatus::Skipped
        );
        assert_eq!(
            EnhancedTerminalUi::result_to_status(&TrackResult::Error("test".to_string())),
            TrackStatus::Failed
        );
    }

    #[test]
    fn test_enhanced_ui_creation() {
        let ui = EnhancedTerminalUi::new();
        assert!(ui.app.is_none());
        assert!(ui.update_sender.is_none());
    }
}