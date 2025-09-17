use std::time::Instant;
use serde::{Serialize, Deserialize};

/// Shared state for progress tracking
#[derive(Debug, Clone)]
pub struct ProgressState {
    pub total_tracks: usize,
    pub processed_tracks: usize,
    pub synced_tracks: usize,
    pub plain_tracks: usize,
    pub missing_tracks: usize,
    pub error_tracks: usize,
    pub current_operation: String,
    pub current_track: Option<String>,
    pub start_time: Instant,
}

impl Default for ProgressState {
    fn default() -> Self {
        Self {
            total_tracks: 0,
            processed_tracks: 0,
            synced_tracks: 0,
            plain_tracks: 0,
            missing_tracks: 0,
            error_tracks: 0,
            current_operation: String::new(),
            current_track: None,
            start_time: Instant::now(),
        }
    }
}

impl ProgressState {
    pub fn new(total: usize) -> Self {
        Self {
            total_tracks: total,
            start_time: Instant::now(),
            ..Default::default()
        }
    }

    /// Get progress percentage (0.0 to 100.0)
    pub fn progress_percentage(&self) -> f64 {
        if self.total_tracks == 0 {
            0.0
        } else {
            (self.processed_tracks as f64 / self.total_tracks as f64) * 100.0
        }
    }

    /// Get success rate percentage
    pub fn success_rate(&self) -> f64 {
        if self.processed_tracks == 0 {
            0.0
        } else {
            let successful = self.synced_tracks + self.plain_tracks;
            (successful as f64 / self.processed_tracks as f64) * 100.0
        }
    }

    /// Get tracks per second
    pub fn tracks_per_second(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.processed_tracks as f64 / elapsed
        } else {
            0.0
        }
    }

    /// Check if processing is complete
    pub fn is_complete(&self) -> bool {
        self.processed_tracks >= self.total_tracks
    }
}

/// Result of processing a track
#[derive(Debug, Clone, PartialEq)]
pub enum TrackResult {
    SyncedLyrics,
    PlainLyrics,
    NotFound,
    Error(String),
}

impl TrackResult {
    pub fn is_success(&self) -> bool {
        matches!(self, TrackResult::SyncedLyrics | TrackResult::PlainLyrics)
    }


    pub fn to_string(&self) -> String {
        match self {
            TrackResult::SyncedLyrics => "Synced lyrics".to_string(),
            TrackResult::PlainLyrics => "Plain lyrics".to_string(),
            TrackResult::NotFound => "Not found".to_string(),
            TrackResult::Error(err) => format!("Error: {}", err),
        }
    }
}

/// Final statistics when processing is complete
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalStats {
    pub total_tracks: usize,
    pub synced_tracks: usize,
    pub plain_tracks: usize,
    pub missing_tracks: usize,
    pub error_tracks: usize,
    pub success_rate: f64,
    pub total_duration: std::time::Duration,
    pub tracks_per_second: f64,
}

impl FinalStats {
    pub fn from_state(state: &ProgressState) -> Self {
        Self {
            total_tracks: state.total_tracks,
            synced_tracks: state.synced_tracks,
            plain_tracks: state.plain_tracks,
            missing_tracks: state.missing_tracks,
            error_tracks: state.error_tracks,
            success_rate: state.success_rate(),
            total_duration: state.start_time.elapsed(),
            tracks_per_second: state.tracks_per_second(),
        }
    }

    /// Get total successful downloads
    pub fn successful_tracks(&self) -> usize {
        self.synced_tracks + self.plain_tracks
    }

    /// Get total failed downloads
    pub fn failed_tracks(&self) -> usize {
        self.missing_tracks + self.error_tracks
    }
}