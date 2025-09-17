use clap::Args;
use anyhow::Result;
use std::path::Path;
use tracing::{info, warn};

use crate::config::Config;
use crate::core::scanner::Track;
use crate::core::lrclib::LrclibClient;
use crate::core::lyrics::LyricsManager;

#[derive(Args)]
pub struct FetchArgs {
    /// Path to the media file
    #[arg(value_name = "FILE_PATH")]
    file_path: String,

    /// Only download synced lyrics
    #[arg(long)]
    synced_only: bool,

    /// Force overwrite existing lyrics
    #[arg(long)]
    force: bool,

    /// Show what would be done without actually downloading
    #[arg(long)]
    dry_run: bool,
}

pub async fn execute(args: FetchArgs, config: &Config) -> Result<()> {
    let file_path = Path::new(&args.file_path);
    
    // Check if file exists
    if !file_path.exists() {
        anyhow::bail!("File not found: {}", args.file_path);
    }

    // Check if file is a supported audio format
    let extension = file_path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase());
    
    let supported_formats = vec!["mp3", "m4a", "flac", "ogg", "opus", "wav"];
    
    match extension {
        Some(ext) if supported_formats.contains(&ext.as_str()) => {
            info!("📁 Processing audio file: {}", args.file_path);
        },
        _ => {
            anyhow::bail!("Unsupported file format. Supported formats: {}", 
                         supported_formats.join(", "));
        }
    }

    // Extract metadata from the audio file
    info!("🎵 Extracting metadata...");
    let track = match Track::new_from_path(file_path) {
        Ok(track) => track,
        Err(e) => {
            anyhow::bail!("Failed to extract metadata from file: {}", e);
        }
    };

    info!("🎤 Artist: {}", track.artist);
    info!("🎶 Title: {}", track.title);
    info!("💿 Album: {}", track.album);
    info!("⏱️  Duration: {:.1}s", track.duration);

    // Check if lyrics already exist
    if !args.force {
        if track.lrc_lyrics.is_some() {
            println!("⚠️  Synced lyrics already exist. Use --force to overwrite.");
            return Ok(());
        }
        if track.txt_lyrics.is_some() && !args.synced_only {
            println!("⚠️  Plain lyrics already exist. Use --force to overwrite.");
            return Ok(());
        }
    }

    // Search for lyrics
    info!("🔍 Searching for lyrics...");
    let client = config.create_lrclib_client();
    
    // Try exact match first with rounded duration
    let rounded_duration = track.duration.round();
    let lyrics = client.get_lyrics(
        &track.title,
        &track.artist,
        &track.album,
        rounded_duration,
    ).await?;

    if let Some(lyrics_data) = lyrics {
        if args.dry_run {
            println!("🔍 DRY RUN - would download:");
            if let Some(_) = &lyrics_data.synced_lyrics {
                println!("  ✅ Synced lyrics available");
            }
            if let Some(_) = &lyrics_data.plain_lyrics {
                println!("  ✅ Plain lyrics available");
            }
            if lyrics_data.instrumental {
                println!("  🎼 Track marked as instrumental");
            }
            return Ok(());
        }

        // Check filters
        if args.synced_only && lyrics_data.synced_lyrics.is_none() {
            println!("❌ No synced lyrics found (--synced-only specified)");
            return Ok(());
        }

        if lyrics_data.instrumental {
            info!("🎼 Track is marked as instrumental");
            let lyrics_manager = LyricsManager::new();
            lyrics_manager.save_instrumental(&track.file_path)?;
            println!("✅ Saved instrumental marker");
            return Ok(());
        }

        // Save lyrics
        let lyrics_manager = LyricsManager::new();
        lyrics_manager.save_lyrics_for_file(
            &track.file_path,
            lyrics_data.plain_lyrics.as_deref(),
            lyrics_data.synced_lyrics.as_deref(),
            false,
        ).await?;

        if let Some(_) = &lyrics_data.synced_lyrics {
            println!("✅ Saved synced lyrics (.lrc)");
        } else if let Some(_) = &lyrics_data.plain_lyrics {
            println!("✅ Saved plain lyrics (.txt)");
        }
    } else {
        println!("❌ No exact match found, trying search...");
        
        // Fallback to search
        let search_results = client.search(
            &track.title,
            &track.artist,
            &track.album,
            "",
        ).await?;

        if search_results.is_empty() {
            println!("❌ No lyrics found for this track");
            return Ok(());
        }

        // Filter results
        let mut filtered_results = search_results;
        if args.synced_only {
            filtered_results.retain(|r| r.synced_lyrics.is_some());
        }

        if filtered_results.is_empty() {
            println!("❌ No lyrics found matching criteria");
            return Ok(());
        }

        println!("🎯 Found {} potential match(es):", filtered_results.len());
        
        // Find best match using the same algorithm from search.rs
        let best_match = find_best_match(&track, &filtered_results);
        
        if let Some(search_result) = best_match {
            println!("🎵 Best match: {} - {} ({})", 
                search_result.artist_name.as_deref().unwrap_or("Unknown"),
                search_result.name.as_deref().unwrap_or("Unknown"),
                search_result.album_name.as_deref().unwrap_or("Unknown")
            );

            if args.dry_run {
                println!("🔍 DRY RUN - would download this match");
                return Ok(());
            }

            let lyrics_manager = LyricsManager::new();
            lyrics_manager.save_lyrics_for_file(
                &track.file_path,
                search_result.plain_lyrics.as_deref(),
                search_result.synced_lyrics.as_deref(),
                search_result.instrumental,
            ).await?;

            if search_result.synced_lyrics.is_some() {
                println!("✅ Saved synced lyrics (.lrc) from search result");
            } else if search_result.plain_lyrics.is_some() {
                println!("✅ Saved plain lyrics (.txt) from search result");
            }
        } else {
            println!("❌ No suitable match found in search results");
        }
    }

    Ok(())
}

fn find_best_match<'a>(track: &Track, results: &'a [crate::core::lrclib::SearchResult]) -> Option<&'a crate::core::lrclib::SearchResult> {
    let mut best_score = 0.0;
    let mut best_match = None;
    
    for result in results {
        let mut score = 0.0;
        
        // Title similarity (most important)
        if let Some(result_title) = &result.name {
            if result_title.to_lowercase() == track.title.to_lowercase() {
                score += 3.0;
            } else if result_title.to_lowercase().contains(&track.title.to_lowercase()) 
                   || track.title.to_lowercase().contains(&result_title.to_lowercase()) {
                score += 2.0;
            }
        }
        
        // Artist similarity
        if let Some(result_artist) = &result.artist_name {
            if result_artist.to_lowercase() == track.artist.to_lowercase() {
                score += 2.0;
            } else if result_artist.to_lowercase().contains(&track.artist.to_lowercase()) 
                   || track.artist.to_lowercase().contains(&result_artist.to_lowercase()) {
                score += 1.0;
            }
        }
        
        // Album similarity
        if let Some(result_album) = &result.album_name {
            if result_album.to_lowercase() == track.album.to_lowercase() {
                score += 1.0;
            } else if result_album.to_lowercase().contains(&track.album.to_lowercase()) 
                   || track.album.to_lowercase().contains(&result_album.to_lowercase()) {
                score += 0.5;
            }
        }
        
        // Duration similarity (if available)
        if let Some(result_duration) = result.duration {
            let duration_diff = (result_duration - track.duration).abs();
            if duration_diff < 5.0 { // Within 5 seconds
                score += 0.5;
            }
        }
        
        // Prefer results with synced lyrics
        if result.synced_lyrics.is_some() {
            score += 0.2;
        }
        
        if score > best_score {
            best_score = score;
            best_match = Some(result);
        }
    }
    
    // Only return a match if it has at least some basic similarity
    if best_score >= 2.0 {
        best_match
    } else {
        None
    }
}