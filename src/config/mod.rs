use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use crate::error::Result;

pub mod validation;
pub mod env;
pub mod builder;

pub use env::EnvVars;
pub use builder::ConfigBuilder;

fn default_watch_debounce_seconds() -> u64 {
    10
}

fn default_watch_batch_size() -> usize {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Database file path
    pub database_path: PathBuf,

    /// LRCLIB instance URL
    pub lrclib_instance: String,

    /// Skip tracks that already have synced lyrics
    pub skip_tracks_with_synced_lyrics: bool,

    /// Skip tracks that already have plain lyrics
    pub skip_tracks_with_plain_lyrics: bool,

    /// Try to embed lyrics into audio files
    pub try_embed_lyrics: bool,

    /// Show line count in lyrics
    pub show_line_count: bool,

    /// Path to local LRCLIB database (optional)
    pub lrclib_database_path: Option<PathBuf>,

    /// Default debounce time for watch command (seconds)
    #[serde(default = "default_watch_debounce_seconds")]
    pub watch_debounce_seconds: u64,

    /// Default batch size for watch command
    #[serde(default = "default_watch_batch_size")]
    pub watch_batch_size: usize,

    /// Redis URL for cache (optional)
    #[serde(default)]
    pub redis_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        // Use builder to ensure consistency
        ConfigBuilder::new()
            .build()
            .unwrap_or_else(|_| {
                // Fallback to basic defaults if builder fails
                Self {
                    database_path: PathBuf::from("lrcget.db"),
                    lrclib_instance: "https://lrclib.net".to_string(),
                    lrclib_database_path: None,
                    skip_tracks_with_synced_lyrics: true,
                    skip_tracks_with_plain_lyrics: false,
                    try_embed_lyrics: false,
                    show_line_count: true,
                    watch_debounce_seconds: 10,
                    watch_batch_size: 50,
                    redis_url: None,
                }
            })
    }
}

impl Config {
    pub fn load(config_path: Option<&str>) -> Result<Self> {
        // Try to load .env file if it exists (for Docker and development)
        dotenvy::dotenv().ok();

        // Start with builder for type safety
        let mut builder = ConfigBuilder::new();

        // Load from config file if available
        let config_file = if let Some(path) = config_path {
            PathBuf::from(path)
        } else {
            Self::default_config_path()?
        };

        if config_file.exists() {
            let content = fs::read_to_string(&config_file).map_err(|e| {
                crate::error::LrcGetError::Validation(format!(
                    "Failed to read config file {}: {}",
                    config_file.display(), e
                ))
            })?;

            let file_config: Config = toml::from_str(&content).map_err(|e| {
                crate::error::LrcGetError::Validation(format!(
                    "Invalid TOML in config file {}: {}",
                    config_file.display(), e
                ))
            })?;

            // Apply file config values to builder
            builder = builder
                .database_path(&file_config.database_path)?
                .lrclib_instance(file_config.lrclib_instance)?
                .lrclib_database_path(file_config.lrclib_database_path.as_ref())?
                .skip_tracks_with_synced_lyrics(file_config.skip_tracks_with_synced_lyrics)
                .skip_tracks_with_plain_lyrics(file_config.skip_tracks_with_plain_lyrics)
                .try_embed_lyrics(file_config.try_embed_lyrics)
                .show_line_count(file_config.show_line_count)
                .watch_debounce_seconds(file_config.watch_debounce_seconds)?
                .watch_batch_size(file_config.watch_batch_size)?
                .redis_url(file_config.redis_url)?;
        }

        // Override with environment variables (highest priority)
        builder = builder.load_from_env()?;

        // Build and validate configuration
        let config = builder.build()?;

        // Ensure data directory exists
        if let Some(parent) = config.database_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                crate::error::LrcGetError::Validation(format!(
                    "Failed to create data directory {}: {}",
                    parent.display(), e
                ))
            })?;
        }

        // Save config file if it doesn't exist
        if !config_file.exists() {
            if let Some(parent) = config_file.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    crate::error::LrcGetError::Validation(format!(
                        "Failed to create config directory {}: {}",
                        parent.display(), e
                    ))
                })?;
            }
            config.save(&config_file)?;
        }

        Ok(config)
    }


    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self).map_err(|e| {
            crate::error::LrcGetError::Validation(format!(
                "Failed to serialize config to TOML: {}",
                e
            ))
        })?;

        fs::write(path, content).map_err(|e| {
            crate::error::LrcGetError::Validation(format!(
                "Failed to write config file {}: {}",
                path.display(), e
            ))
        })?;

        Ok(())
    }

    fn default_config_path() -> Result<PathBuf> {
        use directories::ProjectDirs;

        let project_dirs = ProjectDirs::from("net", "lrclib", "lrcget-cli")
            .ok_or_else(|| crate::error::LrcGetError::Validation(
                "Failed to determine project directories".to_string()
            ))?;

        Ok(project_dirs.config_dir().join("config.toml"))
    }

    pub fn config_path() -> Result<PathBuf> {
        Self::default_config_path()
    }

    pub fn lrclib_db_path(&self) -> PathBuf {
        self.database_path.parent()
            .unwrap_or(&self.database_path)
            .join("lrclib")
            .join("lrclib.db")
    }

    pub fn create_lrclib_client(&self) -> crate::core::services::lrclib::LrclibClient {
        if let Some(ref local_db_path) = self.lrclib_database_path {
            crate::core::services::lrclib::LrclibClient::with_local_db(&self.lrclib_instance, local_db_path)
        } else {
            crate::core::services::lrclib::LrclibClient::new(&self.lrclib_instance)
        }
    }

    pub fn create_lrclib_client_no_local_db(&self) -> crate::core::services::lrclib::LrclibClient {
        crate::core::services::lrclib::LrclibClient::new(&self.lrclib_instance)
    }
}
