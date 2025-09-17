use std::path::{Path, PathBuf};
use directories::ProjectDirs;
use tracing::warn;
use crate::error::{Result, LrcGetError};
use crate::config::validation::ConfigValidator;
use crate::config::env::{EnvVars, EnvParser};
use crate::config::Config;

/// Configuration builder with validation and type safety
pub struct ConfigBuilder {
    database_path: Option<PathBuf>,
    lrclib_instance: Option<String>,
    lrclib_database_path: Option<Option<PathBuf>>,
    skip_tracks_with_synced_lyrics: Option<bool>,
    skip_tracks_with_plain_lyrics: Option<bool>,
    try_embed_lyrics: Option<bool>,
    show_line_count: Option<bool>,
    watch_debounce_seconds: Option<u64>,
    watch_batch_size: Option<usize>,
    redis_url: Option<Option<String>>,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigBuilder {
    /// Create a new configuration builder
    pub fn new() -> Self {
        Self {
            database_path: None,
            lrclib_instance: None,
            lrclib_database_path: None,
            skip_tracks_with_synced_lyrics: None,
            skip_tracks_with_plain_lyrics: None,
            try_embed_lyrics: None,
            show_line_count: None,
            watch_debounce_seconds: None,
            watch_batch_size: None,
            redis_url: None,
        }
    }

    /// Set database path with validation
    pub fn database_path<P: AsRef<Path>>(mut self, path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        ConfigValidator::validate_db_path(&path)?;
        ConfigValidator::validate_path(&path, "database", false)?;
        self.database_path = Some(path);
        Ok(self)
    }

    /// Set LRCLIB instance URL with validation
    pub fn lrclib_instance<S: Into<String>>(mut self, url: S) -> Result<Self> {
        let url = url.into();
        ConfigValidator::validate_url(&url, "LRCLIB instance")?;
        self.lrclib_instance = Some(url);
        Ok(self)
    }

    /// Set local LRCLIB database path with validation
    pub fn lrclib_database_path<P: AsRef<Path>>(mut self, path: Option<P>) -> Result<Self> {
        if let Some(path) = path {
            let path = path.as_ref().to_path_buf();
            ConfigValidator::validate_path(&path, "LRCLIB database", true)?;
            self.lrclib_database_path = Some(Some(path));
        } else {
            self.lrclib_database_path = Some(None);
        }
        Ok(self)
    }

    /// Set skip tracks with synced lyrics
    pub fn skip_tracks_with_synced_lyrics(mut self, skip: bool) -> Self {
        self.skip_tracks_with_synced_lyrics = Some(skip);
        self
    }

    /// Set skip tracks with plain lyrics
    pub fn skip_tracks_with_plain_lyrics(mut self, skip: bool) -> Self {
        self.skip_tracks_with_plain_lyrics = Some(skip);
        self
    }

    /// Set try embed lyrics
    pub fn try_embed_lyrics(mut self, embed: bool) -> Self {
        self.try_embed_lyrics = Some(embed);
        self
    }

    /// Set show line count
    pub fn show_line_count(mut self, show: bool) -> Self {
        self.show_line_count = Some(show);
        self
    }

    /// Set watch debounce seconds with validation
    pub fn watch_debounce_seconds(mut self, seconds: u64) -> Result<Self> {
        ConfigValidator::validate_range(seconds, 1, 3600, "watch debounce seconds")?;
        self.watch_debounce_seconds = Some(seconds);
        Ok(self)
    }

    /// Set watch batch size with validation
    pub fn watch_batch_size(mut self, size: usize) -> Result<Self> {
        ConfigValidator::validate_range(size, 1, 1000, "watch batch size")?;
        self.watch_batch_size = Some(size);
        Ok(self)
    }

    /// Set Redis URL with validation
    pub fn redis_url<S: Into<String>>(mut self, url: Option<S>) -> Result<Self> {
        if let Some(url) = url {
            let url = url.into();
            ConfigValidator::validate_redis_url(&url)?;
            self.redis_url = Some(Some(url));
        } else {
            self.redis_url = Some(None);
        }
        Ok(self)
    }

    /// Load values from environment variables with validation
    pub fn load_from_env(mut self) -> Result<Self> {
        // Database path
        if let Some(path) = EnvParser::parse_path(EnvVars::DATABASE_PATH, false)? {
            self = self.database_path(path)?;
        }

        // LRCLIB instance
        if let Some(url) = EnvParser::parse_string(EnvVars::LRCLIB_INSTANCE, None)? {
            self = self.lrclib_instance(url)?;
        }

        // LRCLIB database path
        let lrclib_db_path = EnvParser::parse_path(EnvVars::LRCLIB_DATABASE_PATH, true)?;
        self = self.lrclib_database_path(lrclib_db_path)?;

        // Boolean flags
        if let Some(skip_synced) = EnvParser::parse_bool(EnvVars::SKIP_TRACKS_WITH_SYNCED_LYRICS)? {
            self = self.skip_tracks_with_synced_lyrics(skip_synced);
        }

        if let Some(skip_plain) = EnvParser::parse_bool(EnvVars::SKIP_TRACKS_WITH_PLAIN_LYRICS)? {
            self = self.skip_tracks_with_plain_lyrics(skip_plain);
        }

        if let Some(embed) = EnvParser::parse_bool(EnvVars::TRY_EMBED_LYRICS)? {
            self = self.try_embed_lyrics(embed);
        }

        if let Some(show_count) = EnvParser::parse_bool(EnvVars::SHOW_LINE_COUNT)? {
            self = self.show_line_count(show_count);
        }

        // Numeric values with validation
        if let Some(debounce) = EnvParser::parse_u64(EnvVars::WATCH_DEBOUNCE_SECONDS, 1, 3600)? {
            self = self.watch_debounce_seconds(debounce)?;
        }

        if let Some(batch_size) = EnvParser::parse_usize(EnvVars::WATCH_BATCH_SIZE, 1, 1000)? {
            self = self.watch_batch_size(batch_size)?;
        }

        // Redis URL
        if let Some(redis) = EnvParser::parse_string(EnvVars::REDIS_URL, Some(|url| {
            ConfigValidator::validate_redis_url(url)
        }))? {
            self = self.redis_url(Some(redis))?;
        }

        Ok(self)
    }

    /// Build the configuration with defaults
    pub fn build(self) -> Result<Config> {
        let default_data_path = Self::default_data_path()?;

        let config = Config {
            database_path: self.database_path
                .unwrap_or_else(|| default_data_path.join("lrcget.db")),
            lrclib_instance: self.lrclib_instance
                .unwrap_or_else(|| "https://lrclib.net".to_string()),
            lrclib_database_path: self.lrclib_database_path
                .unwrap_or(None),
            skip_tracks_with_synced_lyrics: self.skip_tracks_with_synced_lyrics
                .unwrap_or(true),
            skip_tracks_with_plain_lyrics: self.skip_tracks_with_plain_lyrics
                .unwrap_or(false),
            try_embed_lyrics: self.try_embed_lyrics
                .unwrap_or(false),
            show_line_count: self.show_line_count
                .unwrap_or(true),
            watch_debounce_seconds: self.watch_debounce_seconds
                .unwrap_or(10),
            watch_batch_size: self.watch_batch_size
                .unwrap_or(50),
            redis_url: self.redis_url
                .unwrap_or(None),
        };

        // Final validation
        config.validate()?;

        Ok(config)
    }

    /// Get default data path based on environment
    fn default_data_path() -> Result<PathBuf> {
        // Use /data only when explicitly running under Docker
        if EnvParser::is_present(EnvVars::DOCKER) {
            Ok(PathBuf::from("/data"))
        } else {
            match ProjectDirs::from("net", "lrclib", "lrcget-cli") {
                Some(project_dirs) => Ok(project_dirs.data_dir().to_path_buf()),
                None => {
                    warn!("ProjectDirs unavailable; falling back to current directory for data path");
                    Ok(PathBuf::from("."))
                }
            }
        }
    }
}

impl Config {
    /// Validate the entire configuration
    pub fn validate(&self) -> Result<()> {
        // Validate database path
        ConfigValidator::validate_db_path(&self.database_path)?;

        // Validate LRCLIB instance URL
        ConfigValidator::validate_url(&self.lrclib_instance, "LRCLIB instance")?;

        // Validate LRCLIB database path if present
        if let Some(ref path) = self.lrclib_database_path {
            ConfigValidator::validate_path(path, "LRCLIB database", true)?;
        }

        // Validate numeric ranges
        ConfigValidator::validate_range(
            self.watch_debounce_seconds,
            1,
            3600,
            "watch debounce seconds"
        )?;

        ConfigValidator::validate_range(
            self.watch_batch_size,
            1,
            1000,
            "watch batch size"
        )?;

        // Validate Redis URL if present
        if let Some(ref url) = self.redis_url {
            ConfigValidator::validate_redis_url(url)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_config_builder_basic() {
        let config = ConfigBuilder::new()
            .lrclib_instance("https://test.example.com")
            .unwrap()
            .skip_tracks_with_synced_lyrics(false)
            .build()
            .unwrap();

        assert_eq!(config.lrclib_instance, "https://test.example.com");
        assert!(!config.skip_tracks_with_synced_lyrics);
    }

    #[test]
    fn test_config_builder_validation() {
        let result = ConfigBuilder::new()
            .lrclib_instance("not-a-url")
            .unwrap_err();

        assert!(matches!(result, LrcGetError::Validation(_)));
    }

    #[test]
    fn test_config_builder_from_env() {
        env::set_var("LRCGET_LRCLIB_INSTANCE", "https://env.example.com");
        env::set_var("LRCGET_SKIP_TRACKS_WITH_SYNCED_LYRICS", "false");

        let config = ConfigBuilder::new()
            .load_from_env()
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(config.lrclib_instance, "https://env.example.com");
        assert!(!config.skip_tracks_with_synced_lyrics);

        env::remove_var("LRCGET_LRCLIB_INSTANCE");
        env::remove_var("LRCGET_SKIP_TRACKS_WITH_SYNCED_LYRICS");
    }
}