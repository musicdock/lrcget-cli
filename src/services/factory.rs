use std::sync::Arc;
use crate::config::Config;
use crate::core::data::database::Database;
use crate::core::services::lrclib::{LrclibClient, LyricsDownloader};
use crate::core::files::scanner::Scanner;
use crate::error::Result;

/// Centralized factory for creating all core services
/// Eliminates duplication of service creation patterns across CLI commands
pub struct ServiceFactory {
    config: Arc<Config>,
}

impl ServiceFactory {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    /// Create a database connection
    /// Used instead of repeated `Database::new(&config.database_path).await?` patterns
    pub async fn create_database(&self) -> Result<Database> {
        let db = Database::new(&self.config.database_path)
            .await
            .map_err(crate::error::LrcGetError::Internal)?;
        Ok(db)
    }

    /// Create LRCLIB client with standard configuration
    /// Replaces `LrclibClient::new(&config.lrclib_instance)` patterns
    pub fn create_lrclib_client(&self) -> LrclibClient {
        LrclibClient::new(&self.config.lrclib_instance)
    }

    /// Create LRCLIB client without local database (for API-only mode)
    /// Replaces `config.create_lrclib_client_no_local_db()` patterns
    pub fn create_lrclib_client_api_only(&self) -> LrclibClient {
        LrclibClient::new(&self.config.lrclib_instance)
    }

    /// Create lyrics downloader
    /// Replaces `LyricsDownloader::new(&config.lrclib_instance)` patterns
    pub fn create_lyrics_downloader(&self) -> LyricsDownloader {
        LyricsDownloader::new(&self.config.lrclib_instance)
    }

    /// Create scanner instance
    pub fn create_scanner(&self) -> Scanner {
        Scanner
    }

    /// Get configuration reference
    pub fn config(&self) -> Arc<Config> {
        self.config.clone()
    }

    /// Create a database and scanner together (common pattern)
    pub async fn create_database_and_scanner(&self) -> Result<(Database, Scanner)> {
        let db = self.create_database().await?;
        let scanner = self.create_scanner();
        Ok((db, scanner))
    }

    /// Create client based on environment (handles FORCE_API_ONLY pattern)
    pub fn create_client_with_env_override(&self) -> LrclibClient {
        if std::env::var("FORCE_API_ONLY").is_ok() {
            self.create_lrclib_client_api_only()
        } else {
            self.create_lrclib_client()
        }
    }
}

/// Common service bundles to reduce boilerplate
pub struct ServiceBundle {
    pub database: Database,
    pub client: LrclibClient,
    pub scanner: Scanner,
    pub config: Arc<Config>,
}

impl ServiceFactory {
    /// Create a complete service bundle for commands that need everything
    pub async fn create_full_bundle(&self) -> Result<ServiceBundle> {
        let database = self.create_database().await?;
        let client = self.create_lrclib_client();
        let scanner = self.create_scanner();

        Ok(ServiceBundle {
            database,
            client,
            scanner,
            config: self.config.clone(),
        })
    }
}