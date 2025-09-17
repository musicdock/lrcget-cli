use std::sync::Arc;
use tokio::sync::OnceCell;
use crate::config::Config;
use crate::core::data::database::Database;
use crate::core::services::lrclib::LrclibClient as LrcLibClient;
use crate::core::services::lrclib::LyricsDownloader as Downloader;
use crate::core::files::scanner::Scanner;
use crate::core::infrastructure::cache::{HybridCache, CacheService};
use crate::error::{Result, LrcGetError};
use crate::services::{DatabaseService, ClientFactory};

pub struct ServiceContainer {
    config: Arc<Config>,
    database: OnceCell<Arc<dyn DatabaseService>>,
    lrclib_client: OnceCell<Arc<LrcLibClient>>,
    cache_service: OnceCell<Arc<dyn CacheService>>,
    downloader: OnceCell<Arc<Downloader>>,
    scanner: OnceCell<Arc<Scanner>>,
    client_factory: OnceCell<Arc<ClientFactory>>,
}

impl ServiceContainer {
    pub async fn new(config: Config) -> Result<Self> {
        let config = Arc::new(config);

        let container = Self {
            config,
            database: OnceCell::new(),
            lrclib_client: OnceCell::new(),
            cache_service: OnceCell::new(),
            downloader: OnceCell::new(),
            scanner: OnceCell::new(),
            client_factory: OnceCell::new(),
        };

        Ok(container)
    }

    pub fn config(&self) -> Arc<Config> {
        self.config.clone()
    }

    pub async fn database(&self) -> Result<Arc<dyn DatabaseService>> {
        let database = self.database.get_or_try_init(|| async {
            let db = Database::new(&self.config.database_path).await
                .map_err(LrcGetError::Internal)?;

            let service: Arc<dyn DatabaseService> = Arc::new(DatabaseServiceImpl::new(Arc::new(db)));
            Ok(service)
        }).await?;

        Ok(database.clone())
    }

    pub async fn client_factory(&self) -> Result<Arc<ClientFactory>> {
        let factory = self.client_factory.get_or_try_init(|| async {
            Ok(Arc::new(ClientFactory::new(self.config.clone())))
        }).await?;

        Ok(factory.clone())
    }

    pub async fn lrclib_client(&self) -> Result<Arc<LrcLibClient>> {
        let client = self.lrclib_client.get_or_try_init(|| async {
            let factory = self.client_factory().await?;
            let cache = self.cache_service().await?;

            let client = factory.create_lrclib_client(cache).await?;
            Ok(Arc::new(client))
        }).await?;

        Ok(client.clone())
    }

    pub async fn cache_service(&self) -> Result<Arc<dyn CacheService>> {
        let cache = self.cache_service.get_or_try_init(|| async {
            let cache = HybridCache::new(&self.config).await
                .map_err(LrcGetError::Internal)?;

            let service: Arc<dyn CacheService> = Arc::new(cache);
            Ok(service)
        }).await?;

        Ok(cache.clone())
    }

    pub async fn downloader(&self) -> Result<Arc<Downloader>> {
        let downloader = self.downloader.get_or_try_init(|| async {
            let factory = self.client_factory().await?;
            let database = self.database().await?;

            let downloader = factory.create_downloader(database).await?;
            Ok(Arc::new(downloader))
        }).await?;

        Ok(downloader.clone())
    }

    pub async fn scanner(&self) -> Result<Arc<Scanner>> {
        let scanner = self.scanner.get_or_try_init(|| async {
            let database = self.database().await?;

            let scanner = Scanner::new(database, self.config.clone());
            Ok(Arc::new(scanner))
        }).await?;

        Ok(scanner.clone())
    }
}

use crate::core::data::database::Database as CoreDatabase;

struct DatabaseServiceImpl {
    database: Arc<CoreDatabase>,
}

impl DatabaseServiceImpl {
    fn new(database: Arc<CoreDatabase>) -> Self {
        Self { database }
    }
}

#[async_trait::async_trait]
impl DatabaseService for DatabaseServiceImpl {
    async fn get_tracks(&self) -> Result<Vec<crate::core::files::scanner::Track>> {
        let db = self.database.clone();
        tokio::task::spawn_blocking(move || {
            db.get_tracks().map_err(LrcGetError::Internal)
        }).await
        .map_err(|e| LrcGetError::Internal(e.into()))?
    }

    async fn insert_track(&self, track: &crate::core::files::scanner::Track) -> Result<i64> {
        let db = self.database.clone();
        let track = track.clone();
        tokio::task::spawn_blocking(move || {
            db.insert_track(&track).map_err(LrcGetError::Internal)
        }).await
        .map_err(|e| LrcGetError::Internal(e.into()))?
    }

    async fn update_track(&self, track: &crate::core::files::scanner::Track) -> Result<()> {
        let db = self.database.clone();
        let track = track.clone();
        tokio::task::spawn_blocking(move || {
            db.update_track(&track).map_err(LrcGetError::Internal)
        }).await
        .map_err(|e| LrcGetError::Internal(e.into()))?
    }

    async fn delete_track(&self, id: i64) -> Result<()> {
        let db = self.database.clone();
        tokio::task::spawn_blocking(move || {
            db.delete_track(id).map_err(LrcGetError::Internal)
        }).await
        .map_err(|e| LrcGetError::Internal(e.into()))?
    }

    async fn search_tracks(&self, query: &str) -> Result<Vec<crate::core::files::scanner::Track>> {
        let db = self.database.clone();
        let query = query.to_string();
        tokio::task::spawn_blocking(move || {
            db.search_tracks(&query).map_err(LrcGetError::Internal)
        }).await
        .map_err(|e| LrcGetError::Internal(e.into()))?
    }
}