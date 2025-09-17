use clap::Args;
use anyhow::Result;
use tracing::{info, warn};

use crate::config::Config;
use crate::core::database::Database;
use crate::core::lrclib::LyricsDownloader;
use crate::core::cache::{LyricsCache, LyricsCacheInterface};
use crate::core::hooks::{HookManager, HookEvent, HookContext};
use crate::ui::create_progress_interface;
use crate::ui::progress_state::{ProgressState, TrackResult, FinalStats};
use crate::signal_handler::{SignalHandler, AppState};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};

#[derive(Args)]
pub struct DownloadArgs {
    /// Download lyrics for specific track ID
    #[arg(long)]
    track_id: Option<i64>,

    /// Only download for tracks missing lyrics
    #[arg(long)]
    missing_lyrics: bool,

    /// Filter by artist name
    #[arg(long)]
    artist: Option<String>,

    /// Filter by album name
    #[arg(long)]
    album: Option<String>,

    /// Maximum number of parallel downloads
    #[arg(long, default_value = "4")]
    parallel: usize,

    /// Dry run (don't actually download)
    #[arg(long)]
    dry_run: bool,
}

pub async fn execute(args: DownloadArgs, config: &Config) -> Result<()> {
    let db = Database::new(&config.database_path).await?;

    // Initialize hybrid cache (Redis + File)
    let cache_dir = config.database_path.parent()
        .unwrap_or(&config.database_path)
        .join("cache");
    let cache = Arc::new(RwLock::new(
        LyricsCache::new(cache_dir, config.redis_url.as_deref())?
    ));
    let client = config.create_lrclib_client();
    let _downloader = LyricsDownloader::from_client_with_cache(client, cache.clone());

    // Initialize hooks
    let hooks_config_path = config.database_path.parent()
        .unwrap_or(&config.database_path)
        .join("hooks.toml");
    let mut hook_manager = HookManager::new();
    if let Err(e) = hook_manager.load_from_config(&hooks_config_path) {
        warn!("Failed to load hooks configuration: {}", e);
    }

    // Get tracks to process
    let tracks = if let Some(track_id) = args.track_id {
        vec![db.get_track(track_id).await?]
    } else {
        let mut query_tracks = db.get_all_tracks().await?;

        // Apply filters
        if args.missing_lyrics {
            query_tracks.retain(|t| t.lrc_lyrics.is_none() && t.txt_lyrics.is_none());
        }

        if let Some(artist) = &args.artist {
            query_tracks.retain(|t| t.artist_name.to_lowercase().contains(&artist.to_lowercase()));
        }

        if let Some(album) = &args.album {
            query_tracks.retain(|t| t.album_name.to_lowercase().contains(&album.to_lowercase()));
        }

        query_tracks
    };

    if tracks.is_empty() {
        warn!("No tracks found matching criteria");
        return Ok(());
    }

    info!("Processing {} tracks", tracks.len());

    if args.dry_run {
        info!("DRY RUN - would download lyrics for:");
        for track in &tracks {
            println!("  {} - {} ({})", track.artist_name, track.title, track.album_name);
        }
        return Ok(());
    }

    // Execute pre-download hooks
    let mut metadata = HashMap::new();
    metadata.insert("track_count".to_string(), serde_json::Value::Number(tracks.len().into()));
    metadata.insert("parallel".to_string(), serde_json::Value::Number(args.parallel.into()));

    let context = HookContext {
        event: HookEvent::PreDownload,
        track: None,
        metadata,
    };

    if let Err(e) = hook_manager.execute_hooks(HookEvent::PreDownload, context).await {
        warn!("Pre-download hook execution failed: {}", e);
    }

    // Setup progress tracking and signal handling
    let total = tracks.len();
    let mut ui = create_progress_interface();
    let progress_state = ProgressState::new(total);

    // Initialize signal handler
    let mut signal_handler = SignalHandler::new();
    let (mut signal_receiver, signal_handles) = signal_handler.start_signal_monitoring().await;

    ui.start(total).await;

    // Check if we have Terminal UI for enhanced controls
    let is_terminal_ui = std::env::var("LRCGET_FORCE_TERMINAL_UI").is_ok() ||
                         (!std::env::var("DOCKER").is_ok() &&
                          !std::env::var("CI").is_ok() &&
                          atty::is(atty::Stream::Stdout));

    // Start UI update task with signal monitoring
    let ui_update_signal_handler = signal_handler.clone();
    let ui_update_progress = progress_state.clone();

    // For now, we'll skip the UI update task and handle it differently
    // let _ui_update_task = tokio::spawn(async move { ... });

    let progress_state = Arc::new(Mutex::new(progress_state));
    let ui = Arc::new(Mutex::new(ui));
    let signal_handler = Arc::new(signal_handler);
    let progress_state_final = progress_state.clone();
    let ui_final = ui.clone();
    let cache_shared = cache.clone();
    let config_shared = config.clone();

    // Store signal handler for final cleanup
    let signal_handler_for_final = signal_handler.clone();
    let signal_monitor_task = tokio::spawn(async move {
        while let Some(new_state) = signal_receiver.recv().await {
            match new_state {
                AppState::Stopping => {
                    info!("Graceful shutdown initiated - waiting for active downloads to complete");
                    // The download loop will handle stopping gracefully
                    break;
                }
                AppState::Paused => {
                    info!("Downloads paused - press R to resume");
                }
                AppState::Running => {
                    info!("Downloads resumed");
                }
                _ => {}
            }
        }
    });

    // Download lyrics with async parallel processing and signal handling
    use futures::stream::{self, StreamExt};

    // Create a semaphore to control active downloads during pause/resume
    let semaphore = Arc::new(tokio::sync::Semaphore::new(args.parallel));

    stream::iter(tracks)
        .for_each_concurrent(args.parallel, move |track| {
            let progress_state = progress_state.clone();
            let ui = ui.clone();
            let cache = cache_shared.clone();
            let config = config_shared.clone();
            let signal_handler = signal_handler.clone();
            let semaphore = semaphore.clone();
            let is_terminal = is_terminal_ui;

            async move {
                // Wait for semaphore permit and check if we should proceed
                let _permit = semaphore.acquire().await.unwrap();

                // Check for pause/stop before starting
                loop {
                    match signal_handler.get_state() {
                        AppState::Stopped | AppState::Stopping => {
                            return; // Exit immediately on stop
                        }
                        AppState::Paused => {
                            // Wait while paused
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            continue;
                        }
                        AppState::Running => break, // Proceed with download
                    }
                }
                let track_name = format!("{} - {}", track.artist_name, track.title);

                // Update current operation
                {
                    let mut state = progress_state.lock().await;
                    state.current_operation = format!("Searching {}", track_name);
                    state.current_track = Some(track_name.clone());
                }

                let mut ui_guard = ui.lock().await;
                let progress_guard = progress_state.lock().await;
                ui_guard.update_progress(&*progress_guard).await;
                drop(progress_guard);
                ui_guard.set_operation(format!("Searching {}", track_name)).await;
                drop(ui_guard);

                // Create a fresh client for each task to avoid thread safety issues
                let client = config.create_lrclib_client();
                let downloader = LyricsDownloader::from_client_with_cache(client, cache);

                // Check if we should stop during download
                if signal_handler.get_state() == AppState::Stopping {
                    return;
                }

                let result = match downloader.download_for_track(&track).await {
                    Ok(lyrics_info) => {
                        // Check again after download completes
                        if signal_handler.get_state() == AppState::Stopping {
                            return;
                        }

                        if !lyrics_info.found {
                            TrackResult::NotFound
                        } else if lyrics_info.instrumental {
                            info!("✓ Track is instrumental: {} - {}", track.artist_name, track.title);
                            TrackResult::NotFound // Treat instrumental as not found for stats purposes
                        } else if lyrics_info.synced_lyrics {
                            info!("✓ Downloaded synced lyrics for: {} - {}", track.artist_name, track.title);
                            TrackResult::SyncedLyrics
                        } else if lyrics_info.plain_lyrics {
                            info!("✓ Downloaded plain lyrics for: {} - {}", track.artist_name, track.title);
                            TrackResult::PlainLyrics
                        } else {
                            TrackResult::NotFound
                        }
                    }
                    Err(e) => {
                        warn!("✗ Failed to download lyrics for {}: {}", track.title, e);
                        TrackResult::Error(e.to_string())
                    }
                };

                // Update progress state
                {
                    let mut state = progress_state.lock().await;
                    state.processed_tracks += 1;

                    match result {
                        TrackResult::SyncedLyrics => state.synced_tracks += 1,
                        TrackResult::PlainLyrics => state.plain_tracks += 1,
                        TrackResult::NotFound => state.missing_tracks += 1,
                        TrackResult::Error(_) => state.error_tracks += 1,
                    }
                }

                // Update UI with controls if in terminal mode
                let mut ui_guard = ui.lock().await;
                ui_guard.track_completed(&track_name, result).await;
                let progress_guard = progress_state.lock().await;

                if is_terminal {
                    let controls_text = signal_handler.get_status_text();
                    ui_guard.update_progress_with_controls(&*progress_guard, Some(&controls_text)).await;
                } else {
                    ui_guard.update_progress(&*progress_guard).await;
                }
            }
        })
        .await;

    // Mark signal handler as stopped and clean up tasks
    signal_handler_for_final.shutdown();

    // Disable input monitoring to clean up terminal
    let _ = signal_handler_for_final.disable_input_monitoring();

    // Wait for signal monitoring tasks to complete
    for handle in signal_handles {
        let _ = handle.await;
    }

    drop(signal_monitor_task);

    // Generate final statistics and finish UI
    let final_state = progress_state_final.lock().await;
    let final_stats = FinalStats::from_state(&final_state);

    ui_final.lock().await.finish(&final_stats).await;

    // Save cache index
    if let Err(e) = cache.read().await.save_index().await {
        warn!("Failed to save cache index: {}", e);
    }

    let successful = final_stats.successful_tracks();
    let failed = final_stats.failed_tracks();

    if failed > 0 {
        warn!("Some downloads failed. Use --verbose for detailed error information.");
    } else {
        info!("All downloads completed successfully! 🎉");
    }

    // Execute post-download hooks
    let mut metadata = HashMap::new();
    metadata.insert("successful".to_string(), serde_json::Value::Number(successful.into()));
    metadata.insert("failed".to_string(), serde_json::Value::Number(failed.into()));
    metadata.insert("success_rate".to_string(),
        serde_json::Value::Number(serde_json::Number::from_f64(final_stats.success_rate).unwrap_or_else(|| serde_json::Number::from(0))));

    let context = HookContext {
        event: HookEvent::PostDownload,
        track: None,
        metadata,
    };

    if let Err(e) = hook_manager.execute_hooks(HookEvent::PostDownload, context).await {
        warn!("Post-download hook execution failed: {}", e);
    }

    Ok(())
}
