//! Integration module for connecting terminal UI with download system
//!
//! This module provides the necessary bridge between the terminal UI components
//! and the core download/scan functionality.

use std::collections::VecDeque;
use std::time::SystemTime;
use crossterm::event::Event;

use crate::ui::terminal::{
    input::{InputAction, QueueInputHandler, TrackAction},
    state::{AppState, TrackQueueItem, TrackStatus},
    panels::QueuePanel,
};

/// Integration bridge between UI and download system
#[derive(Debug)]
pub struct DownloadIntegration {
    /// Input handler for queue interactions
    pub input_handler: QueueInputHandler,
    /// Queue panel for display
    pub queue_panel: QueuePanel,
    /// Pending queue actions to be processed
    pub pending_actions: Vec<QueueAction>,
}

/// Actions that can be performed on the download queue
#[derive(Debug, Clone)]
pub enum QueueAction {
    /// Add a new track to the queue
    AddTrack {
        title: String,
        artist: String,
        album: String,
    },
    /// Update track status
    UpdateTrackStatus {
        track_id: u64,
        status: TrackStatus,
        progress: Option<f64>,
        error: Option<String>,
    },
    /// Remove track from queue
    RemoveTrack { track_id: u64 },
    /// Retry failed track
    RetryTrack { track_id: u64 },
    /// Skip track
    SkipTrack { track_id: u64 },
    /// Clear completed tracks
    ClearCompleted,
    /// Pause all downloads
    PauseAll,
    /// Resume all downloads
    ResumeAll,
}

impl DownloadIntegration {
    /// Create a new download integration
    pub fn new(queue_panel: QueuePanel) -> Self {
        Self {
            input_handler: QueueInputHandler::new(),
            queue_panel,
            pending_actions: Vec::new(),
        }
    }

    /// Handle input events and return resulting actions
    pub fn handle_input(&mut self, event: &Event, app_state: &AppState) -> Vec<QueueAction> {
        let input_action = self.input_handler.handle_input(event);
        let mut actions = Vec::new();

        match input_action {
            InputAction::Navigate(direction) => {
                self.queue_panel.handle_scroll(direction, app_state.queue.items.len());
            }
            InputAction::ToggleFilter => {
                self.queue_panel.toggle_filter();
            }
            InputAction::UpdateFilter(filter) => {
                self.queue_panel.set_filter(filter);
            }
            InputAction::ClearFilter => {
                self.queue_panel.set_filter(String::new());
            }
            InputAction::RetryTrack(track_id) => {
                actions.push(QueueAction::RetryTrack { track_id });
            }
            InputAction::SkipTrack(track_id) => {
                actions.push(QueueAction::SkipTrack { track_id });
            }
            InputAction::RemoveTrack(track_id) => {
                actions.push(QueueAction::RemoveTrack { track_id });
            }
            InputAction::TogglePause => {
                // Determine whether to pause or resume based on current state
                let has_downloading = app_state.queue.items.iter()
                    .any(|track| track.status == TrackStatus::Downloading);

                if has_downloading {
                    actions.push(QueueAction::PauseAll);
                } else {
                    actions.push(QueueAction::ResumeAll);
                }
            }
            InputAction::ShowDetails(_track_id) => {
                // Show details can be handled at the UI level
                // For now, just log or set a flag
            }
            InputAction::Exit => {
                // Handle at application level
            }
            InputAction::ShowHelp => {
                // Handle at UI level
            }
            InputAction::None => {}
        }

        // Check for track-specific actions
        if let Event::Key(key) = event {
            if let Some(track_action) = self.get_track_action_for_key(key) {
                let filtered_tracks = self.queue_panel.filter_tracks(&app_state.queue.items);
                if let Some(track_id) = self.queue_panel.selected_track_id(&filtered_tracks) {
                    let queue_action = self.convert_track_action(track_id, track_action);
                    actions.push(queue_action);
                }
            }
        }

        actions
    }

    /// Convert key event to track action
    fn get_track_action_for_key(&self, key: &crossterm::event::KeyEvent) -> Option<TrackAction> {
        use crossterm::event::KeyCode;

        if !self.queue_panel.state.is_focused() {
            return None;
        }

        match key.code {
            KeyCode::Char('r') => Some(TrackAction::Retry),
            KeyCode::Char('s') => Some(TrackAction::Skip),
            KeyCode::Char('d') => Some(TrackAction::ShowDetails),
            KeyCode::Delete | KeyCode::Char('x') => Some(TrackAction::Remove),
            _ => None,
        }
    }

    /// Convert track action to queue action
    fn convert_track_action(&self, track_id: u64, action: TrackAction) -> QueueAction {
        match action {
            TrackAction::Retry => QueueAction::RetryTrack { track_id },
            TrackAction::Skip => QueueAction::SkipTrack { track_id },
            TrackAction::Remove => QueueAction::RemoveTrack { track_id },
            TrackAction::ShowDetails => {
                // Details are handled at UI level, return a no-op
                QueueAction::ClearCompleted // Placeholder
            }
        }
    }

    /// Add a new track to the queue
    pub fn add_track(&mut self, title: String, artist: String, album: String) {
        self.pending_actions.push(QueueAction::AddTrack {
            title,
            artist,
            album,
        });
    }

    /// Update track status
    pub fn update_track_status(
        &mut self,
        track_id: u64,
        status: TrackStatus,
        progress: Option<f64>,
        error: Option<String>,
    ) {
        self.pending_actions.push(QueueAction::UpdateTrackStatus {
            track_id,
            status,
            progress,
            error,
        });
    }

    /// Get and clear pending actions
    pub fn take_pending_actions(&mut self) -> Vec<QueueAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Apply queue actions to the app state
    pub fn apply_actions(&self, actions: &[QueueAction], app_state: &mut AppState) {
        for action in actions {
            self.apply_single_action(action, app_state);
        }
    }

    /// Apply a single queue action to the app state
    fn apply_single_action(&self, action: &QueueAction, app_state: &mut AppState) {
        match action {
            QueueAction::AddTrack { title, artist, album } => {
                let track_id = app_state.queue.items.len() as u64 + 1;
                let track = TrackQueueItem {
                    id: track_id,
                    title: title.clone(),
                    artist: artist.clone(),
                    album: album.clone(),
                    status: TrackStatus::Pending,
                    progress: 0.0,
                    error_message: None,
                    download_speed: None,
                    timestamp: SystemTime::now(),
                    started_at: None,
                    completed_at: None,
                };
                app_state.queue.items.push_back(track);
                app_state.queue.total_tracks += 1;
            }
            QueueAction::UpdateTrackStatus { track_id, status, progress, error } => {
                if let Some(track) = app_state.queue.items.iter_mut().find(|t| t.id == *track_id) {
                    track.status = *status;
                    if let Some(prog) = progress {
                        track.progress = *prog;
                    }
                    if let Some(err) = error {
                        track.error_message = Some(err.clone());
                    }

                    // Update timestamps
                    match status {
                        TrackStatus::Downloading => {
                            if track.started_at.is_none() {
                                track.started_at = Some(SystemTime::now());
                            }
                        }
                        TrackStatus::Completed | TrackStatus::Failed | TrackStatus::Skipped => {
                            track.completed_at = Some(SystemTime::now());
                        }
                        _ => {}
                    }
                }
            }
            QueueAction::RemoveTrack { track_id } => {
                app_state.queue.items.retain(|track| track.id != *track_id);
            }
            QueueAction::RetryTrack { track_id } => {
                if let Some(track) = app_state.queue.items.iter_mut().find(|t| t.id == *track_id) {
                    track.status = TrackStatus::Pending;
                    track.progress = 0.0;
                    track.error_message = None;
                    track.started_at = None;
                    track.completed_at = None;
                }
            }
            QueueAction::SkipTrack { track_id } => {
                if let Some(track) = app_state.queue.items.iter_mut().find(|t| t.id == *track_id) {
                    track.status = TrackStatus::Skipped;
                    track.completed_at = Some(SystemTime::now());
                }
            }
            QueueAction::ClearCompleted => {
                app_state.queue.items.retain(|track| {
                    !matches!(track.status, TrackStatus::Completed | TrackStatus::Skipped)
                });
            }
            QueueAction::PauseAll => {
                for track in app_state.queue.items.iter_mut() {
                    if track.status == TrackStatus::Downloading {
                        track.status = TrackStatus::Pending;
                    }
                }
            }
            QueueAction::ResumeAll => {
                for track in app_state.queue.items.iter_mut() {
                    if track.status == TrackStatus::Pending {
                        track.status = TrackStatus::Downloading;
                    }
                }
            }
        }
    }

    /// Set focus on the queue panel
    pub fn set_focus(&mut self, focused: bool) {
        self.queue_panel.set_focus(focused);
    }

    /// Check if queue panel has focus
    pub fn has_focus(&self) -> bool {
        self.queue_panel.state.is_focused()
    }

    /// Render the queue panel
    pub fn render(&mut self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer, app_state: &AppState) {
        self.queue_panel.render_queue(area, buf, &app_state.queue.items);
    }
}

/// Helper trait to check if widget state is focused
trait WidgetStateFocus {
    fn is_focused(&self) -> bool;
}

impl WidgetStateFocus for crate::ui::terminal::widgets::WidgetState {
    fn is_focused(&self) -> bool {
        matches!(self, crate::ui::terminal::widgets::WidgetState::Focused)
    }
}

/// Example integration with download commands
pub struct DownloadSystemBridge {
    integration: DownloadIntegration,
}

impl DownloadSystemBridge {
    /// Create a new download system bridge
    pub fn new(integration: DownloadIntegration) -> Self {
        Self { integration }
    }

    /// Process download events and update UI state
    pub fn process_download_event(&mut self, event: DownloadEvent, app_state: &mut AppState) {
        match event {
            DownloadEvent::TrackStarted { track_id } => {
                self.integration.update_track_status(
                    track_id,
                    TrackStatus::Downloading,
                    Some(0.0),
                    None,
                );
            }
            DownloadEvent::TrackProgress { track_id, progress, speed } => {
                self.integration.update_track_status(
                    track_id,
                    TrackStatus::Downloading,
                    Some(progress),
                    None,
                );

                // Update download speed
                if let Some(track) = app_state.queue.items.iter_mut().find(|t| t.id == track_id) {
                    track.download_speed = speed;
                }
            }
            DownloadEvent::TrackCompleted { track_id } => {
                self.integration.update_track_status(
                    track_id,
                    TrackStatus::Completed,
                    Some(1.0),
                    None,
                );
            }
            DownloadEvent::TrackFailed { track_id, error } => {
                self.integration.update_track_status(
                    track_id,
                    TrackStatus::Failed,
                    None,
                    Some(error),
                );
            }
        }

        // Apply pending actions
        let actions = self.integration.take_pending_actions();
        self.integration.apply_actions(&actions, app_state);
    }
}

/// Events from the download system
#[derive(Debug, Clone)]
pub enum DownloadEvent {
    TrackStarted { track_id: u64 },
    TrackProgress { track_id: u64, progress: f64, speed: Option<f64> },
    TrackCompleted { track_id: u64 },
    TrackFailed { track_id: u64, error: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::terminal::{colors::ColorPalette, panels::QueuePanel};
    use std::collections::VecDeque;

    #[test]
    fn test_download_integration_creation() {
        let colors = ColorPalette::default();
        let queue_panel = QueuePanel::new(colors);
        let integration = DownloadIntegration::new(queue_panel);

        assert!(!integration.has_focus());
        assert!(integration.pending_actions.is_empty());
    }

    #[test]
    fn test_add_track_action() {
        let mut app_state = create_test_app_state();
        let action = QueueAction::AddTrack {
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            album: "Test Album".to_string(),
        };

        let colors = ColorPalette::default();
        let queue_panel = QueuePanel::new(colors);
        let integration = DownloadIntegration::new(queue_panel);

        integration.apply_single_action(&action, &mut app_state);

        assert_eq!(app_state.queue.items.len(), 1);
        assert_eq!(app_state.queue.items[0].title, "Test Song");
        assert_eq!(app_state.queue.items[0].status, TrackStatus::Pending);
    }

    #[test]
    fn test_update_track_status() {
        let mut app_state = create_test_app_state();

        // Add a track first
        let add_action = QueueAction::AddTrack {
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            album: "Test Album".to_string(),
        };

        let colors = ColorPalette::default();
        let queue_panel = QueuePanel::new(colors);
        let integration = DownloadIntegration::new(queue_panel);

        integration.apply_single_action(&add_action, &mut app_state);

        // Update the track status
        let update_action = QueueAction::UpdateTrackStatus {
            track_id: 1,
            status: TrackStatus::Downloading,
            progress: Some(0.5),
            error: None,
        };

        integration.apply_single_action(&update_action, &mut app_state);

        assert_eq!(app_state.queue.items[0].status, TrackStatus::Downloading);
        assert_eq!(app_state.queue.items[0].progress, 0.5);
        assert!(app_state.queue.items[0].started_at.is_some());
    }

    fn create_test_app_state() -> AppState {
        use crate::ui::terminal::state::*;

        AppState {
            mode: AppMode::Downloading,
            queue: TrackQueue {
                items: VecDeque::new(),
                current_index: None,
                total_tracks: 0,
                processed_tracks: 0,
                failed_tracks: 0,
                filter: None,
            },
            metrics: PerformanceMetrics::default(),
            stats: AppStatistics::default(),
            logs: LogBuffer::default(),
            ui_state: UiState::default(),
            config: AppConfig::default(),
        }
    }
}