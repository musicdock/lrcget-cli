use clap::{Args, Subcommand};
use anyhow::Result;
use tracing::info;

use crate::config::Config;
use crate::core::cache::{LyricsCache, LyricsCacheInterface};

#[derive(Args)]
pub struct CacheArgs {
    #[command(subcommand)]
    command: CacheCommands,
}

#[derive(Subcommand)]
enum CacheCommands {
    /// Show cache statistics
    Stats,
    
    /// Clear all cached data
    Clear,
    
    /// Cleanup expired entries
    Cleanup,
    
    /// Show cache configuration
    Info,
}

pub async fn execute(args: CacheArgs, config: &Config) -> Result<()> {
    let cache_dir = config.database_path.parent()
        .unwrap_or(&config.database_path)
        .join("cache");
    
    let mut cache = LyricsCache::new(cache_dir, config.redis_url.as_deref())?;
    
    match args.command {
        CacheCommands::Stats => {
            let stats = cache.get_stats();
            
            println!("📊 Cache Statistics");
            println!("══════════════════");
            println!("🗂️  Total Entries: {}", stats.total_entries);
            println!("📈 Total Requests: {}", stats.total_requests);
            println!("✅ Cache Hits: {}", stats.cache_hits);
            println!("📊 Hit Rate: {:.1}%", stats.hit_rate_percent);
            
            if stats.last_cleanup > 0 {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                    .as_secs();
                let seconds_ago = now.saturating_sub(stats.last_cleanup);
                println!("🧹 Last Cleanup: {} seconds ago", seconds_ago);
            }
            
            // Calculate cache efficiency
            if stats.total_requests > 0 {
                let miss_rate = 100.0 - stats.hit_rate_percent;
                println!("❌ Miss Rate: {:.1}%", miss_rate);
                
                if stats.hit_rate_percent > 80.0 {
                    println!("💡 Cache performance: Excellent! 🌟");
                } else if stats.hit_rate_percent > 60.0 {
                    println!("💡 Cache performance: Good ✅");
                } else if stats.hit_rate_percent > 30.0 {
                    println!("💡 Cache performance: Fair ⚠️");
                } else {
                    println!("💡 Cache performance: Poor ❌");
                }
            }
        },
        
        CacheCommands::Clear => {
            info!("🗑️ Clearing cache...");
            cache.clear().await?;
            cache.save_index().await?;
            
            println!("✅ Cache cleared successfully!");
            println!("📊 All cached lyrics data has been removed");
            println!("💡 Next downloads will rebuild the cache");
        },
        
        CacheCommands::Cleanup => {
            info!("🧹 Cleaning up expired cache entries...");
            let stats_before = cache.get_stats();
            
            cache.cleanup_old_entries().await?;
            cache.save_index().await?;
            
            let stats_after = cache.get_stats();
            let removed = stats_before.total_entries.saturating_sub(stats_after.total_entries);
            
            println!("✅ Cache cleanup completed!");
            println!("🗑️ Removed {} expired entries", removed);
            println!("📊 Cache now contains {} entries", stats_after.total_entries);
        },
        
        CacheCommands::Info => {
            let cache_path = config.database_path.parent()
                .unwrap_or(&config.database_path)
                .join("cache");
            
            println!("ℹ️  Cache Configuration");
            println!("═════════════════════");
            println!("📁 Cache Directory: {}", cache_path.display());
            println!("⏰ Max Age: 7 days");
            println!("📊 Max Entries: 10,000");
            println!("🔧 Auto Cleanup: Yes");
            
            if cache_path.exists() {
                let metadata = std::fs::metadata(&cache_path)?;
                println!("💾 Directory Size: ~{} KB", metadata.len() / 1024);
            } else {
                println!("📝 Status: Not initialized (will be created on first use)");
            }
            
            println!("\n💡 Tips:");
            println!("  • Cache improves performance by storing previous API responses");
            println!("  • Entries expire after 7 days to ensure fresh data");
            println!("  • Use 'lrcget cache stats' to monitor cache performance");
            println!("  • Use 'lrcget cache cleanup' to remove expired entries");
            println!("  • Use 'lrcget cache clear' to start fresh");
        },
    }
    
    Ok(())
}