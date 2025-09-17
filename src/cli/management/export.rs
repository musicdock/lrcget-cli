use clap::Args;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::Write;
use tracing::info;

use crate::config::Config;
use crate::core::data::database::{Database, DatabaseTrack};

#[derive(Args)]
pub struct ExportArgs {
    /// Output format (json, csv, xml, toml, m3u, txt)
    #[arg(short, long, default_value = "json")]
    format: String,

    /// Output file path
    #[arg(short, long)]
    output: Option<String>,

    /// Export type (library, config, missing-lyrics, stats)
    #[arg(short, long, default_value = "library")]
    export_type: String,

    /// Include lyrics content in export
    #[arg(long)]
    include_lyrics: bool,

    /// Filter by artist
    #[arg(long)]
    artist: Option<String>,

    /// Filter by album
    #[arg(long)]
    album: Option<String>,

    /// Pretty print output
    #[arg(long)]
    pretty: bool,
}

#[derive(Serialize, Deserialize)]
struct LibraryExport {
    metadata: ExportMetadata,
    tracks: Vec<ExportTrack>,
}

#[derive(Serialize, Deserialize)]
struct ExportMetadata {
    exported_at: String,
    tool: String,
    version: String,
    total_tracks: usize,
    tracks_with_lyrics: usize,
    tracks_missing_lyrics: usize,
}

#[derive(Serialize, Deserialize)]
struct ExportTrack {
    file_path: String,
    title: String,
    artist: String,
    album: String,
    duration: f64,
    has_synced_lyrics: bool,
    has_plain_lyrics: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    synced_lyrics: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    plain_lyrics: Option<String>,
}

pub async fn execute(args: ExportArgs, config: &Config) -> Result<()> {
    match args.export_type.as_str() {
        "library" => export_library(&args, config).await,
        "config" => export_config(&args, config).await,
        "missing-lyrics" => export_missing_lyrics(&args, config).await,
        "stats" => export_stats(&args, config).await,
        _ => anyhow::bail!("Unknown export type: {}. Available: library, config, missing-lyrics, stats", args.export_type),
    }
}

async fn export_library(args: &ExportArgs, config: &Config) -> Result<()> {
    info!("📊 Exporting library data...");
    
    let db = Database::new(&config.database_path).await?;
    let mut tracks = db.get_all_tracks().await?;

    // Apply filters
    if let Some(artist) = &args.artist {
        tracks.retain(|t| t.artist_name.to_lowercase().contains(&artist.to_lowercase()));
    }
    
    if let Some(album) = &args.album {
        tracks.retain(|t| t.album_name.to_lowercase().contains(&album.to_lowercase()));
    }

    let export_tracks: Vec<ExportTrack> = tracks.iter().map(|track| {
        ExportTrack {
            file_path: track.file_path.clone(),
            title: track.title.clone(),
            artist: track.artist_name.clone(),
            album: track.album_name.clone(),
            duration: track.duration,
            has_synced_lyrics: track.lrc_lyrics.is_some(),
            has_plain_lyrics: track.txt_lyrics.is_some(),
            synced_lyrics: if args.include_lyrics { track.lrc_lyrics.clone() } else { None },
            plain_lyrics: if args.include_lyrics { track.txt_lyrics.clone() } else { None },
        }
    }).collect();

    let tracks_with_lyrics = export_tracks.iter()
        .filter(|t| t.has_synced_lyrics || t.has_plain_lyrics)
        .count();

    let export_data = LibraryExport {
        metadata: ExportMetadata {
            exported_at: chrono::Utc::now().to_rfc3339(),
            tool: "lrcget-cli".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            total_tracks: export_tracks.len(),
            tracks_with_lyrics,
            tracks_missing_lyrics: export_tracks.len() - tracks_with_lyrics,
        },
        tracks: export_tracks,
    };

    let output = match args.format.as_str() {
        "json" => {
            if args.pretty {
                serde_json::to_string_pretty(&export_data)?
            } else {
                serde_json::to_string(&export_data)?
            }
        },
        "csv" => export_to_csv(&export_data)?,
        "xml" => export_to_xml(&export_data)?,
        "toml" => toml::to_string_pretty(&export_data)?,
        "m3u" => export_to_m3u(&export_data),
        "txt" => export_to_txt(&export_data),
        _ => anyhow::bail!("Unsupported format: {}. Available: json, csv, xml, toml, m3u, txt", args.format),
    };

    if let Some(output_path) = &args.output {
        let mut file = File::create(output_path)?;
        file.write_all(output.as_bytes())?;
        info!("✅ Exported {} tracks to {}", export_data.tracks.len(), output_path);
    } else {
        println!("{}", output);
    }

    Ok(())
}

async fn export_config(args: &ExportArgs, config: &Config) -> Result<()> {
    info!("⚙️ Exporting configuration...");
    
    let output = match args.format.as_str() {
        "json" => {
            if args.pretty {
                serde_json::to_string_pretty(config)?
            } else {
                serde_json::to_string(config)?
            }
        },
        "toml" => toml::to_string_pretty(config)?,
        _ => anyhow::bail!("Config export supports only json and toml formats"),
    };

    if let Some(output_path) = &args.output {
        let mut file = File::create(output_path)?;
        file.write_all(output.as_bytes())?;
        info!("✅ Configuration exported to {}", output_path);
    } else {
        println!("{}", output);
    }

    Ok(())
}

async fn export_missing_lyrics(args: &ExportArgs, config: &Config) -> Result<()> {
    info!("🔍 Exporting tracks missing lyrics...");
    
    let db = Database::new(&config.database_path).await?;
    let tracks = db.get_all_tracks().await?;
    
    let missing_tracks: Vec<&DatabaseTrack> = tracks.iter()
        .filter(|t| t.lrc_lyrics.is_none() && t.txt_lyrics.is_none())
        .collect();

    info!("Found {} tracks missing lyrics", missing_tracks.len());

    let output = match args.format.as_str() {
        "json" => serde_json::to_string_pretty(&missing_tracks)?,
        "csv" => {
            let mut csv = "File Path,Title,Artist,Album,Duration\n".to_string();
            for track in missing_tracks {
                csv.push_str(&format!("{},{},{},{},{}\n",
                    escape_csv(&track.file_path),
                    escape_csv(&track.title),
                    escape_csv(&track.artist_name),
                    escape_csv(&track.album_name),
                    track.duration
                ));
            }
            csv
        },
        "txt" => {
            let mut txt = String::new();
            for track in missing_tracks {
                txt.push_str(&format!("{} - {} ({})\n", 
                    track.artist_name, track.title, track.album_name));
            }
            txt
        },
        _ => anyhow::bail!("Missing lyrics export supports json, csv, and txt formats"),
    };

    if let Some(output_path) = &args.output {
        let mut file = File::create(output_path)?;
        file.write_all(output.as_bytes())?;
        info!("✅ Missing lyrics list exported to {}", output_path);
    } else {
        println!("{}", output);
    }

    Ok(())
}

async fn export_stats(args: &ExportArgs, config: &Config) -> Result<()> {
    info!("📈 Generating statistics...");
    
    let db = Database::new(&config.database_path).await?;
    let tracks = db.get_all_tracks().await?;
    
    #[derive(Serialize)]
    struct Stats {
        total_tracks: usize,
        tracks_with_synced_lyrics: usize,
        tracks_with_plain_lyrics: usize,
        tracks_with_any_lyrics: usize,
        tracks_missing_lyrics: usize,
        coverage_percentage: f64,
        artists_count: usize,
        albums_count: usize,
        total_duration_seconds: f64,
        total_duration_formatted: String,
    }

    let synced_count = tracks.iter().filter(|t| t.lrc_lyrics.is_some()).count();
    let plain_count = tracks.iter().filter(|t| t.txt_lyrics.is_some()).count();
    let any_lyrics_count = tracks.iter().filter(|t| t.lrc_lyrics.is_some() || t.txt_lyrics.is_some()).count();
    let missing_count = tracks.len() - any_lyrics_count;
    let coverage = if tracks.len() > 0 { (any_lyrics_count as f64 / tracks.len() as f64) * 100.0 } else { 0.0 };
    
    let unique_artists: std::collections::HashSet<_> = tracks.iter().map(|t| &t.artist_name).collect();
    let unique_albums: std::collections::HashSet<_> = tracks.iter().map(|t| &t.album_name).collect();
    
    let total_duration: f64 = tracks.iter().map(|t| t.duration).sum();
    let hours = total_duration as u64 / 3600;
    let minutes = (total_duration as u64 % 3600) / 60;
    
    let stats = Stats {
        total_tracks: tracks.len(),
        tracks_with_synced_lyrics: synced_count,
        tracks_with_plain_lyrics: plain_count,
        tracks_with_any_lyrics: any_lyrics_count,
        tracks_missing_lyrics: missing_count,
        coverage_percentage: coverage,
        artists_count: unique_artists.len(),
        albums_count: unique_albums.len(),
        total_duration_seconds: total_duration,
        total_duration_formatted: format!("{}h {}m", hours, minutes),
    };

    let output = match args.format.as_str() {
        "json" => {
            if args.pretty {
                serde_json::to_string_pretty(&stats)?
            } else {
                serde_json::to_string(&stats)?
            }
        },
        "txt" => {
            format!(
                "📊 Library Statistics\n\
                ═════════════════════\n\
                🎵 Total Tracks: {}\n\
                🎤 Unique Artists: {}\n\
                💿 Unique Albums: {}\n\
                ⏱️  Total Duration: {}\n\
                \n\
                📝 Lyrics Coverage\n\
                ══════════════════\n\
                🎼 Synced Lyrics: {}\n\
                📄 Plain Lyrics: {}\n\
                ✅ Any Lyrics: {}\n\
                ❌ Missing Lyrics: {}\n\
                📈 Coverage: {:.1}%\n",
                stats.total_tracks,
                stats.artists_count,
                stats.albums_count,
                stats.total_duration_formatted,
                stats.tracks_with_synced_lyrics,
                stats.tracks_with_plain_lyrics,
                stats.tracks_with_any_lyrics,
                stats.tracks_missing_lyrics,
                stats.coverage_percentage
            )
        },
        _ => anyhow::bail!("Stats export supports json and txt formats"),
    };

    if let Some(output_path) = &args.output {
        let mut file = File::create(output_path)?;
        file.write_all(output.as_bytes())?;
        info!("✅ Statistics exported to {}", output_path);
    } else {
        println!("{}", output);
    }

    Ok(())
}

fn export_to_csv(data: &LibraryExport) -> Result<String> {
    let mut csv = "File Path,Title,Artist,Album,Duration,Has Synced,Has Plain\n".to_string();
    
    for track in &data.tracks {
        csv.push_str(&format!("{},{},{},{},{},{},{}\n",
            escape_csv(&track.file_path),
            escape_csv(&track.title),
            escape_csv(&track.artist),
            escape_csv(&track.album),
            track.duration,
            track.has_synced_lyrics,
            track.has_plain_lyrics
        ));
    }
    
    Ok(csv)
}

fn export_to_xml(data: &LibraryExport) -> Result<String> {
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<library>\n");
    xml.push_str(&format!("  <metadata>\n"));
    xml.push_str(&format!("    <exported_at>{}</exported_at>\n", data.metadata.exported_at));
    xml.push_str(&format!("    <tool>{}</tool>\n", data.metadata.tool));
    xml.push_str(&format!("    <version>{}</version>\n", data.metadata.version));
    xml.push_str(&format!("    <total_tracks>{}</total_tracks>\n", data.metadata.total_tracks));
    xml.push_str("  </metadata>\n");
    xml.push_str("  <tracks>\n");
    
    for track in &data.tracks {
        xml.push_str("    <track>\n");
        xml.push_str(&format!("      <file_path>{}</file_path>\n", escape_xml(&track.file_path)));
        xml.push_str(&format!("      <title>{}</title>\n", escape_xml(&track.title)));
        xml.push_str(&format!("      <artist>{}</artist>\n", escape_xml(&track.artist)));
        xml.push_str(&format!("      <album>{}</album>\n", escape_xml(&track.album)));
        xml.push_str(&format!("      <duration>{}</duration>\n", track.duration));
        xml.push_str(&format!("      <has_synced_lyrics>{}</has_synced_lyrics>\n", track.has_synced_lyrics));
        xml.push_str(&format!("      <has_plain_lyrics>{}</has_plain_lyrics>\n", track.has_plain_lyrics));
        xml.push_str("    </track>\n");
    }
    
    xml.push_str("  </tracks>\n");
    xml.push_str("</library>\n");
    Ok(xml)
}

fn export_to_m3u(data: &LibraryExport) -> String {
    let mut m3u = "#EXTM3U\n".to_string();
    
    for track in &data.tracks {
        m3u.push_str(&format!("#EXTINF:{},{} - {}\n", 
            track.duration as i32, 
            track.artist, 
            track.title
        ));
        m3u.push_str(&format!("{}\n", track.file_path));
    }
    
    m3u
}

fn export_to_txt(data: &LibraryExport) -> String {
    let mut txt = format!("Library Export - {}\n", data.metadata.exported_at);
    txt.push_str("═══════════════════════════════════════\n\n");
    
    for track in &data.tracks {
        txt.push_str(&format!("{} - {} ({})\n", 
            track.artist, track.title, track.album));
        txt.push_str(&format!("  File: {}\n", track.file_path));
        txt.push_str(&format!("  Duration: {:.1}s\n", track.duration));
        txt.push_str(&format!("  Lyrics: {} {}\n", 
            if track.has_synced_lyrics { "🎼" } else { "  " },
            if track.has_plain_lyrics { "📝" } else { "  " }
        ));
        txt.push_str("\n");
    }
    
    txt
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace("\"", "\"\""))
    } else {
        s.to_string()
    }
}

fn escape_xml(s: &str) -> String {
    s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
}