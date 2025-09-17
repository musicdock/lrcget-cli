use clap::Args;
use anyhow::Result;
use std::path::PathBuf;
use tracing::{info, warn};

use crate::config::Config;
use crate::core::Database;

#[derive(Args)]
pub struct InitArgs {
    /// Music directory to initialize
    #[arg(value_name = "DIRECTORY")]
    directory: PathBuf,

    /// Force re-initialization even if directory is already configured
    #[arg(short, long)]
    force: bool,
}

pub async fn execute(args: InitArgs, config: &Config) -> Result<()> {
    info!("Initializing music library at: {}", args.directory.display());

    // Validate directory exists
    if !args.directory.exists() {
        anyhow::bail!("Directory does not exist: {}", args.directory.display());
    }

    if !args.directory.is_dir() {
        anyhow::bail!("Path is not a directory: {}", args.directory.display());
    }

    // Initialize database
    let mut db = Database::new(&config.database_path).await?;

    // Check if directory is already configured
    let existing_dirs = db.get_directories().await?;
    let dir_str = args.directory.to_string_lossy().to_string();

    if existing_dirs.contains(&dir_str) && !args.force {
        warn!("Directory already configured. Use --force to re-initialize.");
        return Ok(());
    }

    // Add directory to configuration
    if !existing_dirs.contains(&dir_str) {
        db.add_directory(&dir_str).await?;
        info!("Directory added to library configuration");
    }

    // Initialize library (scan will be done separately)
    db.initialize_library().await?;

    println!("🎵 Library initialized successfully!");
    println!("📂 Directory: {}", args.directory.display());
    println!("🗄️  Database: {}", config.database_path.display());
    println!("\n📋 Next steps:");
    println!("  1. Run 'lrcget scan' to scan for music files");
    println!("  2. Run 'lrcget download --missing-lyrics' to download lyrics");

    Ok(())
}