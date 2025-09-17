use std::sync::Arc;
use crate::config::Config;
use crate::core::services::lrclib::{LrclibClient as LrcLibClient, LyricsDownloader as Downloader};
use crate::core::infrastructure::cache::CacheService;
use crate::services::DatabaseService;
use crate::error::Result;

pub struct ClientFactory {
    config: Arc<Config>,
}

impl ClientFactory {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    pub async fn create_lrclib_client(
        &self,
        cache: Arc<dyn CacheService>,
    ) -> Result<LrcLibClient> {
        let client = LrcLibClient::new(
            self.config.clone(),
            cache,
        ).await
        .map_err(crate::error::LrcGetError::Internal)?;

        Ok(client)
    }

    pub async fn create_downloader(
        &self,
        database: Arc<dyn DatabaseService>,
    ) -> Result<Downloader> {
        let downloader = Downloader::new(
            self.config.clone(),
            database,
        ).await
        .map_err(crate::error::LrcGetError::Internal)?;

        Ok(downloader)
    }

    pub fn create_http_client(&self) -> Result<reqwest::Client> {
        let client = reqwest::Client::builder()
            .user_agent("lrcget-cli/0.1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(crate::error::NetworkError::Http)?;

        Ok(client)
    }

    pub fn config(&self) -> Arc<Config> {
        self.config.clone()
    }
}