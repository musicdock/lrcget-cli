use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use directories::ProjectDirs;
use tracing::warn;

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
        // Use /data only when explicitly running under Docker (DOCKER env var)
        let default_data_path = if env::var("DOCKER").is_ok() {
            PathBuf::from("/data")
        } else {
            match ProjectDirs::from("net", "lrclib", "lrcget-cli") {
                Some(project_dirs) => project_dirs.data_dir().to_path_buf(),
                None => {
                    // Graceful fallback to current directory if project dirs unavailable
                    warn!("ProjectDirs unavailable; falling back to current directory for data path");
                    PathBuf::from(".")
                }
            }
        };

        Self {
            database_path: default_data_path.join("lrcget.db"),
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
    }
}

impl Config {
    pub fn load(config_path: Option<&str>) -> Result<Self> {
        // Try to load .env file if it exists (for Docker and development)
        dotenvy::dotenv().ok();

        // Start with default configuration
        let mut config = Self::default();

        // Override with file configuration if available
        let config_file = if let Some(path) = config_path {
            PathBuf::from(path)
        } else {
            Self::default_config_path()?
        };

        if config_file.exists() {
            let content = fs::read_to_string(&config_file)?;
            let file_config: Config = toml::from_str(&content)?;
            config = file_config;
        }

        // Override with environment variables (highest priority)
        config.load_from_env();

        // Ensure data directory exists
        if let Some(parent) = config.database_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Save config file if it doesn't exist
        if !config_file.exists() {
            if let Some(parent) = config_file.parent() {
                fs::create_dir_all(parent)?;
            }
            config.save(&config_file)?;
        }

        Ok(config)
    }

    /// Load configuration from environment variables
    fn load_from_env(&mut self) {
        if let Ok(db_path) = env::var("LRCGET_DATABASE_PATH") {
            self.database_path = PathBuf::from(db_path);
        }

        if let Ok(instance) = env::var("LRCGET_LRCLIB_INSTANCE") {
            self.lrclib_instance = instance;
        }

        if let Ok(local_db_path) = env::var("LRCGET_LRCLIB_DATABASE_PATH") {
            self.lrclib_database_path = Some(PathBuf::from(local_db_path));
        }

        if let Ok(skip_synced) = env::var("LRCGET_SKIP_TRACKS_WITH_SYNCED_LYRICS") {
            if let Ok(value) = skip_synced.parse::<bool>() {
                self.skip_tracks_with_synced_lyrics = value;
            }
        }

        if let Ok(skip_plain) = env::var("LRCGET_SKIP_TRACKS_WITH_PLAIN_LYRICS") {
            if let Ok(value) = skip_plain.parse::<bool>() {
                self.skip_tracks_with_plain_lyrics = value;
            }
        }

        if let Ok(embed) = env::var("LRCGET_TRY_EMBED_LYRICS") {
            if let Ok(value) = embed.parse::<bool>() {
                self.try_embed_lyrics = value;
            }
        }

        if let Ok(show_count) = env::var("LRCGET_SHOW_LINE_COUNT") {
            if let Ok(value) = show_count.parse::<bool>() {
                self.show_line_count = value;
            }
        }

        if let Ok(debounce) = env::var("LRCGET_WATCH_DEBOUNCE_SECONDS") {
            if let Ok(value) = debounce.parse::<u64>() {
                self.watch_debounce_seconds = value;
            }
        }

        if let Ok(batch_size) = env::var("LRCGET_WATCH_BATCH_SIZE") {
            if let Ok(value) = batch_size.parse::<usize>() {
                self.watch_batch_size = value;
            }
        }

        if let Ok(redis_url) = env::var("LRCGET_REDIS_URL") {
            let trimmed = redis_url.trim().to_string();
            if !trimmed.is_empty() {
                self.redis_url = Some(trimmed);
            } else {
                self.redis_url = None;
            }
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    fn default_config_path() -> Result<PathBuf> {
        let project_dirs = ProjectDirs::from("net", "lrclib", "lrcget-cli")
            .ok_or_else(|| anyhow::anyhow!("Failed to determine project directories"))?;

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

    pub fn create_lrclib_client(&self) -> crate::core::lrclib::LrclibClient {
        if let Some(ref local_db_path) = self.lrclib_database_path {
            crate::core::lrclib::LrclibClient::with_local_db(&self.lrclib_instance, local_db_path)
        } else {
            crate::core::lrclib::LrclibClient::new(&self.lrclib_instance)
        }
    }

    pub fn create_lrclib_client_no_local_db(&self) -> crate::core::lrclib::LrclibClient {
        crate::core::lrclib::LrclibClient::new(&self.lrclib_instance)
    }
}
