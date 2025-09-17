use clap::{Args, Subcommand};
use anyhow::Result;

use crate::config::Config as AppConfig;

#[derive(Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    command: ConfigCommands,
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Show current configuration
    Show,

    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,

        /// Configuration value
        value: String,
    },

    /// Get a configuration value
    Get {
        /// Configuration key
        key: String,
    },

    /// Show configuration file path
    Path,

    /// Reset configuration to defaults
    Reset,

    /// List all available configuration keys
    Keys,

    /// Backup configuration and database
    Backup {
        /// Backup directory path
        #[arg(short, long)]
        path: Option<String>,
    },

    /// Restore configuration and database from backup
    Restore {
        /// Backup file path
        #[arg(short, long)]
        path: String,
    },
}

pub async fn execute(args: ConfigArgs, config: &AppConfig) -> Result<()> {
    match args.command {
        ConfigCommands::Show => {
            println!("Current configuration:");
            println!("  database_path: {}", config.database_path.display());
            println!("  lrclib_database_path: {:?}", config.lrclib_database_path);
            println!("  lrclib_instance: {}", config.lrclib_instance);
            println!("  skip_tracks_with_synced_lyrics: {}", config.skip_tracks_with_synced_lyrics);
            println!("  skip_tracks_with_plain_lyrics: {}", config.skip_tracks_with_plain_lyrics);
            println!("  try_embed_lyrics: {}", config.try_embed_lyrics);
            println!("  show_line_count: {}", config.show_line_count);
            println!("  watch_debounce_seconds: {}", config.watch_debounce_seconds);
            println!("  watch_batch_size: {}", config.watch_batch_size);
        },

        ConfigCommands::Set { key, value } => {
            let config_path = AppConfig::config_path()?;
            let mut config = config.clone();

            match key.as_str() {
                "lrclib_instance" => config.lrclib_instance = value.clone(),
                "lrclib_database_path" => {
                    config.lrclib_database_path = Some(std::path::PathBuf::from(value.clone()));
                }
                "skip_tracks_with_synced_lyrics" => {
                    config.skip_tracks_with_synced_lyrics = value.parse()?;
                },
                "skip_tracks_with_plain_lyrics" => {
                    config.skip_tracks_with_plain_lyrics = value.parse()?;
                },
                "try_embed_lyrics" => {
                    config.try_embed_lyrics = value.parse()?;
                },
                "show_line_count" => {
                    config.show_line_count = value.parse()?;
                },
                "watch_debounce_seconds" => {
                    config.watch_debounce_seconds = value.parse()?;
                },
                "watch_batch_size" => {
                    config.watch_batch_size = value.parse()?;
                },
                _ => {
                    anyhow::bail!("Unknown configuration key: {}", key);
                }
            }

            config.save(&config_path)?;
            println!("Configuration updated: {} = {}", key, value);
        },

        ConfigCommands::Get { key } => {
            let value = match key.as_str() {
                "database_path" => config.database_path.display().to_string(),
                "lrclib_instance" => config.lrclib_instance.clone(),
                "skip_tracks_with_synced_lyrics" => config.skip_tracks_with_synced_lyrics.to_string(),
                "skip_tracks_with_plain_lyrics" => config.skip_tracks_with_plain_lyrics.to_string(),
                "try_embed_lyrics" => config.try_embed_lyrics.to_string(),
                "show_line_count" => config.show_line_count.to_string(),
                "watch_debounce_seconds" => config.watch_debounce_seconds.to_string(),
                "watch_batch_size" => config.watch_batch_size.to_string(),
                _ => {
                    anyhow::bail!("Unknown configuration key: {}", key);
                }
            };

            println!("{}", value);
        },

        ConfigCommands::Path => {
            let config_path = AppConfig::config_path()?;
            println!("{}", config_path.display());
        },

        ConfigCommands::Reset => {
            let config_path = AppConfig::config_path()?;
            let default_config = AppConfig::default();
            default_config.save(&config_path)?;
            println!("✅ Configuration reset to defaults");
            println!("📁 Config file: {}", config_path.display());
        },

        ConfigCommands::Keys => {
            println!("Available configuration keys:");
            println!("  🌐 lrclib_instance           - LRCLIB server URL");
            println!("  🗃️ lrclib_database_path - LRCLIB local database path");
            println!("  ⏭️ skip_tracks_with_synced_lyrics - Skip tracks with synced lyrics");
            println!("  📝 skip_tracks_with_plain_lyrics  - Skip tracks with plain lyrics");
            println!("  🎵 try_embed_lyrics           - Embed lyrics into audio files");
            println!("  📊 show_line_count            - Show line count in lyrics");
            println!("\nUsage:");
            println!("  lrcget config get <key>");
            println!("  lrcget config set <key> <value>");
        },

        ConfigCommands::Backup { path } => {
            use std::fs;
            use chrono::Utc;

            let backup_dir = if let Some(p) = path {
                std::path::PathBuf::from(p)
            } else {
                let home_dir = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
                home_dir.join("lrcget-backups")
            };

            fs::create_dir_all(&backup_dir)?;

            let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
            let backup_name = format!("lrcget_backup_{}", timestamp);
            let backup_path = backup_dir.join(&backup_name);
            fs::create_dir_all(&backup_path)?;

            // Backup configuration
            let config_path = AppConfig::config_path()?;
            if config_path.exists() {
                let backup_config_path = backup_path.join("config.toml");
                fs::copy(&config_path, &backup_config_path)?;
                println!("✅ Configuration backed up");
            }

            // Backup database
            if config.database_path.exists() {
                let backup_db_path = backup_path.join("lrcget.db");
                fs::copy(&config.database_path, &backup_db_path)?;
                println!("✅ Database backed up");
            }

            // Create backup info file
            #[derive(serde::Serialize)]
            struct BackupInfo {
                created_at: String,
                tool_version: String,
                config_path: String,
                database_path: String,
            }

            let backup_info = BackupInfo {
                created_at: Utc::now().to_rfc3339(),
                tool_version: env!("CARGO_PKG_VERSION").to_string(),
                config_path: config_path.display().to_string(),
                database_path: config.database_path.display().to_string(),
            };

            let info_content = serde_json::to_string_pretty(&backup_info)?;
            fs::write(backup_path.join("backup_info.json"), info_content)?;

            println!("🎉 Backup completed successfully!");
            println!("📂 Backup location: {}", backup_path.display());
            println!("📋 Backup contents:");
            println!("   • config.toml (configuration)");
            println!("   • lrcget.db (music library database)");
            println!("   • backup_info.json (backup metadata)");
        },

        ConfigCommands::Restore { path } => {
            use std::fs;

            let restore_path = std::path::PathBuf::from(path);
            if !restore_path.exists() {
                anyhow::bail!("Backup path does not exist: {}", restore_path.display());
            }

            // Read backup info
            let info_path = restore_path.join("backup_info.json");
            if info_path.exists() {
                let info_content = fs::read_to_string(info_path)?;
                let info: serde_json::Value = serde_json::from_str(&info_content)?;
                println!("📋 Backup Info:");
                println!("   Created: {}", info.get("created_at").unwrap_or(&serde_json::Value::Null));
                println!("   Tool Version: {}", info.get("tool_version").unwrap_or(&serde_json::Value::Null));
            }

            // Restore configuration
            let backup_config = restore_path.join("config.toml");
            if backup_config.exists() {
                let current_config_path = AppConfig::config_path()?;
                if let Some(parent) = current_config_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&backup_config, &current_config_path)?;
                println!("✅ Configuration restored");
            }

            // Restore database
            let backup_db = restore_path.join("lrcget.db");
            if backup_db.exists() {
                if let Some(parent) = config.database_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&backup_db, &config.database_path)?;
                println!("✅ Database restored");
            }

            println!("🎉 Restore completed successfully!");
            println!("🔄 You may need to restart the application for changes to take effect");
        },
    }

    Ok(())
}
