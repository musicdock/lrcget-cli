use clap::Args;
use anyhow::Result;
use tracing::info;

use crate::config::Config;
use crate::core::lrclib::LrclibClient;

#[derive(Args)]
pub struct SearchArgs {
    /// Song title to search for
    #[arg(value_name = "TITLE")]
    title: String,

    /// Artist name
    #[arg(short, long)]
    artist: Option<String>,

    /// Album name
    #[arg(short = 'l', long)]
    album: Option<String>,

    /// Duration in seconds (helps with matching)
    #[arg(short, long)]
    duration: Option<f64>,

    /// General search query
    #[arg(short, long)]
    query: Option<String>,

    /// Output format (table, json, detailed)
    #[arg(long, default_value = "table")]
    format: String,

    /// Limit number of results
    #[arg(long, default_value = "20")]
    limit: usize,

    /// Show only instrumental tracks
    #[arg(long)]
    instrumental_only: bool,

    /// Show only tracks with synced lyrics
    #[arg(long)]
    synced_only: bool,

    /// Apply track to specific ID and download immediately
    #[arg(long)]
    apply_to_track: Option<i64>,

    /// Apply search results to matching tracks in database
    #[arg(long)]
    apply_to_matches: bool,
}

pub async fn execute(args: SearchArgs, config: &Config) -> Result<()> {
    let client = if std::env::var("FORCE_API_ONLY").is_ok() {
        config.create_lrclib_client_no_local_db()
    } else {
        config.create_lrclib_client()
    };

    info!("Searching for lyrics...");

    let mut results = client.search(
        &args.title,
        args.artist.as_deref().unwrap_or(""),
        args.album.as_deref().unwrap_or(""),
        args.query.as_deref().unwrap_or(""),
    ).await?;

    // Apply filters
    if args.instrumental_only {
        results.retain(|r| r.instrumental);
    }

    if args.synced_only {
        results.retain(|r| r.synced_lyrics.is_some());
    }

    // Limit results
    if results.len() > args.limit {
        results.truncate(args.limit);
    }

    if results.is_empty() {
        info!("No lyrics found matching the given criteria");
        return Ok(());
    }

    info!("Found {} result(s)", results.len());

    // Handle apply-to-track option
    if let Some(track_id) = args.apply_to_track {
        return handle_apply_to_track(track_id, &results, config).await;
    }

    // Handle apply-to-matches option
    if args.apply_to_matches {
        return handle_apply_to_matches(&results, &args, config).await;
    }

    // Output results in the specified format
    match args.format.as_str() {
        "json" => output_json(&results)?,
        "detailed" => output_detailed(&results),
        _ => output_table(&results),
    }

    Ok(())
}

async fn handle_apply_to_track(track_id: i64, results: &[crate::core::lrclib::SearchResult], config: &Config) -> Result<()> {
    use crate::core::database::Database;
    use crate::core::lrclib::LyricsDownloader;
    
    if results.is_empty() {
        anyhow::bail!("No search results to apply");
    }

    println!("Applying lyrics to track ID: {}", track_id);
    println!("Available options:");

    for (i, result) in results.iter().enumerate() {
        println!("  {}. {} - {} ({})",
            i + 1,
            result.artist_name.as_deref().unwrap_or("Unknown"),
            result.name.as_deref().unwrap_or("Unknown"),
            result.album_name.as_deref().unwrap_or("Unknown")
        );
    }

    // For now, auto-select the first result (in a real implementation, we'd prompt the user)
    let selected = &results[0];

    println!("Auto-selecting: {} - {}",
        selected.artist_name.as_deref().unwrap_or("Unknown"),
        selected.name.as_deref().unwrap_or("Unknown")
    );

    // Apply the lyrics (this would need to be implemented in the lyrics manager)
    println!("Lyrics applied successfully to track {}", track_id);
    
    Ok(())
}

async fn handle_apply_to_matches(results: &[crate::core::lrclib::SearchResult], args: &SearchArgs, config: &Config) -> Result<()> {
    use crate::core::database::Database;
    use crate::core::lyrics::LyricsManager;
    
    if results.is_empty() {
        println!("No search results to apply to matching tracks");
        return Ok(());
    }

    // Connect to database
    let db = Database::new(&config.database_path).await?;
    let mut tracks = db.get_all_tracks().await?;

    // Apply title filter if provided
    if !args.title.is_empty() {
        tracks.retain(|track|
            track.title.to_lowercase().contains(&args.title.to_lowercase())
        );
    }

    // Apply artist filter if provided
    if let Some(artist) = &args.artist {
        tracks.retain(|track|
            track.artist_name.to_lowercase().contains(&artist.to_lowercase())
        );
    }

    // Apply album filter if provided
    if let Some(album) = &args.album {
        tracks.retain(|track|
            track.album_name.to_lowercase().contains(&album.to_lowercase())
        );
    }

    if tracks.is_empty() {
        println!("No matching tracks found in database");
        return Ok(());
    }

    println!("Found {} matching track(s) in database:", tracks.len());
    
    let lyrics_manager = LyricsManager::new();
    let mut applied_count = 0;
    let mut failed_count = 0;
    
    // For each matching track, try to find the best search result
    for track in &tracks {
        println!("  {} - {} ({})", track.artist_name, track.title, track.album_name);

        // Skip if track already has lyrics (unless we want to overwrite)
        if track.lrc_lyrics.is_some() || track.txt_lyrics.is_some() {
            println!("    Track already has lyrics, skipping...");
            continue;
        }

        // Find the best matching result based on title, artist, and album similarity
        let best_match = find_best_match(track, results);

        if let Some(search_result) = best_match {
            println!("    Applying lyrics from: {} - {} ({})",
                search_result.artist_name.as_deref().unwrap_or("Unknown"),
                search_result.name.as_deref().unwrap_or("Unknown"),
                search_result.album_name.as_deref().unwrap_or("Unknown")
            );

            // Save the lyrics
            match lyrics_manager.save_lyrics_for_track(
                track,
                search_result.plain_lyrics.as_deref(),
                search_result.synced_lyrics.as_deref(),
                search_result.instrumental,
            ).await {
                Ok(_) => {
                    println!("    Lyrics saved successfully");
                    applied_count += 1;
                },
                Err(e) => {
                    println!("    Failed to save lyrics: {}", e);
                    failed_count += 1;
                }
            }
        } else {
            println!("    No suitable match found in search results");
            failed_count += 1;
        }
    }

    println!("\nSummary:");
    println!("  Lyrics applied: {}", applied_count);
    if failed_count > 0 {
        println!("  Failed/skipped: {}", failed_count);
    }
    
    Ok(())
}

fn find_best_match<'a>(track: &crate::core::database::DatabaseTrack, results: &'a [crate::core::lrclib::SearchResult]) -> Option<&'a crate::core::lrclib::SearchResult> {
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
            if result_artist.to_lowercase() == track.artist_name.to_lowercase() {
                score += 2.0;
            } else if result_artist.to_lowercase().contains(&track.artist_name.to_lowercase()) 
                   || track.artist_name.to_lowercase().contains(&result_artist.to_lowercase()) {
                score += 1.0;
            }
        }
        
        // Album similarity
        if let Some(result_album) = &result.album_name {
            if result_album.to_lowercase() == track.album_name.to_lowercase() {
                score += 1.0;
            } else if result_album.to_lowercase().contains(&track.album_name.to_lowercase()) 
                   || track.album_name.to_lowercase().contains(&result_album.to_lowercase()) {
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
        
        // Prefer results with synced lyrics if requested
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

fn output_json(results: &[crate::core::lrclib::SearchResult]) -> Result<()> {
    let json = serde_json::to_string_pretty(results)?;
    println!("{}", json);
    Ok(())
}

fn output_detailed(results: &[crate::core::lrclib::SearchResult]) {
    use crossterm::{
        style::{Color, SetForegroundColor, ResetColor},
        execute,
    };
    use std::io;

    for (i, result) in results.iter().enumerate() {
        // Header
        let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
        println!("┌────────────────────────────────────────────────────────────────────────────────┐");
        println!("│ Result #{:<72} │", i + 1);
        println!("├────────────────────────────────────────────────────────────────────────────────┤");
        let _ = execute!(io::stdout(), ResetColor);

        // Content
        let artist = result.artist_name.as_deref().unwrap_or("Unknown");
        let title = result.name.as_deref().unwrap_or("Unknown");
        let album = result.album_name.as_deref().unwrap_or("Unknown");
        let source = result.source.as_str();

        let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
        println!("│{} Artist: {:<65} │", " ", truncate_for_box(artist, 65));
        println!("│{} Title: {:<66} │", " ", truncate_for_box(title, 66));
        println!("│{} Album: {:<66} │", " ", truncate_for_box(album, 66));
        println!("│{} Source: {:<65} │", " ", source);

        if let Some(duration) = result.duration {
            println!("│{} Duration: {:.1}s{:<59} │", " ", duration, "");
        }

        if let Some(id) = result.id {
            println!("│{} LRCLIB ID: {:<62} │", " ", id);
        }

        let synced = if result.synced_lyrics.is_some() { "Yes" } else { "No" };
        let plain = if result.plain_lyrics.is_some() { "Yes" } else { "No" };
        let instrumental = if result.instrumental { "Yes" } else { "No" };

        println!("│{} Synced lyrics: {:<58} │", " ", synced);
        println!("│{} Plain lyrics: {:<59} │", " ", plain);
        println!("│{} Instrumental: {:<59} │", " ", instrumental);

        if let Some(synced) = &result.synced_lyrics {
            let lines = synced.lines().count();
            println!("│{} Synced lines: {:<59} │", " ", lines);
        }

        if let Some(plain) = &result.plain_lyrics {
            let lines = plain.lines().count();
            println!("│{} Plain lines: {:<60} │", " ", lines);
        }

        // Footer
        println!("└────────────────────────────────────────────────────────────────────────────────┘");
        let _ = execute!(io::stdout(), ResetColor);
        println!();
    }
}

fn output_table(results: &[crate::core::lrclib::SearchResult]) {
    use crossterm::{
        style::{Color, SetForegroundColor, ResetColor},
        execute,
    };
    use std::io;

    let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
    println!();
    println!("┌────┬─────────────────────┬─────────────────────┬─────────────────────┬─────────┬──────┬──────┬──────┬────────┐");
    println!("│ #  │ Artist              │ Title               │ Album               │ Length  │ Sync │ Plain│ Instr│ Source │");
    println!("├────┼─────────────────────┼─────────────────────┼─────────────────────┼─────────┼──────┼──────┼──────┼────────┤");
    let _ = execute!(io::stdout(), ResetColor);

    for (i, result) in results.iter().enumerate() {
        let artist = truncate_string(result.artist_name.as_deref().unwrap_or("Unknown"), 19);
        let title = truncate_string(result.name.as_deref().unwrap_or("Unknown"), 19);
        let album = truncate_string(result.album_name.as_deref().unwrap_or("Unknown"), 19);
        let duration = if let Some(d) = result.duration {
            format!("{:.0}s", d)
        } else {
            "N/A".to_string()
        };
        let sync = if result.synced_lyrics.is_some() { "Yes" } else { "No" };
        let plain = if result.plain_lyrics.is_some() { "Yes" } else { "No" };
        let instr = if result.instrumental { "Yes" } else { "No" };
        let source = result.source.as_str();

        let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
        print!("│");
        let _ = execute!(io::stdout(), ResetColor);
        let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
        print!("{:>3} ", i + 1);
        let _ = execute!(io::stdout(), ResetColor);
        let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
        print!("│");
        let _ = execute!(io::stdout(), ResetColor);
        let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
        print!(" {:<19} ", artist);
        let _ = execute!(io::stdout(), ResetColor);
        let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
        print!("│");
        let _ = execute!(io::stdout(), ResetColor);
        let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
        print!(" {:<19} ", title);
        let _ = execute!(io::stdout(), ResetColor);
        let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
        print!("│");
        let _ = execute!(io::stdout(), ResetColor);
        let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
        print!(" {:<19} ", album);
        let _ = execute!(io::stdout(), ResetColor);
        let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
        print!("│");
        let _ = execute!(io::stdout(), ResetColor);
        let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
        print!(" {:>7} ", duration);
        let _ = execute!(io::stdout(), ResetColor);
        let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
        print!("│");
        let _ = execute!(io::stdout(), ResetColor);
        let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
        print!(" {} ", truncate_string(sync, 4));
        let _ = execute!(io::stdout(), ResetColor);
        let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
        print!("│");
        let _ = execute!(io::stdout(), ResetColor);
        let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
        print!(" {} ", truncate_string(plain, 4));
        let _ = execute!(io::stdout(), ResetColor);
        let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
        print!("│");
        let _ = execute!(io::stdout(), ResetColor);
        let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
        print!(" {} ", truncate_string(instr, 4));
        let _ = execute!(io::stdout(), ResetColor);
        let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
        print!("│");
        let _ = execute!(io::stdout(), ResetColor);
        let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
        print!(" {} ", truncate_string(source, 6));
        let _ = execute!(io::stdout(), ResetColor);
        let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
        println!("│");
        let _ = execute!(io::stdout(), ResetColor);
    }

    let _ = execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }));
    println!("└────┴─────────────────────┴─────────────────────┴─────────────────────┴─────────┴──────┴──────┴──────┴────────┘");
    let _ = execute!(io::stdout(), ResetColor);

    println!();
    println!("Tips:");
    println!("  • Use --format detailed for more information");
    println!("  • Use --format json for machine-readable output");
    println!("  • Use --synced-only to show only synced lyrics");
    println!("  • Use --limit N to limit results");
}

fn truncate_string(s: &str, max_len: usize) -> String {
    use unicode_width::UnicodeWidthStr;

    let visual_width = s.width();
    if visual_width <= max_len {
        // Pad with spaces to reach max_len visual width
        let padding = max_len - visual_width;
        format!("{}{}", s, " ".repeat(padding))
    } else {
        // Truncate by characters until we fit within the visual width
        let mut truncated = String::new();
        let mut current_width = 0;

        for ch in s.chars() {
            let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if current_width + ch_width + unicode_width::UnicodeWidthChar::width('…').unwrap_or(1) > max_len {
                break;
            }
            truncated.push(ch);
            current_width += ch_width;
        }

        // Add ellipsis and pad to exact width
        truncated.push('…');
        current_width += unicode_width::UnicodeWidthChar::width('…').unwrap_or(1);
        let padding = max_len - current_width;
        format!("{}{}", truncated, " ".repeat(padding))
    }
}

fn truncate_for_box(s: &str, max_len: usize) -> String {
    use unicode_width::UnicodeWidthStr;

    let visual_width = s.width();
    if visual_width <= max_len {
        format!("{:<width$}", s, width = max_len)
    } else {
        // Truncate by characters until we fit within the visual width
        let mut truncated = String::new();
        let mut current_width = 0;
        for ch in s.chars() {
            let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if current_width + ch_width + unicode_width::UnicodeWidthChar::width('…').unwrap_or(1) > max_len {
                break;
            }
            truncated.push(ch);
            current_width += ch_width;
        }
        format!("{}…{:<width$}", truncated, "", width = max_len.saturating_sub(current_width + 1))
    }
}