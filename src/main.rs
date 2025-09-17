use clap::{Parser, Subcommand};

mod cli;
mod config;
mod core;
mod error;
mod services;
mod signal_handler;
mod ui;
mod utils;

use cli::*;
use config::Config;
use error::Result;
use services::SimpleServices;

#[derive(Parser)]
#[command(name = "lrcget")]
#[command(about = "Command-line utility for mass-downloading LRC synced lyrics")]
#[command(version)]
#[command(author = "tranxuanthang")]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Config file path (optional)
    #[arg(short, long)]
    config: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new music library
    Init(init::InitArgs),
    
    /// Scan music directories for tracks
    Scan(scan::ScanArgs),
    
    /// Download lyrics for tracks
    Download(download::DownloadArgs),
    
    /// Search for lyrics manually
    Search(search::SearchArgs),
    
    /// Fetch lyrics for a specific file
    Fetch(fetch::FetchArgs),
    
    /// Show configuration
    Config(cli::config::ConfigArgs),
    
    /// Export library data in various formats
    Export(export::ExportArgs),
    
    /// Process batch operations from file
    Batch(batch::BatchArgs),
    
    /// Manage cache operations
    Cache(cache::CacheArgs),
    
    /// Manage hooks and plugins
    Hooks(cli::hooks::HooksArgs),
    
    /// Manage output templates
    Templates(cli::templates::TemplatesArgs),
    
    /// Watch directory for new files and auto-download lyrics
    Watch(cli::watch::WatchArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Detect UI mode to determine logging behavior
    let ui_mode = ui::UiMode::detect();
    let suppress_logs = matches!(ui_mode, ui::UiMode::Terminal);

    // Initialize logging
    utils::logging::init_logging(cli.verbose, suppress_logs)
        .map_err(|e| error::LrcGetError::Internal(e))?;

    // Load configuration
    let config = Config::load(cli.config.as_deref())?;

    // Initialize services
    let services = SimpleServices::new(config);

    // Execute command with services
    let config = services.config();
    match cli.command {
        Commands::Init(args) => init::execute(args, &config).await
            .map_err(|e| error::LrcGetError::Internal(e)),
        Commands::Scan(args) => scan::execute(args, &services).await,
        Commands::Download(args) => download::execute(args, &config).await,
        Commands::Search(args) => search::execute(args, &services).await,
        Commands::Fetch(args) => fetch::execute(args, &config).await
            .map_err(|e| error::LrcGetError::Internal(e)),
        Commands::Config(args) => cli::config::execute(args, &config).await,
        Commands::Export(args) => export::execute(args, &config).await
            .map_err(|e| error::LrcGetError::Internal(e)),
        Commands::Batch(args) => batch::execute(args, &config).await,
        Commands::Cache(args) => cache::execute(args, &config).await
            .map_err(|e| error::LrcGetError::Internal(e)),
        Commands::Hooks(args) => cli::hooks::execute(args, &config).await
            .map_err(|e| error::LrcGetError::Internal(e)),
        Commands::Templates(args) => cli::templates::execute(args, &config).await
            .map_err(|e| error::LrcGetError::Internal(e)),
        Commands::Watch(args) => cli::watch::execute(args, &config).await
            .map_err(|e| error::LrcGetError::Internal(e)),
    }
}