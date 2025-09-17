use clap::Args;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use tracing::info;

use crate::config::Config;
use crate::core::database::Database;
use crate::core::scanner::{Scanner, Track};

#[derive(Args)]
pub struct ScanArgs {
    /// Directory to scan (if not specified, scans all configured directories)
    #[arg(value_name = "DIRECTORY")]
    directory: Option<String>,

    /// Rescan all files (ignore existing entries)
    #[arg(short, long)]
    force: bool,
}

pub async fn execute(args: ScanArgs, config: &Config) -> Result<()> {
    let mut db = Database::new(&config.database_path).await?;
    let scanner = Scanner::new();

    let directories = if let Some(dir) = args.directory {
        vec![dir]
    } else {
        db.get_directories().await?
    };

    if directories.is_empty() {
        anyhow::bail!("No directories configured. Run 'lrcget init <directory>' first.");
    }

    info!("Scanning directories: {:?}", directories);

    if args.force {
        info!("Forcing rescan of all files...");
        db.clear_tracks().await?;
    }

    // Create progress bar
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed_precise}] {msg}")
            .expect("valid spinner template"),
    );

    let mut total_tracks = 0;

    let num_directories = directories.len();
    
    for directory in directories {
        pb.set_message(format!("Scanning {}", directory));
        
        let tracks = scanner.scan_directory(&std::path::PathBuf::from(&directory), &None).await?;
        
        if tracks.is_empty() {
            pb.set_message(format!("No music files found in {}", directory));
            continue;
        }
        
        pb.set_message(format!("Processing {} tracks from {}", tracks.len(), directory));
        
        // Process tracks in batches for better performance
        let batch_size = 50;
        for chunk in tracks.chunks(batch_size) {
            for track in chunk {
                match db.add_track(&track).await {
                    Ok(_) => total_tracks += 1,
                    Err(e) => {
                        tracing::warn!("Failed to add track {}: {}", track.file_path, e);
                    }
                }
            }
            pb.set_message(format!("Processed {}/{} tracks", total_tracks, tracks.len()));
        }
    }

    pb.finish_with_message(format!("✓ Scanned {} tracks successfully", total_tracks));
    
    println!("\n🎵 Scan Complete!");
    println!("  📁 Directories scanned: {}", num_directories);
    println!("  🎶 Tracks found: {}", total_tracks);
    
    if total_tracks > 0 {
        println!("\n📋 Next steps:");
        println!("  • Run 'lrcget download --missing-lyrics' to download lyrics");
        println!("  • Run 'lrcget config show' to view configuration");
        println!("  • Run 'lrcget search \"Song Title\" --artist \"Artist\"' to test search");
    } else {
        println!("  ⚠️  No music files found. Check that your directories contain supported formats:");
        println!("     MP3, M4A, FLAC, OGG, Opus, WAV");
    }

    Ok(())
}