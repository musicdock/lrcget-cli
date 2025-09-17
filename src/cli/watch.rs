use clap::Args;
use anyhow::Result;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::collections::{HashSet, VecDeque};
use std::time::{Duration, Instant};
use tokio::time;
use tracing::{warn};
use futures::{channel::mpsc::Receiver, StreamExt};
use std::io::Write;
use chrono::Utc;

use crate::config::Config;
use crate::core::database::Database;
use crate::core::scanner::Scanner;
use crate::core::lrclib::LyricsDownloader;

#[derive(Debug, Clone)]
pub enum WatchAction {
    FileDetected,
    Scan,
    Download,
    Skip,
}

impl WatchAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            WatchAction::FileDetected => "DETECTED",
            WatchAction::Scan => "SCAN",
            WatchAction::Download => "DOWNLOAD",
            WatchAction::Skip => "SKIP",
        }
    }
}

#[derive(Debug, Clone)]
pub enum WatchStatus {
    Success,
    Failed(String),
    Pending,
    SyncedLyrics,
    PlainLyrics,
    NotFound,
}

impl WatchStatus {
    pub fn as_str(&self) -> &str {
        match self {
            WatchStatus::Success => "✓ Done",
            WatchStatus::Failed(_) => "✗ Fail",
            WatchStatus::Pending => "⏳ Pending",
            WatchStatus::SyncedLyrics => "✓ Sync",
            WatchStatus::PlainLyrics => "✓ Plain",
            WatchStatus::NotFound => "✗ Missing",
        }
    }
}

#[derive(Debug, Clone)]
pub struct WatchActivity {
    pub timestamp: Instant,
    pub action: WatchAction,
    pub file_path: PathBuf,
    pub status: WatchStatus,
    pub details: Option<String>,
}

#[derive(Debug)]
pub struct WatchSession {
    pub start_time: Instant,
    pub files_detected: usize,
    pub files_processed: usize,
    pub downloads_attempted: usize,
    pub downloads_successful: usize,
    pub downloads_failed: usize,
    pub pending_files: usize,
    pub recent_activity: VecDeque<WatchActivity>,
    pub last_batch_time: Option<Instant>,
    pub config: WatchConfig,
}

#[derive(Debug, Clone)]
pub struct WatchConfig {
    pub debounce_seconds: u64,
    pub batch_size: usize,
    pub dry_run: bool,
    pub extensions: Option<Vec<String>>,
    pub directory: PathBuf,
}

impl WatchSession {
    pub fn new(config: WatchConfig) -> Self {
        Self {
            start_time: Instant::now(),
            files_detected: 0,
            files_processed: 0,
            downloads_attempted: 0,
            downloads_successful: 0,
            downloads_failed: 0,
            pending_files: 0,
            recent_activity: VecDeque::with_capacity(20),
            last_batch_time: None,
            config,
        }
    }

    pub fn add_activity(&mut self, activity: WatchActivity) {
        if self.recent_activity.len() >= 20 {
            self.recent_activity.pop_front();
        }
        self.recent_activity.push_back(activity);
    }

    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn success_rate(&self) -> f64 {
        if self.downloads_attempted == 0 {
            0.0
        } else {
            (self.downloads_successful as f64 / self.downloads_attempted as f64) * 100.0
        }
    }
}

fn log_with_timestamp(level: &str, message: &str) {
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S.%6fZ").to_string();
    println!("{}  {} {}", timestamp, level, message);
    let _ = std::io::stdout().flush();
}

fn log_docker_startup(config: &WatchConfig) {
    log_with_timestamp("INFO", &format!(
        "Starting watch mode - Directory: {} | Debounce: {}s | Batch: {} | Dry run: {}",
        config.directory.display(),
        config.debounce_seconds,
        config.batch_size,
        config.dry_run
    ));
}

fn log_docker_file_detected(file_path: &std::path::Path) {
    log_with_timestamp("INFO", &format!("File detected: {}", file_path.display()));
}

fn log_docker_batch_start(count: usize, pending: usize) {
    log_with_timestamp("INFO", &format!("Processing batch: {} new files ({} pending)", count, pending));
}

fn log_docker_file_processing(file_path: &PathBuf) {
    log_with_timestamp("INFO", &format!("Processing file: {}", truncate_path_for_log(file_path)));
}

fn log_docker_session_stats(session: &WatchSession) {
    let elapsed = session.start_time.elapsed();
    log_with_timestamp("INFO", &format!(
        "Session stats: {} files detected, {} processed, {} downloads attempted, {} successful, {} failed - Runtime: {:?}",
        session.files_detected,
        session.files_processed,
        session.downloads_attempted,
        session.downloads_successful,
        session.downloads_failed,
        elapsed
    ));
}

fn truncate_path_for_log(path: &PathBuf) -> String {
    let path_str = path.to_string_lossy();
    if path_str.len() > 60 {
        format!("...{}", &path_str[path_str.len() - 57..])
    } else {
        path_str.to_string()
    }
}

fn is_audio_file(path: &PathBuf, extensions: &Option<Vec<String>>) -> Result<bool> {
    if !path.is_file() {
        return Ok(false);
    }

    let file_extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase());

    match file_extension {
        Some(ext) => {
            if let Some(allowed_extensions) = extensions {
                Ok(allowed_extensions.iter().any(|e| e.to_lowercase() == ext))
            } else {
                // Default audio file extensions
                Ok(matches!(ext.as_str(), "mp3" | "flac" | "wav" | "m4a" | "aac" | "ogg" | "opus" | "wma"))
            }
        }
        None => Ok(false),
    }
}

#[derive(Args)]
pub struct WatchArgs {
    /// Directory to watch for new audio files
    #[arg(value_name = "DIRECTORY")]
    pub directory: PathBuf,

    /// Seconds to wait before processing detected files (debounce)
    #[arg(long, default_value = "10")]
    debounce_seconds: u64,

    /// Maximum number of files to process in one batch
    #[arg(long, default_value = "50")]
    batch_size: usize,

    /// Only watch for specific file extensions (comma-separated)
    #[arg(long, value_delimiter = ',')]
    extensions: Option<Vec<String>>,

    /// Dry run - don't actually download lyrics
    #[arg(long)]
    dry_run: bool,

    /// Scan entire directory on startup before watching
    #[arg(long)]
    initial_scan: bool,
}

pub async fn execute(mut args: WatchArgs, config: &Config) -> Result<()> {
    if !args.directory.exists() {
        anyhow::bail!("Directory does not exist: {}", args.directory.display());
    }

    if !args.directory.is_dir() {
        anyhow::bail!("Path is not a directory: {}", args.directory.display());
    }

    // Validate and clamp parameters
    if args.debounce_seconds < 1 {
        args.debounce_seconds = 1;
    } else if args.debounce_seconds > 3600 {
        args.debounce_seconds = 3600;
    }

    if args.batch_size < 1 {
        args.batch_size = 1;
    } else if args.batch_size > 1000 {
        args.batch_size = 1000;
    }

    // Create watch config
    let watch_config = WatchConfig {
        debounce_seconds: args.debounce_seconds,
        batch_size: args.batch_size,
        dry_run: args.dry_run,
        extensions: args.extensions.clone(),
        directory: args.directory.clone(),
    };

    // Initialize session state
    let mut session = WatchSession::new(watch_config.clone());

    // Setup database and downloader
    let mut db = Database::new(&config.database_path).await?;
    let downloader = LyricsDownloader::from_client(config.create_lrclib_client());

    // Log startup
    log_docker_startup(&watch_config);

    // Perform initial scan if requested
    if args.initial_scan {
        log_with_timestamp("INFO", "Starting initial directory scan");

        let scanner = Scanner::new();
        let scan_results = scanner.scan_directory(&args.directory, &args.extensions).await?;

        session.files_detected += scan_results.len();
        log_with_timestamp("INFO", &format!("Initial scan completed: {} audio files found", scan_results.len()));
    }

    // Setup file watcher
    let (tx, rx) = futures::channel::mpsc::channel(100);
    let mut tx_clone = tx.clone();
    let mut watcher = notify::recommended_watcher(move |result: Result<Event, notify::Error>| {
        if let Err(e) = tx_clone.try_send(result) {
            eprintln!("Failed to send watch event: {}", e);
        }
    })?;
    watcher.watch(&args.directory, RecursiveMode::Recursive)?;

    log_with_timestamp("INFO", "File system watcher started");

    // Main event loop
    process_watch_events(rx, args, config, &mut db, &downloader, &mut session).await
}

async fn process_watch_events(
    mut rx: Receiver<Result<Event, notify::Error>>,
    args: WatchArgs,
    _config: &Config,
    db: &mut Database,
    downloader: &LyricsDownloader,
    session: &mut WatchSession,
) -> Result<()> {
    let mut pending_files = HashSet::new();
    let mut debounce_timer = time::interval(Duration::from_secs(args.debounce_seconds));
    let scanner = Scanner::new();

    loop {
        tokio::select! {
            // Process file system events
            event_result = rx.next() => {
                match event_result {
                    Some(Ok(event)) => {
                        if let Err(e) = handle_fs_event(event, &args, &mut pending_files, session).await {
                            warn!("Error handling file system event: {}", e);
                        }
                    }
                    Some(Err(e)) => {
                        warn!("File system watcher error: {}", e);
                    }
                    None => {
                        log_with_timestamp("INFO", "File system watcher stopped");
                        break;
                    }
                }
            }

            // Process pending files on timer
            _ = debounce_timer.tick() => {
                if !pending_files.is_empty() {
                    let files_to_process: Vec<PathBuf> = pending_files.iter()
                        .take(args.batch_size)
                        .cloned()
                        .collect();

                    // Remove processed files from pending set
                    for file in &files_to_process {
                        pending_files.remove(file);
                    }

                    session.last_batch_time = Some(Instant::now());
                    session.pending_files = pending_files.len();

                    if !files_to_process.is_empty() {
                        log_docker_batch_start(files_to_process.len(), pending_files.len());
                    }

                    // Process each file
                    for file_path in files_to_process {
                        if let Err(e) = process_file(&file_path, &args, db, downloader, &scanner, session).await {
                            log_with_timestamp("ERROR", &format!("Error processing file {}: {}", file_path.display(), e));
                        }
                    }

                    log_docker_session_stats(session);
                }
            }
        }
    }

    Ok(())
}

async fn handle_fs_event(
    event: Event,
    args: &WatchArgs,
    pending_files: &mut HashSet<PathBuf>,
    session: &mut WatchSession,
) -> Result<()> {
    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) => {
            for path in event.paths {
                if is_audio_file(&path, &args.extensions)? {
                    if pending_files.insert(path.clone()) {
                        session.files_detected += 1;
                        log_docker_file_detected(&path);
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

async fn process_file(
    file_path: &PathBuf,
    args: &WatchArgs,
    _db: &mut Database,
    _downloader: &LyricsDownloader,
    scanner: &Scanner,
    session: &mut WatchSession,
) -> Result<()> {
    session.files_processed += 1;

    log_docker_file_processing(file_path);

    if args.dry_run {
        log_with_timestamp("INFO", &format!("SKIP {} (dry run mode)", truncate_path_for_log(file_path)));
        return Ok(());
    }

    // Scan the file for metadata
    match scanner.scan_file(file_path).await {
        Ok(Some(track_metadata)) => {
            // Check if we need to download lyrics
            let has_synced = track_metadata.lrc_lyrics.is_some();
            let has_plain = track_metadata.txt_lyrics.is_some();

            if has_synced {
                log_with_timestamp("INFO", &format!("SKIP {} - Already has synced lyrics", truncate_path_for_log(file_path)));
                return Ok(());
            }

            if has_plain {
                log_with_timestamp("INFO", &format!("SKIP {} - Already has plain lyrics", truncate_path_for_log(file_path)));
                return Ok(());
            }

            // Attempt to download lyrics
            session.downloads_attempted += 1;

            // TODO: Implement actual download logic
            // For now, just log the attempt
            log_with_timestamp("WARN", &format!("NOT_FOUND {} - Download logic placeholder", truncate_path_for_log(file_path)));
        }
        Ok(None) => {
            log_with_timestamp("WARN", &format!("SKIP {} - No readable metadata", truncate_path_for_log(file_path)));
        }
        Err(e) => {
            log_with_timestamp("ERROR", &format!("ERROR {} - Scan failed: {}", truncate_path_for_log(file_path), e));
        }
    }

    Ok(())
}