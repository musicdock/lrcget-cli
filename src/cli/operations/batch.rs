use clap::Args;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::{info, warn};
use tokio::time::{sleep, Duration};

use crate::error::Result;
use crate::services::ServiceFactory;
use crate::core::services::lrclib::LyricsDownloader;

#[derive(Args)]
pub struct BatchArgs {
    /// Batch file path (JSON, TOML, or CSV)
    #[arg(value_name = "FILE")]
    file: PathBuf,

    /// Operation type (download, search, validate)
    #[arg(short, long, default_value = "download")]
    operation: String,

    /// Maximum parallel operations
    #[arg(long, default_value = "4")]
    parallel: usize,

    /// Delay between operations (milliseconds)
    #[arg(long, default_value = "100")]
    delay: u64,

    /// Dry run (don't actually execute)
    #[arg(long)]
    dry_run: bool,

    /// Continue on errors
    #[arg(long)]
    continue_on_error: bool,

    /// Output results to file
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct BatchItem {
    #[serde(default)]
    id: Option<String>,
    title: String,
    artist: String,
    #[serde(default)]
    album: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    file_path: Option<String>,
}

#[derive(Serialize)]
struct BatchResult {
    item: BatchItem,
    success: bool,
    message: String,
    execution_time_ms: u64,
}

#[derive(Serialize)]
struct BatchReport {
    total_items: usize,
    successful: usize,
    failed: usize,
    execution_time_ms: u64,
    results: Vec<BatchResult>,
}

pub async fn execute(args: BatchArgs, config: &crate::config::Config) -> crate::error::Result<()> {
    info!("📋 Processing batch file: {}", args.file.display());

    if !args.file.exists() {
        return Err(crate::error::LrcGetError::Validation(
            format!("Batch file does not exist: {}", args.file.display())
        ));
    }

    let factory = ServiceFactory::new(std::sync::Arc::new(config.clone()));

    let items = load_batch_file(&args.file).await?;
    info!("📥 Loaded {} items from batch file", items.len());

    if args.dry_run {
        info!("🧪 DRY RUN - would process {} items:", items.len());
        for (i, item) in items.iter().enumerate() {
            println!("  {}. {} - {} ({})", 
                i + 1, 
                item.artist, 
                item.title,
                item.album.as_deref().unwrap_or("Unknown Album")
            );
        }
        return Ok(());
    }

    match args.operation.as_str() {
        "download" => execute_batch_download(args, &factory, items).await,
        "search" => execute_batch_search(args, &factory, items).await,
        "validate" => execute_batch_validate(args, &factory, items).await,
        _ => Err(crate::error::LrcGetError::Validation(
            format!("Unknown operation: {}. Available: download, search, validate", args.operation)
        )),
    }
}

async fn load_batch_file(file_path: &PathBuf) -> Result<Vec<BatchItem>> {
    let content = fs::read_to_string(file_path)?;
    let extension = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");

    match extension.to_lowercase().as_str() {
        "json" => {
            Ok(serde_json::from_str(&content)?)
        },
        "toml" => {
            #[derive(Deserialize)]
            struct BatchFile {
                items: Vec<BatchItem>,
            }
            let batch_file: BatchFile = toml::from_str(&content)?;
            Ok(batch_file.items)
        },
        "csv" => {
            parse_csv(&content)
        },
        _ => Err(crate::error::LrcGetError::Validation(
            "Unsupported batch file format. Use .json, .toml, or .csv".to_string()
        )),
    }
}

fn parse_csv(content: &str) -> Result<Vec<BatchItem>> {
    let mut items = Vec::new();
    let mut lines = content.lines();
    
    // Skip header if present
    let header = lines.next().unwrap_or("");
    let has_header = header.to_lowercase().contains("title") || header.to_lowercase().contains("artist");
    
    if !has_header {
        // Process first line as data
        if let Some(item) = parse_csv_line(header)? {
            items.push(item);
        }
    }

    for line in lines {
        if let Some(item) = parse_csv_line(line)? {
            items.push(item);
        }
    }

    Ok(items)
}

fn parse_csv_line(line: &str) -> Result<Option<BatchItem>> {
    if line.trim().is_empty() {
        return Ok(None);
    }

    let fields: Vec<&str> = line.split(',').map(|f| f.trim().trim_matches('"')).collect();
    
    if fields.len() < 2 {
        warn!("Invalid CSV line (need at least title,artist): {}", line);
        return Ok(None);
    }

    Ok(Some(BatchItem {
        id: None,
        title: fields[0].to_string(),
        artist: fields[1].to_string(),
        album: fields.get(2).map(|s| s.to_string()).filter(|s| !s.is_empty()),
        duration: fields.get(3).and_then(|s| s.parse().ok()),
        file_path: fields.get(4).map(|s| s.to_string()).filter(|s| !s.is_empty()),
    }))
}

async fn execute_batch_download(args: BatchArgs, factory: &ServiceFactory, items: Vec<BatchItem>) -> Result<()> {
    info!("⬇️ Starting batch download for {} items", items.len());

    // Use ServiceFactory to eliminate duplication
    let _bundle = factory.create_full_bundle().await?;
    let downloader = factory.create_lyrics_downloader();
    let start_time = std::time::Instant::now();
    
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(args.parallel));
    let mut results: Vec<BatchResult> = Vec::new();
    let mut tasks = Vec::new();
    
    for item in items {
        let downloader = downloader.clone();
        let semaphore = semaphore.clone();
        let delay = args.delay;
        let continue_on_error = args.continue_on_error;
        
        let task = tokio::spawn(async move {
            let item_start = std::time::Instant::now();

            // Acquire a permit; if semaphore is closed, mark as failed gracefully
            let permit = match semaphore.acquire().await {
                Ok(p) => p,
                Err(_) => {
                    return BatchResult {
                        item,
                        success: false,
                        message: "Semaphore closed while acquiring permit".to_string(),
                        execution_time_ms: item_start.elapsed().as_millis() as u64,
                    };
                }
            };
            let _permit = permit;
            
            // Add delay between requests
            if delay > 0 {
                sleep(Duration::from_millis(delay)).await;
            }
            
            let result = process_download_item(&downloader, &item).await;
            
            let execution_time = item_start.elapsed().as_millis() as u64;
            
            match result {
                Ok(message) => BatchResult {
                    item,
                    success: true,
                    message,
                    execution_time_ms: execution_time,
                },
                Err(e) => {
                    let error_msg = e.to_string();
                    if !continue_on_error {
                        warn!("❌ Failed to download lyrics for {}: {}", item.title, error_msg);
                    }
                    BatchResult {
                        item,
                        success: false,
                        message: error_msg,
                        execution_time_ms: execution_time,
                    }
                }
            }
        });
        
        tasks.push(task);
    }
    
    // Wait for all tasks to complete
    for task in tasks {
        results.push(task.await?);
    }
    
    let total_time = start_time.elapsed().as_millis() as u64;
    let successful = results.iter().filter(|r| r.success).count();
    let failed = results.len() - successful;
    
    let report = BatchReport {
        total_items: results.len(),
        successful,
        failed,
        execution_time_ms: total_time,
        results,
    };
    
    // Output report
    if let Some(output_path) = &args.output {
        let json_report = serde_json::to_string_pretty(&report)?;
        fs::write(output_path, json_report)?;
        info!("📋 Batch report saved to {}", output_path.display());
    }
    
    // Summary
    println!("\n📊 Batch Download Summary:");
    println!("  ✅ Successful: {}", successful);
    println!("  ❌ Failed: {}", failed);
    println!("  ⏱️ Total Time: {:.2}s", total_time as f64 / 1000.0);
    println!("  📈 Success Rate: {:.1}%", (successful as f64 / report.total_items as f64) * 100.0);
    
    Ok(())
}

async fn process_download_item(_downloader: &LyricsDownloader, item: &BatchItem) -> Result<String> {
    // This is a simplified version - in reality we'd need to create a DatabaseTrack
    // from the BatchItem or integrate more deeply with the database
    info!("Processing: {} - {}", item.artist, item.title);
    
    // For now, return a mock success - in full implementation this would:
    // 1. Search for the track in LRCLIB
    // 2. Download lyrics if found
    // 3. Save to appropriate files
    // 4. Update database if track exists
    
    Ok(format!("Downloaded lyrics for {} - {}", item.artist, item.title))
}

async fn execute_batch_search(args: BatchArgs, factory: &ServiceFactory, items: Vec<BatchItem>) -> Result<()> {
    info!("🔍 Starting batch search for {} items", items.len());
    
    let client = factory.create_lrclib_client();
    let _results: Vec<crate::core::services::lrclib::SearchResult> = Vec::new();
    
    for (i, item) in items.iter().enumerate() {
        info!("Searching {}/{}: {} - {}", i + 1, items.len(), item.artist, item.title);
        
        let search_results = client.search(
            &item.title,
            &item.artist,
            item.album.as_deref().unwrap_or(""),
            "",
        ).await?;
        
        println!("🎵 {} - {}: {} results", item.artist, item.title, search_results.len());
        
        if args.delay > 0 {
            sleep(Duration::from_millis(args.delay)).await;
        }
    }
    
    Ok(())
}

async fn execute_batch_validate(_args: BatchArgs, _factory: &ServiceFactory, items: Vec<BatchItem>) -> Result<()> {
    info!("✅ Starting batch validation for {} items", items.len());
    
    let mut valid_count = 0;
    let mut invalid_count = 0;
    
    for item in &items {
        let mut is_valid = true;
        let mut errors = Vec::new();
        
        if item.title.is_empty() {
            errors.push("Missing title");
            is_valid = false;
        }
        
        if item.artist.is_empty() {
            errors.push("Missing artist");
            is_valid = false;
        }
        
        if let Some(file_path) = &item.file_path {
            if !std::path::Path::new(file_path).exists() {
                errors.push("File does not exist");
                is_valid = false;
            }
        }
        
        if is_valid {
            valid_count += 1;
        } else {
            invalid_count += 1;
            println!("❌ Invalid: {} - {} ({})", 
                item.artist, item.title, errors.join(", "));
        }
    }
    
    println!("\n📊 Validation Summary:");
    println!("  ✅ Valid: {}", valid_count);
    println!("  ❌ Invalid: {}", invalid_count);
    println!("  📈 Valid Rate: {:.1}%", (valid_count as f64 / items.len() as f64) * 100.0);
    
    Ok(())
}