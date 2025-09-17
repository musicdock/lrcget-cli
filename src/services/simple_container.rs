use std::sync::Arc;
use crate::config::Config;
use crate::error::Result;

pub struct SimpleServices {
    config: Arc<Config>,
}

impl SimpleServices {
    pub fn new(config: Config) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    pub fn config(&self) -> Arc<Config> {
        self.config.clone()
    }

    pub async fn create_database(&self) -> Result<crate::core::data::database::Database> {
        let db = crate::core::data::database::Database::new(&self.config.database_path).await
            .map_err(crate::error::LrcGetError::Internal)?;
        Ok(db)
    }

    pub async fn create_scanner(&self) -> Result<crate::core::files::scanner::Scanner> {
        Ok(crate::core::files::scanner::Scanner)
    }

    pub fn create_lrclib_client(&self) -> crate::core::services::lrclib::LrclibClient {
        crate::core::services::lrclib::LrclibClient::new(&self.config.lrclib_instance)
    }

    pub fn create_downloader(&self) -> crate::core::services::lrclib::LyricsDownloader {
        crate::core::services::lrclib::LyricsDownloader::new(&self.config.lrclib_instance)
    }
}