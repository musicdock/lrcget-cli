use async_trait::async_trait;
use crate::ui::{ProgressInterface, format_duration};
use crate::ui::progress_state::{ProgressState, TrackResult, FinalStats};
use std::io::{self, Write};
use chrono::Utc;

/// Docker/CI-friendly UI that outputs structured logs without ANSI codes
pub struct DockerUi {
    // No state needed for simple logging
}

impl DockerUi {
    pub fn new() -> Self {
        Self {}
    }

    fn log(&self, level: &str, message: &str) {
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S.%6fZ").to_string();
        println!("{}  {} {}", timestamp, level, message);
        let _ = io::stdout().flush();
    }

    fn log_progress(&self, processed: usize, total: usize, percentage: f64) {
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S.%6fZ").to_string();
        println!("{}  INFO {}/{} tracks completed ({:.1}%)", timestamp, processed, total, percentage);
        let _ = io::stdout().flush();
    }

    fn log_operation(&self, operation: &str) {
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S.%6fZ").to_string();
        println!("{}  INFO {}", timestamp, operation);
        let _ = io::stdout().flush();
    }

    fn log_result(&self, track: &str, result: &TrackResult) {
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S.%6fZ").to_string();
        let (level, message) = match result {
            TrackResult::SyncedLyrics => ("INFO", format!("Downloaded synced lyrics for {}", track)),
            TrackResult::PlainLyrics => ("INFO", format!("Downloaded plain lyrics for {}", track)),
            TrackResult::NotFound => ("WARN", format!("No lyrics found for {}", track)),
            TrackResult::Error(err) => ("ERROR", format!("Error processing {}: {}", track, err)),
        };
        println!("{}  {} {}", timestamp, level, message);
        let _ = io::stdout().flush();
    }
}

#[async_trait]
impl ProgressInterface for DockerUi {
    async fn start(&mut self, total: usize) {
        self.log("INFO", &format!("Starting lyrics download for {} tracks", total));
    }

    async fn update_progress(&mut self, state: &ProgressState) {
        self.log_progress(state.processed_tracks, state.total_tracks, state.progress_percentage());

        // Log current track if available
        if let Some(ref track) = state.current_track {
            self.log("INFO", &format!("Processing {}/{}: {}",
                state.processed_tracks + 1,
                state.total_tracks,
                track));
        }
    }

    async fn update_progress_with_controls(&mut self, state: &ProgressState, controls_text: Option<&str>) {
        // For Docker UI, we don't display controls but log app state if provided
        self.update_progress(state).await;

        if let Some(controls) = controls_text {
            if controls.contains("PAUSED") {
                self.log("INFO", "Downloads paused by user");
            } else if controls.contains("Finishing current downloads") {
                self.log("INFO", "Graceful shutdown in progress - finishing current downloads");
            }
        }
    }

    async fn set_operation(&mut self, operation: String) {
        self.log_operation(&operation);
    }

    async fn track_completed(&mut self, track: &str, result: TrackResult) {
        self.log_result(track, &result);
    }

    async fn handle_error(&mut self, track: &str, error: &str) {
        self.log("ERROR", &format!("Failed to process {}: {}", track, error));
    }

    async fn finish(&mut self, final_stats: &FinalStats) {
        self.log("INFO", "Download process completed");

        // Log detailed statistics
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S.%6fZ").to_string();
        println!("{}  INFO Total: {} | Synced: {} | Plain: {} | Missing: {} | Errors: {}",
            timestamp,
            final_stats.total_tracks,
            final_stats.synced_tracks,
            final_stats.plain_tracks,
            final_stats.missing_tracks,
            final_stats.error_tracks
        );

        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S.%6fZ").to_string();
        println!("{}  INFO Success Rate: {:.1}% | Speed: {:.1} tracks/sec | Duration: {}",
            timestamp,
            final_stats.success_rate,
            final_stats.tracks_per_second,
            format_duration(final_stats.total_duration)
        );

        // Summary line for easy parsing
        let successful = final_stats.successful_tracks();
        let failed = final_stats.failed_tracks();

        if failed == 0 {
            self.log("INFO", &format!("All {} tracks processed successfully!", successful));
        } else {
            let level = if failed > 0 { "WARN" } else { "INFO" };
            self.log(level, &format!("{} successful, {} failed out of {} total tracks",
                successful, failed, final_stats.total_tracks));
        }

        let _ = io::stdout().flush();
    }
}