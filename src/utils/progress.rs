use indicatif::{ProgressBar, ProgressStyle};

/// Centralized progress bar creation utilities
/// Eliminates duplication of progress bar setup patterns across CLI commands
pub struct ProgressUtils;

impl ProgressUtils {
    /// Create a standard spinner for scanning operations
    pub fn create_scanning_spinner() -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {msg}")
                .expect("valid spinner template"),
        );
        pb
    }

    /// Create a progress bar for download operations
    pub fn create_download_progress(total: u64) -> ProgressBar {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
                .expect("valid progress template")
                .progress_chars("#>-"),
        );
        pb
    }

    /// Create a progress bar for batch operations
    pub fn create_batch_progress(total: u64) -> ProgressBar {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("📋 [{elapsed_precise}] [{bar:40.yellow/cyan}] {pos}/{len} {msg}")
                .expect("valid batch template")
                .progress_chars("█▉▊▋▌▍▎▏ "),
        );
        pb
    }

    /// Create a spinner with custom message formatting
    pub fn create_activity_spinner(template: &str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template(template)
                .expect("valid spinner template"),
        );
        pb
    }

    /// Create a simple counter (no bar, just numbers)
    pub fn create_counter() -> ProgressBar {
        let pb = ProgressBar::new(0);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} {pos}")
                .expect("valid counter template"),
        );
        pb
    }
}

/// Common progress bar messages
pub struct ProgressMessages;

impl ProgressMessages {
    pub const SCANNING: &'static str = "Scanning for audio files...";
    pub const DOWNLOADING: &'static str = "Downloading lyrics...";
    pub const PROCESSING: &'static str = "Processing...";
    pub const COMPLETED: &'static str = "✅ Completed";
    pub const FAILED: &'static str = "❌ Failed";

    pub fn scanning_directory(dir: &str) -> String {
        format!("🔍 Scanning: {}", dir)
    }

    pub fn downloading_for(artist: &str, title: &str) -> String {
        format!("⬇️ {} - {}", artist, title)
    }

    pub fn processed_count(count: usize) -> String {
        format!("📊 Processed {} items", count)
    }
}