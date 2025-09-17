use clap::Args;
use tracing::info;

use crate::error::Result;
use crate::services::SimpleServices;
use crate::utils::progress::{ProgressUtils, ProgressMessages};

#[derive(Args)]
pub struct ScanArgs {
    /// Directory to scan (if not specified, scans all configured directories)
    #[arg(value_name = "DIRECTORY")]
    directory: Option<String>,

    /// Rescan all files (ignore existing entries)
    #[arg(short, long)]
    force: bool,
}

pub async fn execute(args: ScanArgs, services: &SimpleServices) -> Result<()> {
    let mut database = services.create_database().await?;
    let scanner = services.create_scanner().await?;

    let directories = if let Some(dir) = args.directory {
        vec![dir]
    } else {
        // TODO: Implement get_directories in DatabaseService
        return Err(crate::error::LrcGetError::Validation(
            "Directory retrieval not yet implemented in new architecture".to_string()
        ));
    };

    if directories.is_empty() {
        return Err(crate::error::LrcGetError::Validation(
            "No directories configured. Run 'lrcget init <directory>' first.".to_string()
        ));
    }

    info!("Scanning directories: {:?}", directories);

    if args.force {
        info!("Forcing rescan of all files...");
        // TODO: Implement clear_tracks in DatabaseService
    }

    // Create progress bar using centralized utility
    let pb = ProgressUtils::create_scanning_spinner();

    let mut total_tracks = 0;

    let num_directories = directories.len();
    
    for directory in directories {
        pb.set_message(ProgressMessages::scanning_directory(&directory));
        
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
                match database.add_track(&track).await {
                    Ok(_) => total_tracks += 1,
                    Err(e) => {
                        tracing::warn!("Failed to add track {}: {}", track.file_path, e);
                    }
                }
            }
            pb.set_message(ProgressMessages::processed_count(total_tracks));
        }
    }

    pb.finish_with_message(ProgressMessages::COMPLETED);
    
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